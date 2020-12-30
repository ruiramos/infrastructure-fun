# Using terraform to provision a GKE cluster

## Setting up Vault and Terraform

1. Start by configuring Vault locally so we have access key based authentication with the Google Provider

Documented in [this README](./local-vault/README.md).


2. Use the Vault roleset we created above as an access token provider for the Google Terraform provider that we'll use to provision the Kubernetes cluster. This is in [main.tf](./main.tf).


3. Next, configure the terraform backend to use Google Cloud Storage (gcs).

Ideally, we want our terraform state to live remotely, so in the future multiple users can update the terraform state and it doesn't depend on a local state. A Google Storage bucket was created for that purpose. The configuration that instructs terraform to use the bucket is on the `terraform { }` block.

Unfortunately I hit a road block here as I couldn't use the same authentication method - access token from Vault - that I used on the Google provider so I had to create a separate service account (with super limited access - can only access that bucket) and use it here. (`credentials` key).
I believe this limitation is [documented here](https://github.com/hashicorp/terraform/issues/13022).

In order to limit the Service Account to access the bucket, the role of `Storage Admin` was limited by a rule that looked like this:
```
resource.name.startsWith("projects/_/buckets/tf-state-cluster-test")
```


## Creating resources on GCP

1. Create the VPC the cluster will use

The VPC configuration lives in `vpc.tf`.
From here on, most steps follow this [HashiCorp tutorial](https://learn.hashicorp.com/tutorials/terraform/gke).


2. Create the GKE cluster

The cluster configuration lives in `gke.tf`.
It uses a separately configured node pool, which seems like the recommended way to do things, however that does mean some weirdness of specifying 1 node count and removing it immediately (`remove_default_node_pool`) in the cluster config.
It also specifies an `ip_allocation_policy`, so the provisioned cluster is vpc native, meaning it uses alias IPs to route traffic to pods, instead of static routes like the older routes-based cluster. More info [here](https://cloud.google.com/kubernetes-engine/docs/concepts/alias-ips).


3. To import the cluster credentials to my local `kubectl`, I used the `gcloud` CLI:
```
gcloud auth login
gcloud config set project project-name
gcloud container clusters get-credentials $(terraform output kubernetes_cluster_name) --region $(terraform output region)
```

Not sure if there was a Vault-y way of doing this.


4. Deployed [Kubernetes Dashboard](https://kubernetes.io/docs/tasks/access-application-cluster/web-ui-dashboard/), created admin account:
```
kubectl apply -f https://raw.githubusercontent.com/kubernetes/dashboard/v2.0.0-beta8/aio/deploy/recommended.yaml
kubectl apply -f kubernetes-admin/kubernetes-dashboard-admin.rbac.yaml
```

This allows us to generate an authorization token with:
```
kubectl -n kube-system describe secret $(kubectl -n kube-system get secret | grep service-controller-token | awk '{print $1}')
```

The Kubernetes Dashboard should be running [here (local link)](http://127.0.0.1:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:/proxy/), proxied (ie while running `kubectl proxy` on a separate terminal window).


## Deploying our first service

I created a `service/` directory that will hold the code for a bunch of microservices that we'll want to dockerize and deploy to the kubernetes cluster. A few goals I have for now with this repo structure are:
 - Automatically build new Docker images for the services when their source code changes (pushed to Google Artifact Registry, more about this below)
 - (After getting [ArgoCD](https://argoproj.github.io/argo-cd/) configured) Automatically deploy new versions of the services to the cluster when the kubernetes deployment files change

The first service we're going to deploy is called [data-storage-service](./services/data-storage-service) and it's written in Rust. (more info on the [service's README](./services/data-storage-service/README.md)).
As stated there, we're using a multistage build so that we keep the final images smaller and we are able to make the most of Docker Layer Caching by caching the compiled dependencies, if they don't change, instead of doing it in every single build.


### Setting up Google Artifact Registry

We'll be using [Google Artifact Registry](https://cloud.google.com/artifact-registry) to host our containerized images of the services, it's an evolution of the old Google Container Registry and it supports both Docker images and a bunch of languages packages as well (npm, etc). 

To set up GAR, I created [a new .tf file](./google_artifact_registry.tf) that contains definitions for the repository and the service account we'll use on our CI/CD pipeline to push built images into. At this point, I needed to add a few extra roles to the terraform/Vault service account, so we can create and manage service account, keys and IAM roles. When applying this `google_artifact_registry.tf` for the first time, a private key for the SA will be created and output on the terminal. I used this as a secret on Github so we can authenticate when using Github Actions.


### Setting up Github Actions

The workflow definition for the first service, `data-storage-service`, can be [found here](./.github/workflows/build-data-storage-service.yml).

We're using Docker's [build and push action](https://github.com/docker/build-push-action) that uses buildx / [Moby BuildKit](https://github.com/moby/buildkit) behind the scenes to build the images. I didn't know about this at all but from what I could gather it brings some improvements on caching, build parellelization and cross-platform builds. Since we're already using multi-stage builds on this first service's Dockerfile, it should make sense to use it. Another difference vs `docker build` is that buildx supports tagging and pushing the image straight away, but [for now](https://github.com/docker/build-push-action/issues/100) you need to use the `docker-compose` backend, which you can do by including [this setup action](https://github.com/docker/setup-buildx-action).

One last thing that took me a while to get right was setting the cache mode to `max` (on the action's `cache-to` parameter). This makes docker cache every layer of every image built - in our case build and release. The default setting only keeps layers of the final image and so we were not caching the projects dependencies, which reduced the build times almost by half.


### Kubernetes Resource Configs and Kustomize

We'll be using [Kustomize](https://kustomize.io/) to be able to easily create several versions of our Kubernete Resource Configurations (Deployments, Services, etc).
Although Kustomize has been added to `kubectl` already, [the version included on the CLI is out of date](https://github.com/kubernetes-sigs/kustomize#kubectl-integration) so I would recommend installing Kustomize seperately and not using the `kubectl apply -k` syntax for now.

We're using the standard [overlay file structure](https://github.com/kubernetes-sigs/kustomize#2-create-variants-using-overlays) so we can have customizations for different environments in the future, eg dev vs production, and all the yamls can be found on the service's kubernetes/ directory (eg for [data-storage-service](./services/data-storage-service/kubernetes/)). 

We created a service and deployment for `data-storage-service` that pulls the image from GAR and runs a bunch of replicas. That made me realise I had asked for a very strict set of oauth scopes initially on the Kubernetes nodes configuration, and I had to extend it so Kubernetes was authorized to pull the images from GAR. More about the [available scopes here](https://cloud.google.com/sdk/gcloud/reference/container/clusters/create#--scopes).

To apply the dev configuration we run, from the service directory:
```
kustomize build kubernetes/overlays/dev | kubectl apply -f -
```

This just created our first deployment and service on the new GKE cluster! 🎉
We can test that it's working by port-forwarding, for instance, the service:
```
k port-forward service/dev-data-storage-service 8888:80
```

And then use the service:
```
➜  infra git:(main) ✗ curl -d 'some secret' localhost:8888
{"password":"wwdhN)lugP!h0BvF","url":"https://dont-know-yet/K6nR7sd6gHTY8wz5FcGSEDauFW6nSUXa"}% 
```

This service will look more useful later on! :)


## Setting up ArgoCD for Continuous Deployment

[ArgoCD](https://argoproj.github.io/argo-cd/) is a continuous delivery tool for Kubernetes that syncs your Kubernetes Resource Config files (or Kustomize templates) with the state of the cluster. This is called GitOps!

We can install is by running:
```
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
```

We'll create two manifests for two seperate Applications - the dev and production versions of the `data-storage-service`. They live [on the argocd/ directory](./argocd). Applying this to the cluster will create the 2 apps on ArgoCD, which will then poll our Github repository for changes on the apps manifests/kustomize files.

### Using a GitHub webhook

TODO - this would improve deployment times as ArgoCD would be notifiied of the change on the  manifest.

## Setting up an Ingress controller with a static IP address



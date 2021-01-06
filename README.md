# Zero to Kubernetes (Hero!)

This is a log of my ventures into "modern" infrastructure management. This is work in progress! My main objective was to get hands on experience with Kubernetes and microservices deployment in a somewhat realistic setting, ie one that could be used for a production project, without the shortcuts normally taken in getting started tutorials. Some things I wanted to achieve were:
 - provision a Kubernetes cluster running on [GKE](https://cloud.google.com/kubernetes-engine) using [Terraform](https://www.terraform.io/)
 - proper service account and permission management (done with [Hashicorp Vault](https://www.vaultproject.io/))
 - build and deploy a few Rust microservices, with CI (used Github Actions) and CD the [GitOps](https://www.gitops.tech/) way (used [ArgoCD](https://argoproj.github.io/argo-cd/)). Manage different versions of the app (ie different environments) using [Kustomize](https://kustomize.io/). 
 - explore some ingress/API Gateway solutions (used [Ambassador](getambassador.io/))
 - explore service meshes and what they offer, try out [dapr](https://dapr.io/) (not done yet) 
 - serverless on kubernetes with knative (not done yet)
 - metrics and alerts (not done yet)

Because of the way this has been done, iteratively and always using this repository, people following will unfortunately only access the final form of the files worked on (unless of course you're digging through git history). Hopefully this is not too big of a limitation in understading whats going on. I've tried to note whenever I had to go back and change previous work significantely.


## Setting up Vault and Terraform

The main objective for this section is to get Vault running locally so it can authenticate Terraform calls via short-lived access tokens that belong to a service account created and managed by Vault (with the right permissions Terraform needs).

We started by configuring Vault locally so we have access key based authentication with the Google Provider, this is documented in [this other README](./local-vault) on the `local-vault` directory. We'll then use the Vault roleset we created above as an access token provider for the Google Terraform provider that will provision the Kubernetes cluster. This is done in [main.tf](./main.tf).

Finally, we configured the terraform backend to use Google Cloud Storage (gcs). We want our terraform state to live remotely, so in the future multiple users can update the terraform state and it doesn't depend on a local state. A Google Storage bucket was created for that purpose. The configuration that instructs terraform to use the bucket is on the `terraform { }` block.

Unfortunately I hit a road block here as I couldn't use the same authentication method - access token from Vault - that I used on the Google provider so I had to create a separate service account (with super limited access - can only access that bucket) and use it here. (`credentials` key).
I believe this limitation is [documented here](https://github.com/hashicorp/terraform/issues/13022).

In order to limit the Service Account to access the bucket, the role of `Storage Admin` was limited by a rule that looked like this:
```
resource.name.startsWith("projects/_/buckets/tf-state-cluster-test")
```


## Creating resources on GCP

1. Create the VPC the cluster will use.

The VPC configuration lives in `vpc.tf`.
From here on, most steps follow this [HashiCorp tutorial](https://learn.hashicorp.com/tutorials/terraform/gke).

2. Create the GKE cluster.

The cluster configuration lives in [gke.tf](./gke.tf).
It uses a separately configured node pool, which seems like the recommended way to do things, however that does mean some weirdness of specifying 1 node count and removing it immediately (`remove_default_node_pool`) in the cluster config.
It also specifies an `ip_allocation_policy`, so the provisioned cluster is vpc native, meaning it uses alias IPs to route traffic to pods, instead of static routes like the older routes-based cluster. [More info about this here.](https://cloud.google.com/kubernetes-engine/docs/concepts/alias-ips)


3. Import the cluster credentials to my local `kubectl`. I used the `gcloud` CLI at this point:
```
gcloud auth login
gcloud config set project project-name
gcloud container clusters get-credentials $(terraform output kubernetes_cluster_name) --region $(terraform output region)
```

Not sure if there was a better, Vault-y way of doing this.

4. Deployed [Kubernetes Dashboard](https://kubernetes.io/docs/tasks/access-application-cluster/web-ui-dashboard/), created admin account:

As a first, hello-world deployment, we deployed the Kubernetes dashboard by running:

```
kubectl apply -f kubernetes/kubernetes-dashboard-admin.rbac.yaml
kubectl apply -f https://raw.githubusercontent.com/kubernetes/dashboard/v2.0.0-beta8/aio/deploy/recommended.yaml
```

This creates a `cluster-admin` service account, and we can then generate an authorization token with:

```
kubectl -n kube-system describe secret $(kubectl -n kube-system get secret | grep service-controller-token | awk '{print $1}')
```

The Kubernetes Dashboard should be running [here (local link)](http://127.0.0.1:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:/proxy/), proxied (ie while running `kubectl proxy` on a separate terminal window).

This dashboard looks cool but Google offers a lot of the same functionality on their UI so I ended up not using it.


## Deploying the first service

I created a `service/` directory that will hold the code for a bunch of microservices that we'll want to dockerize and deploy to the kubernetes cluster. A few goals I have for now with this repo structure are:
 - Automatically build new Docker images for the services when their source code changes (pushed to Google Artifact Registry, more about this below)
 - Automatically deploy new versions of the services to the cluster when the kubernetes deployment files change

The first service we're going to deploy is called [data-storage-service](./services/data-storage-service) and it's written in Rust. (more info on the [service's README](./services/data-storage-service/README.md)).
As stated there, we're using a multistage build so that we keep the final images smaller and we are able to make the most of Docker Layer Caching by caching the compiled dependencies, if they don't change, instead of doing it in every single build.


### Setting up Google Artifact Registry

We'll be using [Google Artifact Registry](https://cloud.google.com/artifact-registry) to host our containerized images of the services, it's an evolution of the old Google Container Registry and it supports both Docker images and a bunch of languages packages as well (npm, etc). 

To set up GAR, I created [a new .tf file](./google_artifact_registry.tf) that contains definitions for the repository and the service account we'll use on our CI pipeline to push built images into. At this point, I needed to add a few extra roles to the terraform/Vault service account, so we can create and manage service account, keys and IAM roles. When applying this `google_artifact_registry.tf` for the first time, a private key for the SA will be created and output on the terminal. I used this as a secret on Github so we can authenticate when using Github Actions.


### Setting up Github Actions

The workflow definition for the first service, `data-storage-service`, can be [found here](./.github/workflows/build-data-storage-service.yml).

We're using Docker's [build and push action](https://github.com/docker/build-push-action) that uses buildx / [Moby BuildKit](https://github.com/moby/buildkit) behind the scenes to build the images. I didn't know about this at all but from what I could gather it brings some improvements on caching, build parellelization and cross-platform builds. Since we're already using multi-stage builds on this first service's Dockerfile, it should make sense to use it. Another difference vs `docker build` is that buildx supports tagging and pushing the image straight away, but [for now](https://github.com/docker/build-push-action/issues/100) you need to use the `docker-compose` backend, which you can do by including [this setup action](https://github.com/docker/setup-buildx-action).

One last thing that took me a while to get right was setting the cache mode to `max` (on the action's `cache-to` parameter). This makes docker cache every layer of every image built - in our case build and release. The default setting only keeps layers of the final image and so we were not caching the projects dependencies, which reduced the build times almost by half.


### Kubernetes Resource Configs and Kustomize

We'll be using [Kustomize](https://kustomize.io/) to be able to easily create several versions of our Kubernete Resource Configurations (Deployments, Services, etc).
Although Kustomize has been added to `kubectl` already, [the version included on the CLI is out of date](https://github.com/kubernetes-sigs/kustomize#kubectl-integration) so I would recommend installing Kustomize seperately and not using the `kubectl apply -k` syntax for now.

We're using the standard [overlay file structure](https://github.com/kubernetes-sigs/kustomize#2-create-variants-using-overlays) so we can have customizations for different environments in the future, eg dev vs production, and all the yamls can be found on the service's kubernetes/ directory (eg for [data-storage-service](./services/data-storage-service/kubernetes/)). 

We created a service and deployment for `data-storage-service` that pulls the image from GAR and runs a bunch of replicas. That made me realise I had asked for a very strict set of oauth scopes initially on the [Kubernetes nodes configuration](./gke.tf#L52), and I had to extend it so Kubernetes was authorized to pull the images from GAR. More about the [available scopes here](https://cloud.google.com/sdk/gcloud/reference/container/clusters/create#--scopes).

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


## Setting up ArgoCD for Continuous Deployment

[ArgoCD](https://argoproj.github.io/argo-cd/) is a continuous delivery tool for Kubernetes that syncs your Kubernetes Resource Config files (or Kustomize templates) living on a git repository with the state of the cluster. This is called GitOps!

We can install is by running:
```
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
```

We'll create two manifests for two seperate Applications - the dev and production versions of the `data-storage-service`. They live [on the argocd/ directory](./argocd). Applying this to the cluster will create the 2 apps on ArgoCD, which will then poll our Github repository for changes on the apps manifests/kustomize files.

Normally, as a best practice, you would create a separate repository to hold the manifests than the one that holds application code and point ArgoCD there. For simplicity, I'm only using this one.

At this point, ArgoCD is monitoring this git repository every 3 minutes and diffing the manifests with the state of the cluster. We can do better by...

### Using a GitHub webhook

This would improve deployment times as ArgoCD would be notified of changes on the manifest files.
However at this point we don't have the cluster exposed to the world yet, so we'll do this later. To have configuration changes instantly applied, we can for now port-forward the ArgoCD service and use the CLI to manually trigger a Argo sync, ie:
```
kubectl port-forward svc/argocd-server -n argocd 8080:443
argocd app sunc data-storage-app
```

We will revisit this later!


## Exposing our cluster to the world

In this section, we'll allow outside traffic to the cluster by adding an Ingress controller with a static IP address, with a Google Managed SSL certificate so all traffic is HTTPS. We're going to be using [Ambassador](https://github.com/datawire/ambassador), a Kubernetes native API gateway based on the [Envoy proxy](https://www.envoyproxy.io/), because it seems it can do a lot for us besides load balancing, seems nicer to use and configure, and for my own educational purposes.

### Setting up an Ingress controller with a static IP address

At this point, we created a new section on the [Terraform GKE cluster definition file](./gke.tf) to provision a Google static IP address called `global-cluster-ip`. This will be used as an annotation on the Ingress controller - the L7 HTTP(S) load balancer we'll create next.

### Installing Ambassador on the cluster

We followed [this guide](https://www.getambassador.io/docs/latest/topics/running/ambassador-with-gke/) to get Ambassador running on the GKE cluster. Start by applying the configuration files (we downloaded the base ones and extended with some additional config that will be useful in a second):

```
k apply -f kubernetes/ambassador/ambassador-crds.yaml
k apply -f kubernetes/ambassador/
```

Now we have Ambassador running and exposed with a `NodePort` service. This will be the service used by the Ingress controller we'll create next to forward traffic to, after terminating TLS.

### Create a Google Managed Certificate and the Ingress controller

```
k apply -f kubernetes/certificate.yaml
k apply -f kubernetes/basic-ingress.yaml
```

This is all quite simple stuff, as you can see the [ingress](./kubernetes/basic-ingress.yaml) uses the `ambassador` service, using the static IP address and certificates we've created just now. There's a few limiting things with these managed certificates like the fact that you can't use wildcards, so for now we have to specify each domain/sub-domain we'll be using in the certificate Kubernetes resource definition.

There are a final annoying thing to do here related with the `ambassador`'s health checks [documented here](https://www.getambassador.io/docs/latest/topics/running/ambassador-with-gke/#5-configure-ambassador-to-do-http---https-redirection) - we need to point the Ingress backend health check to use the `ambassador-admin` `NodePort`, as the `ambassador` will return a 301 to everything it doesn't think it's HTTPS. This was done on the Google Cloud UI directly. :(

### Route traffic to the deployed service

At this stage, I created the [Host](./services/data-storage-service/kubernetes/overlays/dev/host.yaml) and [Mapping](./services/data-storage-service/kubernetes/overlays/dev/mapping.yaml) configurations in the service Kubernetes dev overlay that tell Ambassador to route traffic to the service based on Host and make HTTP requests redirect to HTTPS.

The service is now available on `https://keep.gke.ruiramos.com`!


## Adding Redis

The first version of the `data-storage` service was stateful, using an in-memory HashMap to record keys and values - which is less than ideal in this high-availability world as we have multiple versions of the application running behind the single service, so we would end up with keys in different places. To fix this, we added a Redis backend deployed as a separate pod, and introduced a new environment variable that tells the service where to find Redis. For local development, we can use Docker and expose a port, for our development and production deployments we're using Kubernetes DNS to find the Redis service: it will be accessible in `service-name.namespace.cluster.local`, in this case for eg dev, `dev-redis-service.default.svc.cluster.local`.

Around this time we also created config maps for the pods environment variables using `.properties` files, with kustomize helping us applying the right configuration for production and development environments.

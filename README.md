# Using terraform to provision a GKE cluster

## Setting up Vault and Terraform

1. Start by configuring Vault locally so we have access key based authentication with the Google Provider

Documented in [this README](./local-vault/README.md).

2. Next, configure the terraform backend to use Google Cloud Storage (gcs).

Ideally, we want our terraform state to live remotely, so in the future multiple users can update the terraform state and it doesn't depend on a local state. A Google Storage bucket was created for that purpose. The configuration that instructs terraform to use the bucket is on the `terraform { }` block.

Unfortunately I hit a road block here as I couldn't use the same authentication method - access token from Vault - that I used on the Google provider so I had to create a separate service account (with super limited access - can only access that bucket) and use it here. (`credentials` key).
I believe this limitation is [documented here](https://github.com/hashicorp/terraform/issues/13022).

In order to limit the Service Account to access the bucket, the role of Storage Admin was limited by a rule that looked like this:
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


4. Deployed Kubernetes Dashboard, created admin account:
```
kubectl apply -f https://raw.githubusercontent.com/kubernetes/dashboard/v2.0.0-beta8/aio/deploy/recommended.yaml
kubectl apply -f kubernetes-admin/kubernetes-dashboard-admin.rbac.yaml
```

This allows us to generate an authorization token with:
```
kubectl -n kube-system describe secret $(kubectl -n kube-system get secret | grep service-controller-token | awk '{print $1}')
```

The Kubernetes Dashboard should be running [here](http://127.0.0.1:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:/proxy/), proxied (ie while running `kubectl proxy` on a separate terminal window).


## Deploying rust-echo-service

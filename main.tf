# VAULT_ADDR read in from Environment variables on the workspace
provider "vault" {}

# What roleset are we looking for - read in from Terraform variable on the workspace
data "vault_generic_secret" "gcp_auth" {
  path = "gcp/token/${var.roleset}"
}

provider "google" {
  access_token = data.vault_generic_secret.gcp_auth.data.token
  project     = var.project_id
  region      = var.region
  zone        = var.zone
}

provider "google-beta" {
  access_token = data.vault_generic_secret.gcp_auth.data.token
  project     = var.project_id
  region      = var.region
  zone        = var.zone
}

terraform {
  backend "gcs" {
    credentials = "tfstate-sa.json"
    bucket  = "tf-state-cluster-test"
    prefix  = "terraform/state"
  }
}


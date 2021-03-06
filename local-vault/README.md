# Configuring local Vault

## Prerequisites

Downloading and installing HashiCorp Vault. See how [here](https://www.vaultproject.io/docs/install#install-vault).

## Process

1. Start the dev server
```
vault server -dev
```

Export the server location as an env var
```
export VAULT_ADDR='http://127.0.0.1:8200'

```

2. Generate a Service Account key Vault can use. It needs [these permissions](https://www.vaultproject.io/docs/secrets/gcp#required-permissions).

3. Configure GCP credentials on Vault
```
vault secrets enable gcp
vault write gcp/config credentials=@vault-sa.json
```

4. Create a bindings file. The credentials generated by Vault will have these permissions. Check [bindings.hcl](./bindings.hcl).

5. Create a roleset with those permissions that generate shortlived OAuth2 access tokens:
```
vault write gcp/roleset/project-factory-roleset \
  project=helical-theater-274414 \
  secret-type="access_token" \
  token_scopes="https://www.googleapis.com/auth/cloud-platform" \
  bindings=@bindings.hcl
```

The above will create a new Service Account on your GCP Project with the following roles:
 - Compute Admin
 - Kubernetes Engine Admin
 - Service Account User

6. You can now access shortlived (1h) OAuth2 tokens:
```
vault read gcp/token/project-factory-roleset
```

# data-storage-service

This is an encrypted data storage service. It allows a user to POST some data to it and get a URL and a password back (as json). Making a POST request to the provided URL with the password as body will give access to the decrypted data.

It listens on the port provided by the `PORT` env var, or 8088.

The service will be packaged in a Docker container and pushed to Google Registry using a Github Actions workflow.
It will then run on the GKE cluster created. The kubernetes definition files live in the `kubernetes/` directory.

TODO: Add Redis

## Dockerfile

We're using a multistage build with a build-context and a release-context.
On `build-context`, we use some clever tricks to preserve as many of the layers as possible, compiling dependencies even before copying the source files. The `release-context` is a thin Debian image that runs the binary, using [tini](https://github.com/krallin/tini). This was all copied from [one of the MHRA services](https://github.com/MHRA/products/blob/master/hello-world/Dockerfile) so thanks team! :D


## Actions

There's a Github Actions workflow that builds and pushes the new image to the Google artifact registry (GAR) if it detects any changes to the `src/` files in this projects directory (that might have also been slightly adapted from MHRA).


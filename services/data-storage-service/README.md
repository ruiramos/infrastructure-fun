# rust-echo-service

This is a very simple echo service written in Rust, using [Tide](https://github.com/http-rs/tide).
It replies back whatever you post to its `/echo` endpoint.

It listens on the port provided by the `PORT` env var, or 8088.

The service will be packaged in a Docker container and pushed to Google Registry using a Github Actions workflow.
It will then run on the GKE cluster created. The kubernetes definition files live in the `kubernetes/` directory.


## Dockerfile

We're using a multistage build with a build-context and a release-context.
On `build-context`, we use some clever tricks to preserve as many of the layers as possible, compiling dependencies even before copying the source files. The `release-context` is a thin Debian image that runs the binary, using [tini](https://github.com/krallin/tini). This was all copied from [one of the MHRA services](https://github.com/MHRA/products/blob/master/hello-world/Dockerfile) so thanks team.


## Actions

There's a GH Actions workflow that builds and pushes the new image to the Google container registry if it detects any changes to the `src/` files in this projects directory (that might have also been slightly adapted from MHRA).


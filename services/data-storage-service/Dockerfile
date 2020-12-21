# Setup rust build environment
FROM rust:1.48 AS build-context

RUN rustup component add rustfmt clippy

WORKDIR /usr/src/data-storage-service

COPY Cargo.toml .
COPY Cargo.lock .

# Layer hack: Build an empty program to compile dependencies and place on their own layer.
# This cuts down build time. It was borrowed from here:
# https://github.com/deislabs/krustlet/blob/master/Dockerfile#L7
RUN mkdir -p ./src/ && \
    echo 'fn main() {}' >./src/main.rs && \
    echo '' >./src/lib.rs

RUN cargo fetch

RUN cargo build --release && \
    rm -rf ./target/release/.fingerprint/data_storage_service-*

# Setup debian release environment
FROM debian:buster-slim AS release-context

RUN apt-get update && apt-get install -y \
    tini \
    ;

RUN useradd svc

# Build real binaries now, as late as possible
FROM build-context AS build

COPY ./src ./src

RUN cargo clippy --release -- -D warnings && \
    cargo test --release && \
    cargo build --release

# Create the release
FROM release-context AS release

COPY --from=build /usr/src/data-storage-service/target/release/data_storage_service /

RUN chown -R svc /data_storage_service

USER svc

ENTRYPOINT ["/usr/bin/tini", "--"]

CMD ["/data_storage_service"]

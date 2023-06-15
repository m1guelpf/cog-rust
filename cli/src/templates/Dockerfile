# syntax = docker/dockerfile:1.2
FROM lukemathwalker/cargo-chef:latest-rust-slim-bullseye AS chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --bin {:bin_name} --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && apt-get install -y g++ pkg-config libssl-dev
COPY --from=planner /src/recipe.json recipe.json
RUN RUSTFLAGS='-C target-feature=+crt-static' cargo chef cook --release --bin {:bin_name} --target x86_64-unknown-linux-gnu
COPY . .
RUN RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --bin {:bin_name} --target x86_64-unknown-linux-gnu

FROM debian:buster-slim as runtime
WORKDIR /src
COPY --from=builder /src/target/release/{:bin_name} /usr/local/bin/cog
ENTRYPOINT ["/usr/local/bin/cog"]
EXPOSE 5000

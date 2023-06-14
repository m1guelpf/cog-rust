# syntax = docker/dockerfile:1.2
FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --bin {:bin_name} --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release
COPY . .
RUN cargo build --release --bin {:bin_name}

FROM debian:bullseye-slim as runtime
WORKDIR /src
RUN apt-get update && apt-get install -y libssl1.1 && apt clean && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/{:bin_name} /usr/local/bin/cog
ENTRYPOINT ["/usr/local/bin/cog"]
EXPOSE 5000

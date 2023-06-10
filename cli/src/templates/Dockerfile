# syntax = docker/dockerfile:1.2
FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin {:bin_name}

FROM alpine AS runtime
WORKDIR /src
EXPOSE 5000
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/{:bin_name} /usr/local/bin/cog
ENTRYPOINT ["/usr/local/bin/cog"]

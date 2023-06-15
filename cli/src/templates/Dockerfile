# syntax = docker/dockerfile:1.2
FROM clux/muslrust:stable AS chef
WORKDIR /src
RUN cargo install cargo-chef

FROM chef AS planner
COPY . .
RUN cargo chef prepare --bin {:bin_name} --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --target x86_64-unknown-linux-musl --release
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin {:bin_name}

FROM alpine AS runtime
WORKDIR /src
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/{:bin_name} /usr/local/bin/cog
ENTRYPOINT ["/usr/local/bin/cog"]
EXPOSE 5000

# syntax = docker/dockerfile:1.2
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /src
#SLOT setup

FROM chef AS planner
COPY . .
RUN cargo chef prepare --bin {:bin_name} --recipe-path recipe.json

FROM chef AS builder
RUN mkdir /src/lib
RUN apt-get update && apt-get install -y cmake clang
RUN curl -L --proto '=https' --tlsv1.2 -sSf "https://github.com/cargo-bins/cargo-quickinstall/releases/download/cargo-deb-1.44.0/cargo-deb-1.44.0-x86_64-unknown-linux-gnu.tar.gz" | tar -xzvvf - -C /usr/local/cargo/bin
#SLOT before_build
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --bin {:bin_name}
COPY . .
RUN cargo deb --output /src/target/{:bin_name}.deb
#SLOT after_build

FROM {:base_image} as runtime
WORKDIR /src
RUN apt-get update
COPY --from=builder /src/lib/* /lib/x86_64-linux-gnu/
COPY --from=builder /src/target/{:bin_name}.deb /src/{:bin_name}.deb
#SLOT before_runtime
RUN apt-get install -y /src/{:bin_name}.deb && rm /src/{:bin_name}.deb
EXPOSE 5000
CMD ["/usr/bin/{:bin_name}"]
#SLOT after_runtime

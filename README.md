# Cog[-rust]: Containers for machine learning

[Cog](https://github.com/replicate/cog) is an open-source tool that lets you package machine learning models in a standard, production-ready container.

Cog bundles Python models into a Docker image with a simple interface for loading and running models. This project aims to match that interface for Rust models, so they can be used interchangeably (on [Replicate](https://replicate.com) or Dyson).

## WIP

To see the proposed DX, check the [hello-world example](examples/hello-world/src/main.rs).

- [x] Basic web server
- [x] Rust Cog interface
- [x] Make everything work
- [x] Request validation
- [x] Dockerfile
- [x] `cargo cog login`
- [x] `cargo cog debug`
- [x] `cargo cog build`
- [x] `cargo cog push`
- [ ] `cargo cog predict`
- [x] deploys to Replicate work
- [x] Run locally
- [ ] Run on Replicate

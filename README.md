# Cog[-rust]: Containers for machine learning

Cog is an open-source tool that lets you package Rust ML models in a standard, production-ready container.

It's output should be interchangeable with [Replicate's own Cog](https://github.com/replicate/cog) (for Python models).

## Highlights

- 📦 **Docker containers without the pain.** Writing your own `Dockerfile` can be a bewildering process. With Cog, you define your environment [inside your Cargo.toml](#how-it-works) and it generates a Docker image with all the best practices: Nvidia base images, efficient caching of dependencies, minimal image sizes, sensible environment variable defaults, and so on.

- 🤬️ **No more CUDA hell.** Cog knows which CUDA/cuDNN/tch/tensorflow combos are compatible and will set it all up correctly for you.

- ✅ **Define the inputs and outputs for your model in Rust.** Then, Cog generates an OpenAPI schema and validates the inputs and outputs with JSONSchema.

- 🎁 **Automatic HTTP prediction server**: Your model's types are used to dynamically generate a RESTful HTTP API using [axum](https://github.com/tokio-rs/axum).

- ☁️ **Cloud storage.** Files can be read and written directly to Amazon S3 and Google Cloud Storage. (Coming soon.)

- 🚀 **Ready for production.** Deploy your model anywhere that Docker images run. Your own infrastructure, or [Replicate](https://replicate.com).

## How it works

Easily define your environment inside your `Cargo.toml`. Cog infers the rest:

```toml
[package]
name = "ml-model"

[package.metadata.cog]
gpu = true # optional, defaults to false
image = "docker-image-name" # optional, defaults to `cog-[package.name]`
```

Define how predictions are run on your model on your `main.rs`:

```rust
use anyhow::Result;
use cog_rust::Cog;
use schemars::JsonSchema;
use std::collections::HashMap;
use tch::{
  nn::{ModuleT, VarStore},
  vision::{imagenet, resnet::resnet50},
  Device,
};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ModelRequest {
  /// Image to classify
  image: cog_rust::Path,
}

struct ResnetModel {
  model: Box<dyn ModuleT + Send>,
}

impl Cog for ResnetModel {
  type Request = ModelRequest;
  type Response = HashMap<String, f64>;

  async fn setup() -> Result<Self> {
    let mut vs = VarStore::new(Device::Cpu);
    vs.load("weights/model.safetensors")?;
    let model = Box::new(resnet50(&vs.root(), imagenet::CLASS_COUNT));

    Ok(Self { model })
  }

  fn predict(&self, input: Self::Request) -> Result<Self::Response> {
    let image = imagenet::load_image_and_resize224(&input.image)?;
    let output = self
      .model
      .forward_t(&image.unsqueeze(0), false)
      .softmax(-1, tch::Kind::Float);

    Ok(imagenet::top(&output, 5)
      .into_iter()
      .map(|(prob, class)| (class, 100.0 * prob))
      .collect())
  }
}

cog_rust::start!(ResnetModel);
```

Now, you can run predictions on this model:

```console
$ cargo cog predict -i @input.jpg
--> Building Docker image...
--> Running Prediction...
--> Output written to output.jpg
```

Or, build a Docker image for deployment:

```console
$ cargo cog build -t my-colorization-model
--> Building Docker image...
--> Built my-colorization-model:latest

$ docker run -d -p 5000:5000 --gpus all my-colorization-model

$ curl http://localhost:5000/predictions -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"image": "https://.../input.jpg"}}'
```

## Why am I building this?

The Replicate team has done an amazing job building the simplest way to go from Python notebook to Docker image to API endpoint.

However, using Python as the base layer comes with its on share of challenges, like enormus image sizes or extra latency on model requests.

As the non-Python ML ecosystem slowly flourishes (see [whisper.cpp](https://github.com/ggerganov/whisper.cpp) and [llama.cpp](https://github.com/ggerganov/whisper.cpp) for example), cog-rust will provide that extra performance exposed on the same interfaces users and tools are already used to.

## Prerequisites

- **macOS, Linux or Windows**. Cog works anywhere Rust works.
- **Docker**. Cog uses Docker to create a container for your model. You'll need to [install Docker](https://docs.docker.com/get-docker/) before you can run Cog.

## Install

<a id="upgrade"></a>

You can install Cog with Cargo:

```console
cargo install cargo-cog
```

## Usage

```
$ cargo cog --help
A cargo subcommand to build, run and publish machine learning containers

Usage: cargo cog [OPTIONS] [COMMAND]

Commands:
  login    Log in to Replicate's Docker registry
  build    Build the model in the current directory into a Docker image
  push     Build and push model in current directory to a Docker registry
  predict  Run a prediction
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

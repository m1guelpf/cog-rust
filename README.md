# Cog[-rust]: Containers for machine learning

[Cog](https://github.com/replicate/cog) is an open-source tool that lets you package machine learning models in a standard, production-ready container.

Cog bundles Python models into a Docker image with a simple interface for loading and running models. This project aims to match that interface for Rust models, so they can be used interchangeably (on [Replicate](https://replicate.com) or Dyson).

```rust
use cog_rust::Cog;
use anyhow::Result;
use async_trait::async_trait;

struct ExampleModel {
    prefix: String,
}

#[derive(serde::Deserialize)]
struct ModelRequest {
    /// Text to prefix with 'hello '
    text: String,
}

#[async_trait]
impl Cog for ExampleModel {
    type Request = ModelRequest;
    type Response = String;

    async fn setup() -> Result<Self> {
        Ok(Self {
            prefix: "hello".to_string(),
        })
    }

    fn predict(&self, input: Self::Request) -> Result<Self::Response> {
        Ok(format!("{} {}", self.prefix, input.text))
    }
}

#[tokio::main]
async fn main() {
    cog_rust::start::<ExampleModel>().await.unwrap();
}
```

## WIP

This is a work in progress. It's not ready for use yet. Check back soon!

- [ ] Basic web server
- [ ] Rust Cog interface
- [ ] Make everything work
- [ ] Dockerfile
- [ ] CLI?

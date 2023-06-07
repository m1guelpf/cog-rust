#![feature(return_position_impl_trait_in_trait)]

use anyhow::Result;
use async_trait::async_trait;
use cog_rust::Cog;

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

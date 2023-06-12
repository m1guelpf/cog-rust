use anyhow::Result;
use async_trait::async_trait;
use cog_rust::Cog;
use schemars::JsonSchema;

#[derive(serde::Deserialize, JsonSchema)]
struct ModelRequest {
	/// Text to prefix with 'hello '
	text: String,
}

struct ExampleModel {
	prefix: String,
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

	async fn predict(&self, input: Self::Request) -> Result<Self::Response> {
		Ok(format!("{} {}", self.prefix, input.text))
	}
}

cog_rust::start!(ExampleModel);

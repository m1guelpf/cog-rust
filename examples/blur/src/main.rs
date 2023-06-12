use anyhow::Result;
use async_trait::async_trait;
use cog_rust::{Cog, Path};
use schemars::JsonSchema;

#[derive(serde::Deserialize, JsonSchema)]
struct ModelRequest {
	/// Input image
	image: Path,
	/// Blur radius (default: 5)
	blur: Option<f32>,
}

struct BlurModel {}

#[async_trait]
impl Cog for BlurModel {
	type Request = ModelRequest;
	type Response = Path;

	async fn setup() -> Result<Self> {
		Ok(Self {})
	}

	async fn predict(&self, input: Self::Request) -> Result<Self::Response> {
		let image = image::open(&input.image)?;
		image.blur(input.blur.unwrap_or(5.0)).save(&input.image)?;

		Ok(input.image)
	}
}

cog_rust::start!(BlurModel);

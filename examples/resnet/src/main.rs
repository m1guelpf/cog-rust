use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use cog_rust::Cog;
use schemars::JsonSchema;
use tch::{
	nn::{ModuleT, VarStore},
	vision::{imagenet, resnet::resnet50},
	Device,
};

#[derive(serde::Deserialize, JsonSchema)]
struct ModelRequest {
	/// Image to classify
	image: cog_rust::Path,
}

struct BlurModel {
	model: Box<dyn ModuleT + Send>,
}

#[async_trait]
impl Cog for BlurModel {
	type Request = ModelRequest;
	type Response = HashMap<String, f64>;

	async fn setup() -> Result<Self> {
		let mut vs = VarStore::new(Device::cuda_if_available());
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

cog_rust::start!(BlurModel);

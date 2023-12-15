use anyhow::Result;
use cog_rust::Cog;
use schemars::JsonSchema;
use std::collections::HashMap;
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

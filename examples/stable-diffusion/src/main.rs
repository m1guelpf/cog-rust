use anyhow::Result;
use cog_rust::{Cog, Path};
use diffusers::{
	models::{unet_2d::UNet2DConditionModel, vae::AutoEncoderKL},
	pipelines::stable_diffusion::{self, StableDiffusionConfig},
	transformers::clip::{self, Tokenizer},
	utils::DeviceSetup,
};
use schemars::JsonSchema;
use std::path::PathBuf;
use tch::{nn::Module, Device, Kind, Tensor};

#[derive(serde::Deserialize, JsonSchema)]
struct ModelRequest {
	/// Input prompt
	prompt: String,

	/// Random seed. Leave blank to randomize the seed
	seed: Option<u32>,

	/// Number of images to output.
	#[validate(range(min = 1, max = 4))]
	num_outputs: Option<u8>,

	/// Number of denoising steps.
	#[validate(range(min = 1, max = 500))]
	num_inference_steps: Option<u8>,

	/// Scale for classifier-free guidance.
	#[validate(range(min = 1, max = 20))]
	guidance_scale: Option<f64>,
}

struct StableDiffusion {
	vae: AutoEncoderKL,
	tokenizer: Tokenizer,
	devices: DeviceSetup,
	unet: UNet2DConditionModel,
	sd_config: StableDiffusionConfig,
	text_model: clip::ClipTextTransformer,
}

impl Cog for StableDiffusion {
	type Request = ModelRequest;
	type Response = Vec<Path>;

	async fn setup() -> Result<Self> {
		tch::maybe_init_cuda();
		let sd_config = stable_diffusion::StableDiffusionConfig::v2_1(None, None, None);
		let device_setup = diffusers::utils::DeviceSetup::new(vec![]);
		let tokenizer =
			clip::Tokenizer::create("weights/bpe_simple_vocab_16e6.txt", &sd_config.clip)?;
		let text_model = sd_config
			.build_clip_transformer("weights/clip_v2.1.safetensors", device_setup.get("clip"))?;
		let vae = sd_config.build_vae("weights/vae_v2.1.safetensors", device_setup.get("vae"))?;
		let unet =
			sd_config.build_unet("weights/unet_v2.1.safetensors", device_setup.get("unet"), 4)?;

		Ok(Self {
			vae,
			unet,
			tokenizer,
			sd_config,
			text_model,
			devices: device_setup,
		})
	}

	fn predict(&self, input: Self::Request) -> Result<Self::Response> {
		let _no_grad_guard = tch::no_grad_guard();
		let scheduler = self
			.sd_config
			.build_scheduler(input.num_inference_steps.unwrap_or(50).into());
		let text_embeddings = self.tokenize(input.prompt)?;

		let mut outputs = Vec::new();
		for idx in 0..input.num_outputs.unwrap_or(1) {
			if let Some(seed) = input.seed {
				tch::manual_seed((seed + idx as u32).into());
			} else {
				tch::manual_seed(-1);
			}

			let mut latents = Tensor::randn(
				[1, 4, self.sd_config.height / 8, self.sd_config.width / 8],
				(Kind::Float, self.devices.get("unet")),
			);

			latents *= scheduler.init_noise_sigma();

			for &timestep in scheduler.timesteps().iter() {
				let latent_model_input = Tensor::cat(&[&latents, &latents], 0);

				let latent_model_input = scheduler.scale_model_input(latent_model_input, timestep);
				let noise_pred =
					self.unet
						.forward(&latent_model_input, timestep as f64, &text_embeddings);
				let noise_pred = noise_pred.chunk(2, 0);
				let (noise_pred_uncond, noise_pred_text) = (&noise_pred[0], &noise_pred[1]);
				let noise_pred = noise_pred_uncond
					+ (noise_pred_text - noise_pred_uncond) * input.guidance_scale.unwrap_or(7.5);
				latents = scheduler.step(&noise_pred, timestep, &latents);
			}

			let latents = latents.to(self.devices.get("vae"));
			let image = self.vae.decode(&(&latents / 0.18215));
			let image = (image / 2 + 0.5).clamp(0., 1.).to_device(Device::Cpu);
			let image = (image * 255.).to_kind(Kind::Uint8);

			let final_image = PathBuf::from(format!("output-{idx}.png"));
			tch::vision::image::save(&image, &final_image)?;
			outputs.push(final_image.into());
		}

		Ok(outputs)
	}
}

impl StableDiffusion {
	fn tokenize(&self, prompt: String) -> Result<Tensor> {
		let tokens = self.tokenizer.encode(&prompt)?;
		let tokens: Vec<i64> = tokens.into_iter().map(|x| x as i64).collect();
		let tokens = Tensor::from_slice(&tokens)
			.view((1, -1))
			.to(self.devices.get("clip"));

		let uncond_tokens = self.tokenizer.encode("")?;
		let uncond_tokens: Vec<i64> = uncond_tokens.into_iter().map(|x| x as i64).collect();
		let uncond_tokens = Tensor::from_slice(&uncond_tokens)
			.view((1, -1))
			.to(self.devices.get("clip"));

		let text_embeddings = self.text_model.forward(&tokens);
		let uncond_embeddings = self.text_model.forward(&uncond_tokens);

		Ok(Tensor::cat(&[uncond_embeddings, text_embeddings], 0).to(self.devices.get("unet")))
	}
}

cog_rust::start!(StableDiffusion);

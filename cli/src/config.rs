use cargo_metadata::Package;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
	#[serde(default)]
	pub gpu: bool,
	pub image: Option<String>,
}

impl Config {
	pub fn from_package(package: &Package) -> Self {
		let mut config = package
			.metadata
			.get("cog")
			.and_then(|config| Self::deserialize(config).ok())
			.unwrap_or_default();

		if config.image.is_none() {
			config.image = Some(Self::generate_image_name(&package.name));
		}

		config
	}

	pub fn image_name(&self, image: Option<String>) -> String {
		image.or_else(|| self.image.clone()).unwrap()
	}

	fn generate_image_name(name: &str) -> String {
		let mut image_name = name
			.to_lowercase()
			.replace(|c: char| !c.is_alphanumeric(), "-");

		if !image_name.starts_with("cog-") {
			image_name = format!("cog-{image_name}");
		}

		let mut image_name = image_name
			.chars()
			.take(30 - "cog-".len())
			.collect::<String>();

		while let Some(last_char) = image_name.chars().last() {
			if last_char.is_alphanumeric() {
				break;
			}

			image_name.pop();
		}

		image_name
	}

	#[allow(clippy::unused_self)]
	pub fn as_cog_config(&self) -> String {
		serde_json::to_string(&json!({
			"predict": "main.rs:CogModel",
			"build": {
				"gpu": self.gpu,
				"python_version" : "N/A"
			},
		}))
		.unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn generate_image_name() {
		assert_eq!(
			"cog-hello-world",
			Config::generate_image_name("hello-world"),
		);

		assert_eq!(
			"cog-hello-world",
			Config::generate_image_name("cog-hello-world"),
		);

		assert_eq!(
			"cog-a-very-very-long-packa",
			Config::generate_image_name("a-very-very-long-package-name"),
		);

		assert_eq!(
			"cog-with-a-very-very-long",
			Config::generate_image_name("cog-with-a-very-very-long-package-name"),
		);

		assert_eq!(
			"cog-with-a-very-very-long",
			Config::generate_image_name("cog-with-a-very-very-long-package-name"),
		);

		assert_eq!(
			"cog-with-invalid-name",
			Config::generate_image_name("cog-with-invalid-name-!@#$%^&*()"),
		);
	}
}

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
	pub image: Option<String>,
}

impl Config {
	pub fn from_package(package: cargo_toml::Package) -> Self {
		package
			.metadata
			.and_then(|meta| meta.as_table()?.get("cog").cloned())
			.and_then(|config| Config::deserialize(config).ok())
			.unwrap_or_default()
	}

	pub fn image_name(&self, image: Option<String>, cwd: &Path) -> String {
		if let Some(image) = image.or(self.image.clone()) {
			return image;
		}

		let project_name = cwd
			.file_name()
			.unwrap()
			.to_str()
			.unwrap()
			.to_lowercase()
			.replace(" ", "-")
			.replace(|c: char| !c.is_alphanumeric(), "")
			.chars()
			.take(30 - "cog-".len()) // Docker image names can only be 30 characters long.
			.collect::<String>();

		format!("cog-{project_name}")
	}
}

use cargo_metadata::{MetadataCommand, Package};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
	pub image: Option<String>,
}

impl Config {
	pub fn from_path(path: &Path) -> Self {
		let cargo_toml = MetadataCommand::new()
			.manifest_path(path.join("Cargo.toml"))
			.exec()
			.expect(
				"Failed to read Cargo.toml. Make sure you are in the root of your Cog project.",
			);

		let package = cargo_toml
			.root_package()
			.expect("Couldn't find the package section in Cargo.toml.");

		Self::from_package(package)
	}
	pub fn from_package(package: &Package) -> Self {
		package
			.metadata
			.get("cog")
			.and_then(|config| Self::deserialize(config).ok())
			.unwrap_or_default()
	}

	pub fn image_name(&self, image: Option<String>, cwd: &Path) -> String {
		if let Some(image) = image.or_else(|| self.image.clone()) {
			return image;
		}

		let project_name = cwd
			.file_name()
			.unwrap()
			.to_str()
			.unwrap()
			.to_lowercase()
			.replace(' ', "-")
			.replace(|c: char| !c.is_alphanumeric(), "")
			.chars()
			.take(30 - "cog-".len()) // Docker image names can only be 30 characters long.
			.collect::<String>();

		format!("cog-{project_name}")
	}

	#[allow(clippy::unused_self)]
	pub fn as_cog_config(&self) -> String {
		serde_json::to_string(&json!({
			"predict": "main.rs:CogModel",
			"build": {
				"python_version" : "N/A"
			},
		}))
		.unwrap()
	}
}

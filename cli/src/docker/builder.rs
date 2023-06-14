use cargo_toml::Manifest;
use map_macro::hash_map;
use std::{
	collections::HashMap,
	fs::{self, File},
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};

use crate::{config::Config, helpers::is_m1_mac};

pub struct Builder {
	cwd: PathBuf,
	pub config: Config,
	binary_name: String,
	cog_version: String,
}

impl Builder {
	pub fn new(cwd: PathBuf) -> Self {
		let cargo_toml = Manifest::from_path(cwd.join("Cargo.toml")).expect(
			"Failed to read Cargo.toml. Make sure you are in the root of your Cog project.",
		);

		let package = cargo_toml
			.package
			.expect("Couldn't find the package section in Cargo.toml.");

		let cog_version = cargo_toml
			.dependencies
			.get("cog-rust")
			.expect("Couldn't find cog-rust in your Cargo.toml")
			.req()
			.to_string();

		assert!(cog_version != "*", "Couldn't resolve cog version. Make sure you're loading the package through the registry, not from git or a local path.");

		Self {
			cwd,
			cog_version,
			binary_name: package.name.clone(),
			config: Config::from_package(package),
		}
	}

	pub fn generate_dockerfile(&self) -> String {
		include_str!("../templates/Dockerfile").replace("{:bin_name}", &self.binary_name)
	}

	pub fn build(&self, tag: Option<String>) -> String {
		let dockerfile = self.generate_dockerfile();

		File::create(self.cwd.join(".dockerignore")).and_then(|mut file| write!(file, "target")).expect(
            "Failed to create .dockerignore file. Make sure you are in the root of your Cog project."
        );

		let image_name = self.config.image_name(tag, &self.cwd);
		Self::build_image(&dockerfile, &image_name, None, true);

		fs::remove_file(self.cwd.join(".dockerignore")).expect("Failed to clean up .dockerignore");

		println!("Adding labels to image...");
		let output = Command::new("docker")
			.arg("run")
			.arg("--rm")
			.arg("-e")
			.arg("RUST_LOG=cog_rust=error")
			.arg(&image_name)
			.arg("--dump-schema-and-exit")
			.output()
			.expect("Failed to extract schema from image.");

		assert!(
			output.status.success(),
			"Failed to extract schema from image: {}",
			String::from_utf8(output.stdout).expect(
				"Failed to parse output from command `docker run --rm -e RUST_LOG=cog_rust=error {image_name} --dump-schema-and-exit`."
			)
		);

		let schema = String::from_utf8(output.stdout).expect("Failed to parse schema.");

		Self::build_image(
			&format!("FROM {image_name}"),
			&image_name,
			Some(hash_map! {
				"run.cog.has_init" => "true",
				"run.cog.openapi_schema" => schema.trim(),
				"org.cogmodel.openapi_schema" => schema.trim(),
				"run.cog.config" => &self.config.as_cog_config(),
				"org.cogmodel.config" => &self.config.as_cog_config(),
				"run.cog.version" => &format!("{}-rust", self.cog_version),
				"org.cogmodel.cog_version" => &format!("{}-rust", self.cog_version),
				"org.cogmodel.deprecated" =>  "The org.cogmodel labels are deprecated. Use run.cog.",
			}),
			false,
		);

		image_name
	}

	pub fn push(&self, image: &Option<String>) {
		let status = Command::new("docker")
			.arg("push")
			.arg(
				image
					.as_ref()
					.or(self.config.image.as_ref())
					.expect("Image name not specified"),
			)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.status()
			.expect("Failed to push image.");

		assert!(status.success(), "Failed to push image.");
	}

	fn build_image(
		dockerfile: &str,
		image_name: &str,
		labels: Option<HashMap<&str, &str>>,
		show_logs: bool,
	) {
		let mut process = Command::new("docker")
			.args(if is_m1_mac() {
				vec!["buildx", "build", "--platform", "linux/amd64", "--load"]
			} else {
				vec!["build"]
			})
			.args([
				"--file",
				"-",
				"--tag",
				image_name,
				"--build-arg",
				"BUILDKIT_INLINE_CACHE=1",
			])
			.args(
				labels
					.unwrap_or_default()
					.iter()
					.flat_map(|(key, value)| ["--label".to_string(), format!("{key}={value}")]),
			)
			.arg(".")
			.env("DOCKER_BUILDKIT", "1")
			.stdin(Stdio::piped())
			.stderr(if show_logs {
				Stdio::inherit()
			} else {
				Stdio::null()
			})
			.stdout(if show_logs {
				Stdio::inherit()
			} else {
				Stdio::null()
			})
			.spawn()
			.expect("Failed to spawn docker build process.");

		process
			.stdin
			.as_mut()
			.expect("Failed to open stdin.")
			.write_all(dockerfile.as_bytes())
			.expect("Failed to write to stdin.");

		let status = process
			.wait()
			.expect("Failed to wait for docker build process.");

		assert!(status.success(), "Failed to build docker image.");
	}
}

impl Drop for Builder {
	fn drop(&mut self) {
		let _ = fs::remove_file(self.cwd.join(".dockerignore"));
	}
}

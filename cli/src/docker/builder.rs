use cargo_toml::Manifest;
use map_macro::hash_map;
use std::{
	collections::HashMap,
	fs,
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};

use crate::{config::Config, helpers::is_m1_mac};

pub struct Builder {
	cwd: PathBuf,
	pub config: Config,
	binary_name: String,
}

impl Builder {
	pub fn new(cwd: PathBuf) -> Self {
		let cargo_toml = Manifest::from_path(cwd.join("Cargo.toml")).expect(
			"Failed to read Cargo.toml. Make sure you are in the root of your Cog project.",
		);

		cwd.join("src/main.rs").metadata().expect(
        "Couldn't find the project's entry point. Make sure you are in the root of your Cog project.",
        );

		fs::File::create(cwd.join(".dockerignore")).and_then(|mut file| write!(file, "target")).expect(
            "Failed to create .dockerignore file. Make sure you are in the root of your Cog project.",
        );

		let package = cargo_toml
			.package
			.expect("Couldn't find the package section in Cargo.toml.");

		Self {
			cwd,
			binary_name: package.name.clone(),
			config: Config::from_package(package),
		}
	}

	pub async fn build(&self, tag: Option<String>) -> String {
		let dockerfile =
			include_str!("../templates/Dockerfile").replace("{:bin_name}", &self.binary_name);

		let image_name = self.config.image_name(tag, &self.cwd);
		Self::build_image(dockerfile, &image_name, None, true);

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

		let schema = String::from_utf8(output.stdout).expect("Failed to parse schema.");

		Self::build_image(
			format!("FROM {image_name}"),
			&image_name,
			Some(hash_map! {
				"run.cog.version" => "dev",
				"run.cog.has_init" => "true",
				"org.cogmodel.cog_version" => "dev",
				"run.cog.openapi_schema" => schema.trim(),
				"org.cogmodel.openapi_schema" => schema.trim(),
				"org.cogmodel.deprecated" =>  "The org.cogmodel labels are deprecated. Use run.cog.",
				"run.cog.config" => r#"{"build":{"python_version":"3.8"},"predict":"predict.py:Predictor"}"#,
				"org.cogmodel.config" => r#"{"build":{"python_version":"3.8"},"predict":"predict.py:Predictor"}"#,
			}),
			false,
		);

		image_name
	}

	pub fn push(&self, image_name: &str) {
		let status = Command::new("docker")
			.arg("push")
			.arg(image_name)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.status()
			.expect("Failed to push image.");

		if !status.success() {
			panic!("Failed to push image.");
		}
	}

	fn build_image(
		dockerfile: String,
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

		if !status.success() {
			panic!("Failed to build docker image.");
		}
	}
}

impl Drop for Builder {
	fn drop(&mut self) {
		fs::remove_file(self.cwd.join(".dockerignore"))
			.expect("Failed to remove .dockerignore file. You may need to remove it manually.");
	}
}

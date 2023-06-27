use cargo_metadata::{MetadataCommand, Package};
use map_macro::hash_map;
use std::{
	collections::HashMap,
	fs::{self, File},
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};

use super::dockerfile::{Dockerfile, DockerfileExt};
use crate::{config::Config, docker::Docker};

pub struct Builder {
	cwd: PathBuf,
	package: Package,
	pub config: Config,
	_cog_version: String,
	deps: Vec<Package>,
}

impl Builder {
	pub fn new(cwd: PathBuf) -> Self {
		let cargo_metadata = MetadataCommand::new()
			.manifest_path(cwd.join("Cargo.toml"))
			.exec()
			.expect(
				"Failed to read Cargo.toml. Make sure you are in the root of your Cog project.",
			);

		let package = cargo_metadata
			.root_package()
			.expect("Couldn't find the package section in Cargo.toml.");

		assert!(
			!package.authors.is_empty(),
			"You must specify at least one author in Cargo.toml"
		);

		let cog_version = package
			.dependencies
			.iter()
			.find(|dep| dep.name == "cog-rust")
			.expect("Couldn't find cog-rust in your Cargo.toml")
			.req
			.to_string();

		assert!(cog_version != "*", "Couldn't resolve cog version. Make sure you're loading the package through the registry, not from git or a local path.");

		Self {
			cwd,
			package: package.clone(),
			_cog_version: cog_version,
			deps: cargo_metadata.packages.clone(),
			config: Config::from_package(package),
		}
	}

	pub fn generate_dockerfile(&self) -> String {
		let torchlib_cpu = || {
			self.deps.iter().find(|dep| dep.name == "torch-sys")?;

			Some(Dockerfile::new().run_multiple(&[
                Command::new("curl")
                    .args(["-sSL", "https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-2.0.1%2Bcpu.zip", "-o libtorch.zip"]),
                Command::new("unzip").arg("libtorch.zip"),
                Command::new("rm").arg("libtorch.zip"),
                Command::new("cp").arg("libtorch/lib/*").arg("/src/lib")
            ]).env("LIBTORCH", "/src/libtorch"))
		};

		let weights_dir = || {
			if self.cwd.join("weights").exists() {
				Some(Dockerfile::new().copy("weights/", "/src/weights/"))
			} else {
				None
			}
		};

		let replicate_hack = || {
			Some(
				Dockerfile::new()
					.run_multiple(&[
						Command::new("echo")
							.arg(r##""#!/bin/bash\nexit 0""##)
							.arg(">")
							.arg("/usr/local/bin/pip"),
						Command::new("chmod").arg("+x").arg("/usr/local/bin/pip"),
						Command::new("echo")
							.arg(format!(
								// Replicate runs `python -m cog.server.http --other-args*` and we only care about the other args
								"\"#!/bin/bash\\nshift 2; /usr/bin/{} \"\\$@\"\"",
								self.package.name
							))
							.arg(">")
							.arg("/usr/local/bin/python"),
						Command::new("chmod").arg("+x").arg("/usr/local/bin/python"),
					])
                    // Replicate also doesn't provide a way to set the log level, so we have to do it manually
					.env("RUST_LOG", r#""cog_rust=trace""#),
			)
		};

		include_str!("../templates/Dockerfile")
			.to_string()
			.for_bin(&self.package.name)
			.handler("before_build", torchlib_cpu)
			.handler("before_runtime", weights_dir)
			.handler("after_runtime", replicate_hack)
			.build()
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
			.arg(format!("/usr/bin/{}", self.package.name))
			.arg("--dump-schema-and-exit")
			.output()
			.expect("Failed to extract schema from image.");

		assert!(
			output.status.success(),
			"Failed to extract schema from image: {}",
			String::from_utf8(output.stderr).expect(
				"Failed to parse output from command `docker run --rm -e RUST_LOG=cog_rust=error {image_name} --dump-schema-and-exit`."
			)
		);

		let schema = String::from_utf8(output.stdout).expect("Failed to parse schema.");

		Self::build_image(
			&format!("FROM {image_name}"),
			&image_name,
			Some(hash_map! {
				"run.cog.has_init" => "true",
				// Seems like Replicate will only allow `cog` versions, so we hardcode it here
				"run.cog.version" => "0.7.2",
				"org.cogmodel.cog_version" => "0.7.2",
				"run.cog.openapi_schema" => schema.trim(),
				"org.cogmodel.openapi_schema" => schema.trim(),
				"run.cog.config" => &self.config.as_cog_config(),
				"rs.cog.authors" => &self.package.authors.join(", "),
				"org.cogmodel.config" => &self.config.as_cog_config(),
				"org.cogmodel.deprecated" =>  "The org.cogmodel labels are deprecated. Use run.cog.",
			}),
			false,
		);

		image_name
	}

	pub fn push(&self, image: &Option<String>) {
		Docker::push(
			image
				.as_ref()
				.or(self.config.image.as_ref())
				.expect("Image name not specified"),
		)
		.expect("Failed to push image.");
	}

	fn build_image(
		dockerfile: &str,
		image_name: &str,
		labels: Option<HashMap<&str, &str>>,
		show_logs: bool,
	) {
		let mut process = Command::new("docker")
			.arg("build")
			.args([
				"--platform",
				"linux/amd64",
				"--tag",
				image_name,
				"--build-arg",
				"BUILDKIT_INLINE_CACHE=1",
				"--file",
				"-",
			])
			.args(
				labels
					.unwrap_or_default()
					.iter()
					.flat_map(|(key, value)| ["--label".to_string(), format!("{key}={value}")]),
			)
			.arg(".")
			.env("DOCKER_BUILDKIT", "1")
			.env("DOCKER_DEFAULT_PLATFORM", "linux/amd64")
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

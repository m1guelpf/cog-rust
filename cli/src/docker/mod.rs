mod auth;
mod builder;
mod dockerfile;
pub mod predictor;

use std::{
	collections::HashMap,
	io,
	path::PathBuf,
	process::{self, Command, Stdio},
};

pub use auth::store_credentials;
pub use builder::Builder;
pub use predictor::Predictor;

/// Errors that can occur when interacting with the docker CLI.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Could not connect to Docker. Is the docker daemon running?")]
	NotRunning,

	#[error("The provided image could not be found.")]
	NotFound,

	#[error("Provided flags without a command.")]
	CmdMissing,

	#[error("{0}")]
	Command(String),

	#[error("Failed to parse output from command: {0}")]
	Parse(String),

	#[error("Failed to run command: {0}")]
	Spawn(#[from] std::io::Error),

	#[error("Failed to parse output from command: {0}")]
	ToString(#[from] std::string::FromUtf8Error),

	#[error("Failed to parse output from command: {0}")]
	Deserialize(#[from] serde_json::Error),
}

#[derive(Debug, Default)]
pub struct RunOptions {
	detach: bool,
	image: String,
	env: Vec<String>,
	interactive: bool,
	flags: Vec<String>,
	cmd: Option<String>,
	ports: HashMap<u16, u16>,
	volumes: HashMap<PathBuf, String>,
}

/// A wrapper around the docker CLI.
pub struct Docker {}

impl Docker {
	/// Check if the docker daemon is running.
	///
	/// # Errors
	///
	/// Returns an error if the docker daemon is not running.
	pub fn check_connection() -> Result<(), Error> {
		let status = Command::new("docker")
			.arg("info")
			.stdout(Stdio::null())
			.status()?;

		if !status.success() {
			return Err(Error::NotRunning);
		}

		Ok(())
	}

	/// Inspect the given image.
	/// Returns the image metadata as a JSON struct.
	///
	/// # Errors
	///
	/// Returns an error if the image could not be found.
	pub fn inspect_image(image: &str) -> Result<serde_json::Value, Error> {
		let output = Command::new("docker")
			.arg("image")
			.arg("inspect")
			.arg(image)
			.output()?;

		if !output.status.success()
			&& String::from_utf8(output.stderr.clone())?.contains("No such image")
		{
			return Err(Error::NotFound);
		}

		if !output.status.success() {
			return Err(Error::Command(format!(
				"Failed to inspect image: {}",
				String::from_utf8(output.stderr)?.trim()
			)));
		}

		Ok(serde_json::from_slice(&output.stdout)?)
	}

	/// Inspect the given container.
	/// Returns the container metadata as a JSON struct.
	///
	/// # Errors
	///
	/// Returns an error if the container could not be found.
	pub fn inspect_container(container_id: &str) -> Result<serde_json::Value, Error> {
		let output = Command::new("docker")
			.arg("container")
			.arg("inspect")
			.arg(container_id)
			.output()?;

		if !output.status.success()
			&& String::from_utf8(output.stderr.clone())?.contains("No such container")
		{
			return Err(Error::NotFound);
		}

		if !output.status.success() {
			return Err(Error::Command(format!(
				"Failed to inspect container: {}",
				String::from_utf8(output.stderr)?.trim()
			)));
		}

		Ok(serde_json::from_slice(&output.stdout)?)
	}

	/// Pull the given image from the Docker registry.
	/// Returns the image digest.
	///
	/// # Errors
	///
	/// Returns an error if the command fails or if the output cannot be parsed.
	pub fn pull(image: &str) -> Result<String, Error> {
		let output = Command::new("docker")
			.arg("pull")
			.arg(image)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.output()?;

		if !output.status.success() {
			return Err(Error::Command(format!(
				"Failed to pull image: {}",
				String::from_utf8(output.stderr)?.trim()
			)));
		}

		Ok(String::from_utf8(output.stdout)?
			.split("sha256:")
			.last()
			.ok_or_else(|| Error::Parse("docker pull".to_string()))?
			.split(' ')
			.next()
			.ok_or_else(|| Error::Parse("docker pull".to_string()))?
			.to_string())
	}

	/// Push the given image to the Docker registry.
	/// Returns the image digest.
	///
	/// # Errors
	///
	/// Returns an error if the command fails or if the output cannot be parsed.
	pub fn push(image: &str) -> Result<String, Error> {
		let output = Command::new("docker")
			.arg("push")
			.arg(image)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.output()?;

		if !output.status.success() {
			return Err(Error::Command(format!(
				"Failed to push image: {}",
				String::from_utf8(output.stderr)?.trim()
			)));
		}

		Ok(String::from_utf8(output.stdout)?
			.split("sha256:")
			.last()
			.ok_or_else(|| Error::Parse("docker push".to_string()))?
			.split(' ')
			.next()
			.ok_or_else(|| Error::Parse("docker push".to_string()))?
			.to_string())
	}

	/// Run a container with the given image, volumes, and ports.
	/// Returns the container ID.
	///
	/// # Errors
	///
	/// Returns an error if the command fails or if the output cannot be parsed.
	pub fn run(opts: RunOptions) -> Result<String, Error> {
		let mut cmd = Command::new("docker");

		cmd.arg("run").arg("--rm").stderr(Stdio::piped());

		if opts.detach {
			cmd.arg("--detach");
		}

		if opts.interactive {
			cmd.arg("--interactive");
		}

		for var in opts.env {
			cmd.args(["--env", &var]);
		}

		for (source, destination) in opts.ports {
			cmd.args(["--publish", &format!("{source}:{destination}")]);
		}

		for (source, destination) in opts.volumes {
			cmd.args([
				"--mount",
				&format!(
					"type=bind,source={},destination={destination}",
					source.display()
				),
			]);
		}

		cmd.arg(opts.image);

		if let Some(bin) = opts.cmd {
			cmd.arg(bin);

			for flag in opts.flags {
				cmd.arg(flag);
			}
		} else if !opts.flags.is_empty() {
			return Err(Error::CmdMissing);
		}

		let output = cmd.stderr(Stdio::piped()).output()?;

		if !output.status.success() {
			return Err(Error::Command(format!(
				"Failed to start container: {}",
				String::from_utf8(output.stderr)?.trim()
			)));
		}

		Ok(String::from_utf8(output.stdout)
			.map_err(|_| Error::Parse("docker run".to_string()))?
			.trim()
			.to_string())
	}

	pub fn stop(container_id: &str) -> Result<(), Error> {
		let output = Command::new("docker")
			.arg("container")
			.arg("stop")
			.args(["--time", "3"])
			.arg(container_id)
			.stderr(Stdio::inherit())
			.output()?;

		if !output.status.success() {
			return Err(Error::Command(format!(
				"Failed to stop container: {}",
				String::from_utf8(output.stderr)?.trim()
			)));
		}

		Ok(())
	}

	pub fn find_port(container_id: &str, container_port: u16) -> Result<u16, Error> {
		let output = Command::new("docker")
			.arg("port")
			.arg(container_id)
			.arg(container_port.to_string())
			.output()?;

		if !output.status.success() {
			return Err(Error::Command(format!(
				"Failed to get port: {}",
				String::from_utf8(output.stderr)?.trim()
			)));
		}

		let stdout = String::from_utf8(output.stdout)?;

		stdout
			.trim()
			.split(':')
			.last()
			.ok_or_else(|| Error::Parse("docker port".to_string()))?
			.parse()
			.map_err(|_| Error::Parse("docker port".to_string()))
	}

	pub fn tail_logs(container_id: &str) -> io::Result<process::Child> {
		Command::new("docker")
			.arg("container")
			.arg("logs")
			.arg("-f")
			.arg(container_id)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.spawn()
	}
}

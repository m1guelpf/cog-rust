use std::path::PathBuf;

use anyhow::Result;

use crate::docker::Docker;

#[derive(Debug, Clone)]
pub struct Context {
	pub cwd: PathBuf,
}

impl Context {
	/// Create a new context
	///
	/// # Errors
	///
	/// This function will return an error if the Docker daemon is not running or if the current working directory cannot be determined.
	pub fn new() -> Result<Self> {
		Docker::check_connection()?;

		Ok(Self {
			cwd: std::env::current_dir()?,
		})
	}

	#[must_use]
	pub fn into_builder(self) -> crate::docker::Builder {
		crate::docker::Builder::new(self.cwd)
	}
}

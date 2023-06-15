#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::Result;
use clap::Parser;
use docker::Docker;
use std::path::PathBuf;

mod commands;
mod config;
mod docker;
mod helpers;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: commands::Command,
}

#[derive(Debug, Clone)]
pub struct Context {
	pub cwd: PathBuf,
}

impl Context {
	/// Create a new context
	///
	/// # Errors
	///
	/// This function will return an error if the current working directory cannot be determined.
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

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	let ctx = Context::new().unwrap();

	commands::exec(ctx, cli.command).await;
}

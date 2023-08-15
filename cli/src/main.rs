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
#[clap(bin_name = "cargo")]
struct Cli {
	#[clap(subcommand)]
	command: CargoInvocation,
}

#[derive(Parser)]
pub enum CargoInvocation {
	// All `cargo` subcommands receive their name (e.g. `cog` as the first command).
	// See https://github.com/rust-lang/rustfmt/pull/3569
	/// A cargo subcommand to build, run and publish machine learning containers
	Cog {
		#[command(subcommand, long_about)]
		command: commands::Command,
	},
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
	let CargoInvocation::Cog { command } = cli.command;

	commands::exec(ctx, command).await;
}

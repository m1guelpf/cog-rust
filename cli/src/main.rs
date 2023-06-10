use std::path::PathBuf;

use anyhow::Result;
use bollard::Docker;
use clap::Parser;
use commands::Command;

mod commands;
mod config;
mod docker;
mod helpers;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Debug)]
pub struct Context {
	pub cwd: PathBuf,
	pub docker: Docker,
}

impl Context {
	pub async fn new() -> Result<Self> {
		let docker = Docker::connect_with_local_defaults()?
			.negotiate_version()
			.await
			.expect("Couldn't connect to Docker. Is the Docker daemon running?");

		Ok(Self {
			docker,
			cwd: std::env::current_dir()?,
		})
	}
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	let ctx = Context::new().await.unwrap();

	commands::exec(ctx, cli.command).await;
}

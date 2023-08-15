#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use clap::Parser;
use context::Context;
use CargoSubcommand::Cog;

mod commands;
mod config;
mod context;
mod docker;
mod helpers;

/// Cog's CLI interface
///
/// This binary should be invoked by Cargo with the new `cog` subcommand. If
/// you're reading this, consider manually adding `cog` as the first argument.
#[derive(Debug, Parser)]
struct Cargo {
	#[clap(subcommand)]
	command: CargoSubcommand,
}

#[derive(Debug, Parser)]
pub enum CargoSubcommand {
	/// A cargo subcommand to build, run and publish machine learning containers
	Cog(Cli),
}

#[derive(Parser, Debug)]
#[command(about, author, display_name = "cargo-cog")]
#[command(override_usage = "cargo cog [OPTIONS] [COMMAND]")]
pub struct Cli {
	#[command(subcommand)]
	pub command: commands::Command,
}

#[tokio::main]
async fn main() {
	let cargo = Cargo::parse();
	let Cog(cli) = cargo.command;

	commands::exec(cli.command, Context::new().unwrap()).await;
}

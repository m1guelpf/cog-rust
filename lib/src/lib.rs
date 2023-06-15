#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{
	prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

pub use cog_core::{Cog, CogResponse};
pub use spec::Path;

mod errors;
mod helpers;
mod prediction;
mod routes;
mod runner;
mod server;
mod shutdown;
mod spec;
mod webhooks;

#[derive(Debug, clap::Parser)]
pub(crate) struct Cli {
	/// Dump the schema and exit
	#[clap(long)]
	dump_schema_and_exit: bool,

	/// Ignore SIGTERM and wait for a request to /shutdown (or a SIGINT) before exiting
	#[clap(long)]
	await_explicit_shutdown: bool,

	/// An endpoint for Cog to PUT output files to
	#[clap(long)]
	upload_url: Option<url::Url>,
}

/// Start the server with the given cog.
///
/// # Errors
///
/// This function will return an error if the PORT environment variable is set but cannot be parsed, or if the server fails to start.
pub async fn start<T: Cog + 'static>() -> Result<()> {
	let args = Cli::parse();

	if !args.dump_schema_and_exit {
		tracing_subscriber::registry()
			.with(tracing_subscriber::fmt::layer().with_filter(
				EnvFilter::try_from_default_env().unwrap_or_else(|_| "cog_rust=info".into()),
			))
			.init();
	}

	server::start::<T>(args).await
}

#[macro_export]
macro_rules! start {
	($struct_name:ident) => {
		#[tokio::main]
		async fn main() {
			cog_rust::start::<$struct_name>().await.unwrap();
		}
	};
}

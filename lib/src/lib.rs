#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::Result;
use tracing_subscriber::{
	prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

pub use spec::{Cog, CogResponse};

mod errors;
mod helpers;
mod prediction;
mod routes;
mod runner;
mod server;
mod shutdown;
mod spec;
mod webhooks;

/// Start the server with the given cog.
///
/// # Errors
///
/// This function will return an error if the PORT environment variable is set but cannot be parsed, or if the server fails to start.
pub async fn start<T: Cog + 'static>() -> Result<()> {
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer().with_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| "cog_rust=info".into()),
		))
		.init();

	server::start::<T>().await
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

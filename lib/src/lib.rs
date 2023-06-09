#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use crate::{prediction::Prediction, shutdown::Shutdown};
use anyhow::Result;
use axum::Server;
use std::{env, net::SocketAddr, num::ParseIntError};

pub use spec::{Cog, CogResponse};

mod errors;
mod helpers;
mod prediction;
mod routes;
mod runner;
mod shutdown;
mod spec;

/// Start the server with the given cog.
///
/// # Errors
///
/// This function will return an error if the PORT environment variable is set but cannot be parsed, or if the server fails to start.
pub async fn start<T: Cog + 'static>() -> Result<()> {
	let shutdown = Shutdown::new()?;
	let prediction = Prediction::setup::<T>(shutdown.clone());

	let addr = SocketAddr::from((
		[0, 0, 0, 0],
		env::var("PORT").map_or(Ok::<u16, ParseIntError>(5000), |p| p.parse())?,
	));
	println!("Listening on {addr}");

	let app = routes::handler::<T>()
		.layer(prediction.extension())
		.layer(shutdown.extension());

	Server::bind(&addr)
		.serve(app.into_make_service())
		.with_graceful_shutdown(shutdown.handle())
		.await?;

	Ok(())
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

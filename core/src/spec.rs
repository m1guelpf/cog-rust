use anyhow::Result;
use core::fmt::Debug;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::future::Future;

use crate::http::Request;

/// A Cog model
pub trait Cog: Sized + Send {
	type Request: DeserializeOwned + JsonSchema + Send;
	type Response: CogResponse + Debug + JsonSchema;

	/// Setup the model
	///
	/// # Errors
	///
	/// Returns an error if setup fails.
	fn setup() -> impl Future<Output = Result<Self>> + Send;

	/// Run a prediction on the model
	///
	/// # Errors
	///
	/// Returns an error if the prediction fails.
	fn predict(&self, input: Self::Request) -> Result<Self::Response>;
}

/// A response from a Cog model
pub trait CogResponse: Send {
	/// Convert the response into a JSON value
	fn into_response(self, request: Request) -> impl Future<Output = Result<Value>> + Send;
}

impl<T: Serialize + Send + 'static> CogResponse for T {
	async fn into_response(self, _: Request) -> Result<Value> {
		// We use spawn_blocking here to allow blocking code in serde Serialize impls (used in `Path`, for example).
		Ok(tokio::task::spawn_blocking(move || serde_json::to_value(self)).await??)
	}
}

use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::http::Request;

/// A Cog model
#[async_trait]
pub trait Cog: Sized + Send {
	type Request: DeserializeOwned + JsonSchema + Send;
	type Response: CogResponse + Debug + JsonSchema;

	/// Setup the model
	///
	/// # Errors
	///
	/// Returns an error if setup fails.
	async fn setup() -> Result<Self>;

	/// Run a prediction on the model
	fn predict(&self, input: Self::Request) -> Result<Self::Response>;
}

/// A response from a Cog model
#[async_trait]
pub trait CogResponse: Send {
	/// Convert the response into a JSON value
	async fn into_response(self, request: Request) -> Result<Value>;
}

#[async_trait]
impl<T: Serialize + Send + 'static> CogResponse for T {
	async fn into_response(self, _: Request) -> Result<Value> {
		// We use spawn_blocking here to allow blocking code in serde Serialize impls (used in `Path`, for example).
		Ok(tokio::task::spawn_blocking(move || serde_json::to_value(self)).await??)
	}
}

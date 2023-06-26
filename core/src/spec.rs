use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::http::Request;

#[async_trait]
pub trait Cog: Sized + Send {
	type Request: DeserializeOwned + JsonSchema + Send;
	type Response: CogResponse + Debug + JsonSchema;

	/// Setup the cog
	///
	/// # Errors
	///
	/// Returns an error if setup fails.
	async fn setup() -> Result<Self>;

	/// Run a prediction
	fn predict(&self, input: Self::Request) -> Result<Self::Response>;
}

/// A response from a cog
#[async_trait]
pub trait CogResponse: Send {
	async fn into_response(self, request: Request) -> Result<Value>;
}

#[async_trait]
impl<T: Serialize + Send> CogResponse for T {
	async fn into_response(self, _: Request) -> Result<Value> {
		Ok(serde_json::to_value(self)?)
	}
}

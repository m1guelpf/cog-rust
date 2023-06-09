use anyhow::Result;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

#[async_trait]
pub trait Cog: Sized + Send {
	type Request: DeserializeOwned + JsonSchema;
	type Response: CogResponse + JsonSchema;

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
pub trait CogResponse: Send {
	fn into_response(self) -> Value;
}

impl<T: Serialize + Send> CogResponse for T {
	fn into_response(self) -> Value {
		serde_json::to_value(self).unwrap()
	}
}

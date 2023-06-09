use std::sync::atomic::Ordering;

use aide::axum::{
	routing::{get, post},
	ApiRouter,
};
use axum::Extension;
use axum_jsonschema::Json;
use chrono::Utc;
use schemars::JsonSchema;

use crate::{
	runner::{Health, RUNNER_HEALTH},
	shutdown::Agent as Shutdown,
};

pub fn handler() -> ApiRouter {
	ApiRouter::new()
		.api_route("/", get(root))
		.api_route("/health-check", get(health_check))
		.api_route("/shutdown", post(shutdown))
}

#[derive(Debug, serde::Serialize, JsonSchema)]
pub struct RootResponse {
	/// Relative URL to Swagger UI
	pub docs_url: String,
	/// Relative URL to OpenAPI specification
	pub openapi_url: String,
}

#[allow(clippy::unused_async)]
pub async fn root() -> Json<RootResponse> {
	Json(RootResponse {
		docs_url: "/docs".to_string(),
		openapi_url: "/openapi.json".to_string(),
	})
}

#[derive(Debug, serde::Serialize, JsonSchema)]
pub struct HealthCheckSetup {
	/// Setup logs
	pub logs: String,
	/// Setup status
	pub status: String,
	/// Setup started time
	pub started_at: String,
	/// Setup completed time
	pub completed_at: String,
}

#[derive(Debug, serde::Serialize, JsonSchema)]
pub struct HealthCheck {
	/// Current health status
	pub status: Health,
	/// Setup information
	pub setup: HealthCheckSetup,
}

#[allow(clippy::unused_async)]
pub async fn health_check() -> Json<HealthCheck> {
	Json(HealthCheck {
		status: RUNNER_HEALTH.load(Ordering::SeqCst),
		setup: HealthCheckSetup {
			logs: String::new(),
			status: "succeeded".to_string(),
			started_at: Utc::now().to_rfc3339(),
			completed_at: Utc::now().to_rfc3339(),
		},
	})
}

#[allow(clippy::unused_async)]
pub async fn shutdown(Extension(shutdown): Extension<Shutdown>) -> Json<String> {
	shutdown.start();

	Json(String::new())
}

use std::sync::atomic::Ordering;

use axum::{
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::Utc;

use crate::{
    runner::RUNNER_HEALTH,
    schema::{HealthCheck, HealthCheckSetup, RootResponse},
    shutdown::Agent as Shutdown,
};

pub fn handler() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health-check", get(health_check))
        .route("/shutdown", post(shutdown))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "cog",
    operation_id = "root__get",
    responses(
        (status = 200, description = "Successful Response", body = [RootResponse])
    )
)]
#[allow(clippy::unused_async)]
pub async fn root() -> Json<RootResponse> {
    Json(RootResponse {
        docs_url: "/docs".to_string(),
        openapi_url: "/openapi.json".to_string(),
    })
}

#[utoipa::path(
    get,
    tag = "cog",
    path = "/health-check",
    operation_id = "healthcheck_health_check_get",
    responses(
        (status = 200, description = "Successful Response", body = [HealthCheck])
    )
)]
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

#[utoipa::path(
    post,
    tag = "cog",
    path = "/shutdown",
    operation_id = "start_shutdown_shutdown_post",
    responses(
        (status = 200, description = "Successful Response", body = [Json<()>])
    )
)]
#[allow(clippy::unused_async)]
pub async fn shutdown(Extension(shutdown): Extension<Shutdown>) -> Json<String> {
    shutdown.start();

    Json(String::new())
}

use std::sync::atomic::Ordering;

use aide::axum::{routing::post, ApiRouter};
use axum::{http::StatusCode, Extension};
use axum_jsonschema::Json;
use schemars::JsonSchema;
use serde_json::Value;

use crate::{
    errors::HTTPError,
    runner::{Error as RunnerError, Health, Runner, RUNNER_HEALTH},
};

pub fn handler() -> ApiRouter {
    ApiRouter::new().api_route("/predictions", post(create_prediction))
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct PredictionRequest<T = Value> {
    /// Input data
    pub input: T,
}

#[derive(Debug, serde::Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PredictionStatus {
    _Processing,
    Succeeded,
    Failed,
}

#[derive(Debug, serde::Serialize, JsonSchema)]
pub struct Prediction<T = Value> {
    /// Prediction status
    pub status: PredictionStatus,

    /// Prediction result
    pub output: Option<T>,

    /// Prediction started time
    pub error: Option<String>,
}

async fn create_prediction(
    Extension(runner): Extension<Runner>,
    Json(req): Json<PredictionRequest>,
) -> Result<Json<Prediction>, HTTPError> {
    if matches!(RUNNER_HEALTH.load(Ordering::SeqCst), Health::Busy) {
        return Err(
            HTTPError::new("Already running a prediction").with_status(StatusCode::CONFLICT)
        );
    }

    let Ok(input) = serde_json::to_value(req.input) else {
        return Err(HTTPError::new("Failed to serialize input"));
    };

    match runner.run(input).await {
        Ok(output) => Ok(Json(Prediction {
            error: None,
            output: Some(serde_json::from_value(output).unwrap()),
            status: PredictionStatus::Succeeded,
        })),
        Err(RunnerError::Serialization(_)) => Err(HTTPError::new("Failed to serialize input")),
        Err(error) => Ok(Json(Prediction {
            output: None,
            error: Some(error.to_string()),
            status: PredictionStatus::Failed,
        })),
    }
}

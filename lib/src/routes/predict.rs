use std::sync::atomic::Ordering;

use axum::{http::StatusCode, routing::post, Extension, Json, Router};

use crate::{
    errors::HTTPError,
    runner::{Health, Runner, RUNNER_HEALTH},
    schema::{Prediction, PredictionRequest, PredictionStatus},
};

pub fn handler() -> Router {
    Router::new().route("/predictions", post(create_prediction))
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

    let output = runner.run(req.input).await;

    Ok(Json(match output {
        Ok(output) => Prediction {
            error: None,
            output: Some(output),
            status: PredictionStatus::Succeeded,
        },
        Err(error) => Prediction {
            output: None,
            error: Some(error.to_string()),
            status: PredictionStatus::Failed,
        },
    }))
}

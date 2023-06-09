use aide::axum::{routing::post, ApiRouter};
use axum::{extract::Path, http::StatusCode, Extension};
use axum_jsonschema::Json;
use std::sync::atomic::Ordering;

use crate::{
	errors::HTTPError,
	prediction::{
		Extension as ExtractPrediction, Request as PredictionRequest, Response as Prediction,
	},
	runner::{Health, RUNNER_HEALTH},
};

pub fn handler() -> ApiRouter {
	ApiRouter::new()
		.api_route("/predictions", post(create_prediction))
		.api_route(
			"/predictions/:prediction_id/cancel",
			post(cancel_prediction),
		)
}

async fn create_prediction(
	Extension(prediction): ExtractPrediction,
	Json(req): Json<PredictionRequest>,
) -> Result<Json<Prediction>, HTTPError> {
	if matches!(RUNNER_HEALTH.load(Ordering::SeqCst), Health::Busy) {
		return Err(
			HTTPError::new("Already running a prediction").with_status(StatusCode::CONFLICT)
		);
	}

	let mut prediction = prediction.write().await;
	Ok(Json(prediction.init(req)?.run().await?))
}

async fn cancel_prediction(
	Path(id): Path<String>,
	Extension(prediction): ExtractPrediction,
) -> Result<Json<()>, HTTPError> {
	let mut prediction = prediction.write().await;
	prediction.cancel(id)?;
	drop(prediction);

	Ok(Json(()))
}

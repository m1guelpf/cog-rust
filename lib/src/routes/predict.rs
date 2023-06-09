use aide::axum::{routing::post, ApiRouter};
use axum::{extract::Path, http::StatusCode, Extension};
use axum_jsonschema::Json;

use crate::{
	errors::HTTPError,
	prediction::{
		Extension as ExtractPrediction, Request as PredictionRequest, Response as Prediction,
		SyncGuard,
	},
};

pub fn handler() -> ApiRouter {
	ApiRouter::new()
		.api_route("/predictions", post(create_prediction))
		.api_route("/predictions/:prediction_id", post(create_prediction))
		.api_route(
			"/predictions/:prediction_id/cancel",
			post(cancel_prediction),
		)
}

async fn create_prediction(
	id: Option<Path<String>>,
	Extension(prediction): ExtractPrediction,
	Json(req): Json<PredictionRequest>,
) -> Result<Json<Prediction>, HTTPError> {
	let id = id.map(|id| id.0);

	// If the user provides an ID, we check if there is already a prediction running with that ID and if so, wait for it to finish and return the result.
	if let Some(id) = id.clone() {
		let prediction = prediction.read().await;
		if let Some(prediction_id) = prediction.id.clone() {
			if Some(prediction_id.clone()) != Some(id.clone()) {
				tracing::debug!("Trying to run a named prediction {id} while another prediction {prediction_id} is running");
				return Err(HTTPError::new("Already running a prediction")
					.with_status(StatusCode::CONFLICT));
			}

			return Ok(Json(prediction.wait_for(id).await?));
		}
	}

	let mut prediction = SyncGuard::new(prediction.write().await);
	Ok(Json(prediction.init(id, req)?.run().await?))
}

async fn cancel_prediction(
	Path(id): Path<String>,
	Extension(prediction): ExtractPrediction,
) -> Result<Json<()>, HTTPError> {
	let mut prediction = prediction.write().await;
	prediction.cancel(&id)?;
	drop(prediction);

	Ok(Json(()))
}

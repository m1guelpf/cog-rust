use aide::axum::{
	routing::{post, put},
	ApiRouter,
};
use axum::{extract::Path, http::StatusCode, Extension, TypedHeader};
use axum_jsonschema::Json;
use cog_core::http::Status;

use crate::{
	errors::HTTPError,
	helpers::headers::Prefer,
	prediction::{Extension as ExtractPrediction, ResponseHelpers, SyncGuard},
};

pub fn handler() -> ApiRouter {
	ApiRouter::new()
		.api_route("/predictions", post(create_prediction))
		.api_route("/predictions/:prediction_id", put(create_prediction))
		.api_route(
			"/predictions/:prediction_id/cancel",
			post(cancel_prediction),
		)
}

async fn create_prediction(
	id: Option<Path<String>>,
	prefer: Option<TypedHeader<Prefer>>,
	Extension(prediction): ExtractPrediction,
	Json(req): Json<cog_core::http::Request>,
) -> Result<(StatusCode, Json<cog_core::http::Response>), HTTPError> {
	let id = id.map(|id| id.0);
	let respond_async = prefer
		.map(|prefer| prefer.0)
		.unwrap_or_default()
		.has("respond-async");

	tracing::debug!(
		"Received {}prediction request{}.",
		if respond_async { "async " } else { "" },
		id.as_ref()
			.map_or(String::new(), |id| format!(" with id {id}")),
	);
	tracing::trace!("{req:?}");

	let r_prediction = prediction.read().await;

	// If a named prediction is already running...
	if let Some(prediction_id) = r_prediction.id.clone() {
		// ...and the request is for a different prediction, return an error.
		if let Some(id) = id.clone() {
			if Some(prediction_id.clone()) != Some(id.clone()) {
				tracing::debug!(
					"Trying to run a named prediction {id} while another prediction {prediction_id} is running"
				);
				return Err(HTTPError::new("Already running a prediction")
					.with_status(StatusCode::CONFLICT));
			}

			// ...and this is an async request, return the current response.
			if respond_async {
				return Ok((
					StatusCode::ACCEPTED,
					Json(r_prediction.response.clone().unwrap()),
				));
			}

			// wait for the current prediction to complete
			return Ok((StatusCode::OK, Json(r_prediction.wait_for(id).await?)));
		}
	}

	// If the request is synchronous, run the prediction and return the result.
	if !respond_async {
		drop(r_prediction);
		let mut prediction = SyncGuard::new(prediction.write().await);

		return Ok((StatusCode::OK, Json(prediction.init(id, req)?.run().await?)));
	}

	// If there's a running prediction, return an error.
	if !matches!(r_prediction.status, Status::Idle) {
		return Err(
			HTTPError::new("Already running a prediction").with_status(StatusCode::CONFLICT)
		);
	}

	// Throw an error if the request is invalid.
	r_prediction.validate(&req.input)?;
	drop(r_prediction);

	let thread_req = req.clone();
	let thread_id = id.clone();
	tokio::spawn(async move {
		tracing::debug!("Running prediction asynchronously: {:?}", thread_id);

		let mut prediction = prediction.write().await;

		prediction.init(thread_id.clone(), thread_req).unwrap();
		prediction.process().unwrap().await;
		prediction.reset();
		drop(prediction);

		tracing::debug!("Asynchronous prediction complete: {thread_id:?}");
	});

	Ok((
		StatusCode::ACCEPTED,
		Json(cog_core::http::Response::starting(id, req)),
	))
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

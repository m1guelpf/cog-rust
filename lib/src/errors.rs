use aide::OperationOutput;
use axum::{
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use jsonschema::ErrorIterator;
use serde_json::{json, Value};

use crate::prediction::Error as PredictionError;

#[derive(Debug)]
pub struct HTTPError {
	detail: Value,
	status_code: StatusCode,
}

impl HTTPError {
	pub fn new(detail: &str) -> Self {
		Self {
			detail: detail.into(),
			status_code: StatusCode::UNPROCESSABLE_ENTITY,
		}
	}

	pub const fn with_status(mut self, status_code: StatusCode) -> Self {
		self.status_code = status_code;
		self
	}
}

impl IntoResponse for HTTPError {
	fn into_response(self) -> Response {
		(self.status_code, Json(json!({ "detail": self.detail }))).into_response()
	}
}

impl OperationOutput for HTTPError {
	type Inner = Self;
}

#[derive(Debug, serde::Serialize)]
pub struct ValidationError {
	msg: String,
	loc: Vec<String>,
}

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[error("Validation Errors")]
pub struct ValidationErrorSet {
	errors: Vec<ValidationError>,
}

impl ValidationErrorSet {
	pub fn fill_loc(mut self, loc: &[&str]) -> Self {
		self.errors
			.iter_mut()
			.map(|error| {
				error.loc = loc
					.iter()
					.map(ToString::to_string)
					.chain(error.loc.clone())
					.collect();
			})
			.for_each(drop);

		self
	}
}

impl From<ErrorIterator<'_>> for ValidationErrorSet {
	fn from(e: ErrorIterator<'_>) -> Self {
		Self {
			errors: e
				.map(|e| ValidationError {
					msg: e.to_string(),
					loc: e.instance_path.into_vec(),
				})
				.collect(),
		}
	}
}

#[allow(clippy::fallible_impl_from)]
impl From<ValidationErrorSet> for HTTPError {
	fn from(e: ValidationErrorSet) -> Self {
		Self {
			status_code: StatusCode::UNPROCESSABLE_ENTITY,
			detail: serde_json::to_value(e.errors).unwrap(),
		}
	}
}

#[allow(clippy::fallible_impl_from)]
impl From<PredictionError> for HTTPError {
	fn from(e: PredictionError) -> Self {
		match e {
			PredictionError::Validation(e) => e.into(),
			PredictionError::AlreadyRunning => Self {
				status_code: StatusCode::CONFLICT,
				detail: serde_json::to_value(e.to_string()).unwrap(),
			},
			PredictionError::NotComplete => Self {
				status_code: StatusCode::INTERNAL_SERVER_ERROR,
				detail: serde_json::to_value(e.to_string()).unwrap(),
			},
		}
	}
}

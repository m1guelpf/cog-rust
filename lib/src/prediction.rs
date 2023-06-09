use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use map_macro::hash_map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use url::Url;

use crate::{
	errors::ValidationErrorSet,
	runner::{Error as RunnerError, Runner},
	shutdown::Shutdown,
	Cog,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
	#[serde(skip)]
	Idle,

	Failed,
	Starting,
	Canceled,
	Succeeded,
	Processing,
}

pub type Extension = axum::Extension<Arc<RwLock<Prediction>>>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Attempted to re-initialize a prediction")]
	AlreadyRunning,

	#[error("Prediction is not yet complete")]
	NotComplete,

	#[error("Failed to run prediction: {0}")]
	Validation(#[from] ValidationErrorSet),
}

pub struct Prediction {
	runner: Runner,
	status: Status,
	pub shutdown: Shutdown,
	request: Option<Request>,
	response: Option<Response>,
}

impl Prediction {
	pub fn setup<T: Cog + 'static>(shutdown: Shutdown) -> Self {
		Self {
			request: None,
			response: None,
			status: Status::Idle,
			shutdown: shutdown.clone(),
			runner: Runner::new::<T>(shutdown),
		}
	}

	pub fn init(&mut self, req: Request) -> Result<&mut Self, Error> {
		if let Some(existing_req) = self.request.as_ref() {
			if req.id.is_none() || req.id == existing_req.id {
				return Err(Error::AlreadyRunning);
			}
		}

		self.request = Some(req);
		self.status = Status::Starting;

		Ok(self)
	}

	pub async fn run(&mut self) -> Result<Response, Error> {
		self.process()?.await;

		self.result()
	}

	pub fn process(&mut self) -> Result<impl Future<Output = ()> + '_, Error> {
		if !matches!(self.status, Status::Starting) {
			return Err(Error::AlreadyRunning);
		}

		let req = self.request.clone().unwrap();
		self.runner
			.validate(&req.input)
			.map_err(|e| e.fill_loc(&["body", "input"]))?;

		self.status = Status::Processing;

		Ok(async move {
			(self.status, self.response) = match self.runner.run(req.input.clone()).await {
				Ok((output, predict_time)) => (
					Status::Succeeded,
					Some(Response::success(req, output, predict_time)),
				),
				Err(error) => (Status::Failed, Some(Response::error(req, &error))),
			};
		})
	}

	pub fn result(&mut self) -> Result<Response, Error> {
		if !matches!(self.status, Status::Succeeded | Status::Failed) {
			return Err(Error::NotComplete);
		}

		let response = self.response.clone().ok_or(Error::NotComplete)?;
		self.reset();

		Ok(response)
	}

	pub fn cancel(&mut self) -> Result<&mut Self, Error> {
		if !matches!(self.status, Status::Processing) {
			return Err(Error::AlreadyRunning);
		}

		self.status = Status::Canceled;

		Ok(self)
	}

	fn reset(&mut self) {
		self.request = None;
		self.response = None;
		self.status = Status::Idle;
	}

	pub fn extension(self) -> Extension {
		axum::Extension(Arc::new(RwLock::new(self)))
	}
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub enum WebhookEvent {
	Start,
	Output,
	Logs,
	Completed,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct Request<T = Value> {
	pub id: Option<String>,
	pub webhook: Option<Url>,
	pub webhook_event_filters: Option<Vec<WebhookEvent>>,
	pub output_file_prefix: Option<String>,

	pub input: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Response<Req = Value, Res = Value> {
	pub input: Option<Req>,
	pub output: Option<Res>,

	pub id: Option<String>,
	pub version: Option<String>,

	pub created_at: Option<DateTime<Utc>>,
	pub started_at: Option<DateTime<Utc>>,
	pub completed_at: Option<DateTime<Utc>>,

	pub logs: String,
	pub status: Status,
	pub error: Option<String>,

	metrics: Option<HashMap<String, Value>>,
}

impl Response {
	pub fn success(req: Request, output: Value, predict_time: Duration) -> Self {
		Self {
			id: req.id,
			output: Some(output),
			input: Some(req.input),
			status: Status::Succeeded,
			metrics: Some(hash_map! {
				"predict_time".to_string() => predict_time.as_secs_f64().into()
			}),
			..Self::default()
		}
	}
	pub fn error(req: Request, error: &RunnerError) -> Self {
		Self {
			id: req.id,
			input: Some(req.input),
			status: Status::Failed,
			error: Some(error.to_string()),
			..Self::default()
		}
	}
}

impl Default for Response {
	fn default() -> Self {
		Self {
			id: None,
			error: None,
			input: None,
			output: None,
			metrics: None,
			version: None,
			created_at: None,
			logs: String::new(),
			status: Status::Starting,
			started_at: Utc::now().into(),
			completed_at: Utc::now().into(),
		}
	}
}

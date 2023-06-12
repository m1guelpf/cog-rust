use std::{
	collections::HashMap,
	future::Future,
	sync::{atomic::Ordering, Arc},
	time::Duration,
};

use chrono::{DateTime, Utc};
use map_macro::hash_map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use url::Url;

use crate::{
	errors::ValidationErrorSet,
	runner::{Error as RunnerError, Health, Runner, RUNNER_HEALTH},
	shutdown::Shutdown,
	webhooks::{WebhookEvent, WebhookSender},
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

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
	#[error("Attempted to re-initialize a prediction")]
	AlreadyRunning,

	#[error("Prediction is not yet complete")]
	NotComplete,

	#[error("The requested prediction does not exist")]
	Unknown,

	#[error("Failed to wait for prediction: {0}")]
	ReceiverError(#[from] flume::RecvError),

	#[error("Failed to run prediction: {0}")]
	Validation(#[from] ValidationErrorSet),
}

pub struct Prediction {
	runner: Runner,
	pub status: Status,
	pub id: Option<String>,
	pub shutdown: Shutdown,
	webhooks: WebhookSender,
	cancel: flume::Sender<()>,
	pub request: Option<Request>,
	pub response: Option<Response>,
	complete: Option<flume::Receiver<Response>>,
}

impl Prediction {
	pub fn setup<T: Cog + 'static>(shutdown: Shutdown) -> Self {
		let (cancel_tx, cancel_rx) = flume::unbounded();

		Self {
			id: None,
			request: None,
			complete: None,
			response: None,
			cancel: cancel_tx,
			status: Status::Idle,
			shutdown: shutdown.clone(),
			webhooks: WebhookSender::new().unwrap(),
			runner: Runner::new::<T>(shutdown, cancel_rx),
		}
	}

	pub fn init(&mut self, id: Option<String>, req: Request) -> Result<&mut Self, Error> {
		if !matches!(self.status, Status::Idle) {
			tracing::debug!("Attempted to re-initialize a prediction");
			return Err(Error::AlreadyRunning);
		}

		self.validate(&req.input)
			.map_err(|e| e.fill_loc(&["body", "input"]))?;

		tracing::debug!("Initializing prediction: {id:?}");

		self.id = id;
		self.request = Some(req);
		self.status = Status::Starting;

		Ok(self)
	}

	pub fn validate(&self, input: &Value) -> Result<(), ValidationErrorSet> {
		self.runner.validate(input)
	}

	pub async fn run(&mut self) -> Result<Response, Error> {
		self.process()?.await;

		self.result()
	}

	pub async fn wait_for(&self, id: String) -> Result<Response, Error> {
		if self.id != Some(id.clone()) {
			tracing::debug!("Attempted to wait for prediction with unknown ID: {id:?}");
			return Err(Error::Unknown);
		}

		if let Some(response) = self.response.clone() {
			tracing::debug!("Prediction already complete: {id:?}");
			return Ok(response);
		}

		if !matches!(self.status, Status::Processing) {
			tracing::debug!("Attempted to wait for prediction that isn't running: {id:?}");
			return Err(Error::AlreadyRunning);
		}

		tracing::debug!("Waiting for prediction: {id:?}");
		let complete = self.complete.as_ref().unwrap();
		Ok(complete.recv_async().await?)
	}

	pub fn process(&mut self) -> Result<impl Future<Output = ()> + '_, Error> {
		if !matches!(self.status, Status::Starting) {
			tracing::debug!(
				"Attempted to process prediction while not ready: {:?}",
				self.id
			);
			return Err(Error::AlreadyRunning);
		}

		let req = self.request.clone().unwrap();
		self.status = Status::Processing;

		let (complete_tx, complete_rx) = flume::bounded(1);
		self.complete = Some(complete_rx);

		Ok(async move {
			let started_at = Utc::now();
			tracing::debug!("Running prediction: {:?}", self.id);

			self.status = Status::Processing;
			self.response = Some(Response::starting(self.id.clone(), req.clone()));
			if let Err(e) = self.webhooks.starting(self).await {
				tracing::error!("Failed to send start webhook for prediction: {e:?}",);
			};

			tokio::select! {
				_ = self.shutdown.handle() => {
					tracing::debug!("Shutdown requested. Cancelling running prediction: {:?}", self.id);
					return;
				},
				output = self.runner.run(req.clone()) => {
					tracing::debug!("Prediction complete: {:?}", self.id);

					match output {
						Ok((output, predict_time)) => {
							self.status = Status::Succeeded;
							self.response = Some(Response::success(self.id.clone(), req, output, predict_time, started_at));
						},
						Err(RunnerError::Canceled) => {
							self.status = Status::Canceled;
							self.response = Some(Response::canceled(self.id.clone(), req, started_at));

						},
						Err(error) => {
							self.status = Status::Failed;
							self.response = Some(Response::error(self.id.clone(), req, &error, started_at));
						}
					}

					if let Err(e) = self.webhooks.finished(self, self.response.clone().unwrap()).await {
						tracing::error!("Failed to send finished webhook for prediction: {e:?}",);
					};
				}
			}
			complete_tx.send(self.response.clone().unwrap()).unwrap();
		})
	}

	pub fn result(&mut self) -> Result<Response, Error> {
		if !matches!(
			self.status,
			Status::Succeeded | Status::Failed | Status::Canceled
		) {
			tracing::debug!(
				"Attempted to get result of prediction that is not complete: {:?}",
				self.id
			);
			return Err(Error::NotComplete);
		}

		tracing::debug!("Getting result of prediction: {:?}", self.id);
		let response = self.response.clone().ok_or(Error::NotComplete)?;
		self.reset();

		Ok(response)
	}

	pub fn cancel(&mut self, id: &str) -> Result<&mut Self, Error> {
		if self.id != Some(id.to_string()) {
			tracing::debug!("Attempted to cancel prediction with unknown ID: {id}");
			return Err(Error::Unknown);
		}

		if !matches!(self.status, Status::Processing) {
			tracing::debug!("Attempted to cancel prediction that is not running: {id}");
			return Err(Error::AlreadyRunning);
		}

		tracing::debug!("Canceling prediction: {id}");
		self.cancel.send(()).unwrap();
		self.status = Status::Canceled;

		Ok(self)
	}

	pub fn reset(&mut self) {
		tracing::debug!("Resetting prediction");

		self.id = None;
		self.request = None;
		self.response = None;
		self.complete = None;
		self.status = Status::Idle;
	}

	pub fn extension(self) -> Extension {
		axum::Extension(Arc::new(RwLock::new(self)))
	}
}

pub struct SyncGuard<'a> {
	prediction: tokio::sync::RwLockWriteGuard<'a, Prediction>,
}

impl<'a> SyncGuard<'a> {
	pub fn new(prediction: tokio::sync::RwLockWriteGuard<'a, Prediction>) -> Self {
		Self { prediction }
	}

	pub fn init(&mut self, id: Option<String>, req: Request) -> Result<&mut Self, Error> {
		self.prediction.init(id, req)?;
		Ok(self)
	}

	pub async fn run(&mut self) -> Result<Response, Error> {
		self.prediction.run().await
	}
}

impl Drop for SyncGuard<'_> {
	fn drop(&mut self) {
		tracing::debug!("SyncGuard dropped, resetting prediction");

		self.prediction.reset();
		if matches!(RUNNER_HEALTH.load(Ordering::SeqCst), Health::Busy) {
			self.prediction.cancel.send(()).unwrap();
		}
	}
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct Request<T = Value> {
	pub webhook: Option<Url>,
	pub webhook_event_filters: Option<Vec<WebhookEvent>>,
	pub output_file_prefix: Option<Url>,

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
	pub fn success(
		id: Option<String>,
		req: Request,
		output: Value,
		predict_time: Duration,
		started_at: DateTime<Utc>,
	) -> Self {
		Self {
			id,
			output: Some(output),
			input: Some(req.input),
			status: Status::Succeeded,
			started_at: Some(started_at),
			completed_at: Some(Utc::now()),
			metrics: Some(hash_map! {
				"predict_time".to_string() => predict_time.as_secs_f64().into()
			}),
			..Self::default()
		}
	}
	pub fn error(
		id: Option<String>,
		req: Request,
		error: &RunnerError,
		started_at: DateTime<Utc>,
	) -> Self {
		Self {
			id,
			input: Some(req.input),
			status: Status::Failed,
			started_at: Some(started_at),
			error: Some(error.to_string()),
			..Self::default()
		}
	}

	pub fn starting(id: Option<String>, req: Request) -> Self {
		Self {
			id,
			input: Some(req.input),
			status: Status::Processing,
			started_at: Some(Utc::now()),
			..Self::default()
		}
	}

	pub fn canceled(id: Option<String>, req: Request, started_at: DateTime<Utc>) -> Self {
		Self {
			id,
			input: Some(req.input),
			status: Status::Canceled,
			started_at: Some(started_at),
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

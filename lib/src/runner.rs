use anyhow::Result;
use atomic_enum::atomic_enum;
use cog_core::{Cog, CogResponse};
use jsonschema::JSONSchema;
use schemars::{schema_for, JsonSchema};
use serde_json::Value;
use std::{
	env,
	sync::{atomic::Ordering, Arc, Mutex},
	time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};
use tracing::{trace_span, Instrument};

use crate::{errors::ValidationErrorSet, shutdown::Shutdown};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Runner is busy")]
	Busy,

	#[error("Prediction was canceled")]
	Canceled,

	#[error("Failed to validate input.")]
	Validation(ValidationErrorSet),

	#[error("Failed to run prediction: {0}")]
	Prediction(#[from] anyhow::Error),
}

#[atomic_enum]
#[derive(serde::Serialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Health {
	Unknown,
	Starting,
	Ready,
	Busy,
	SetupFailed,
}

pub static RUNNER_HEALTH: AtomicHealth = AtomicHealth::new(Health::Unknown);

type ResponseSender = oneshot::Sender<Result<(Value, Duration), Error>>;

#[derive(Clone)]
pub struct Runner {
	schema: Arc<JSONSchema>,
	sender: mpsc::Sender<(ResponseSender, cog_core::http::Request)>,
}

impl Runner {
	pub fn new<T: Cog + 'static>(shutdown: Shutdown, cancel: flume::Receiver<()>) -> Self {
		RUNNER_HEALTH.swap(Health::Starting, Ordering::SeqCst);

		let (sender, mut rx) = mpsc::channel::<(ResponseSender, cog_core::http::Request)>(1);

		let handle_shutdown = shutdown.clone();
		let handle = tokio::spawn(async move {
			tracing::info!("Running setup()...");
			let cog = tokio::select! {
				() = tokio::time::sleep(Duration::from_secs(5 * 60)) => {
					tracing::error!("Failed run setup(): Timed out");
					RUNNER_HEALTH.swap(Health::SetupFailed, Ordering::SeqCst);
					handle_shutdown.start();
					return;
				}
				cog = T::setup().instrument(trace_span!("cog_setup")) => {
					match cog {
						Ok(cog) => Arc::new(Mutex::new(cog)),
						Err(error) => {
							tracing::error!("Failed run setup(): {error}");
							RUNNER_HEALTH.swap(Health::SetupFailed, Ordering::SeqCst);
							handle_shutdown.start();
							return;
						}
					}
				}
			};

			tracing::debug!("setup() finished. Cog is ready to accept predictions.");
			RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);
			if env::var("KUBERNETES_SERVICE_HOST").is_ok() {
				if let Err(err) = tokio::fs::create_dir_all("/var/run/cog").await {
					tracing::error!("Failed to create cog runtime state directory: {err}");
					RUNNER_HEALTH.swap(Health::SetupFailed, Ordering::SeqCst);
					handle_shutdown.start();
					return;
				}

				if let Err(error) = tokio::fs::File::create("/var/run/cog/ready").await {
					tracing::error!("Failed to signal cog is ready: {error}");
					RUNNER_HEALTH.swap(Health::SetupFailed, Ordering::SeqCst);
					handle_shutdown.start();
					return;
				}
			}

			// Cog is not Sync, so we wrap it with a Mutex and this function to run it from an async context (and thus make it cancellable).
			let run_prediction_async = |input| {
				async {
					let cog = cog.lock().unwrap();

					cog.predict(input)
				}
				.instrument(trace_span!("cog_predict"))
			};

			while let Some((tx, req)) = rx.recv().await {
				tracing::debug!("Processing prediction: {req:?}");
				RUNNER_HEALTH.swap(Health::Busy, Ordering::SeqCst);

				// We need spawn_blocking here to (sneakily) allow blocking code in serde Deserialize impls (used in `Path`, for example).
				let input = req.input.clone();
				let input =
					tokio::task::spawn_blocking(move || serde_json::from_value(input).unwrap())
						.await
						.unwrap();

				let start = Instant::now();
				tokio::select! {
					_ = cancel.recv_async() => {
						let _ = tx.send(Err(Error::Canceled));
						tracing::debug!("Prediction canceled");
					},
					response = run_prediction_async(input) => {
						tracing::debug!("Prediction complete: {response:?}");
						let _ = tx.send(match response {
							Err(error) => Err(Error::Prediction(error)),
							Ok(response) => match response.into_response(req).await {
								Err(error) => Err(Error::Prediction(error)),
								Ok(response) => Ok((response, start.elapsed())),
							},
						});
					}
				}

				RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);
			}
		});

		tokio::spawn(async move {
			shutdown.handle().await;
			tracing::debug!("Shutting down runner...");
			handle.abort();
		});

		let schema = jsonschema::JSONSchema::compile(
			&serde_json::to_value(schema_for!(T::Request)).unwrap(),
		)
		.unwrap();

		Self {
			sender,
			schema: Arc::new(schema),
		}
	}

	pub fn validate(&self, input: &Value) -> Result<(), ValidationErrorSet> {
		self.schema.validate(input)?;

		Ok(())
	}

	pub async fn run(&self, req: cog_core::http::Request) -> Result<(Value, Duration), Error> {
		if !matches!(RUNNER_HEALTH.load(Ordering::SeqCst), Health::Ready) {
			tracing::debug!("Failed to run prediction: runner is busy");
			return Err(Error::Busy);
		}

		self.validate(&req.input).map_err(Error::Validation)?;
		RUNNER_HEALTH.swap(Health::Busy, Ordering::SeqCst);

		let (tx, rx) = oneshot::channel();

		tracing::debug!("Sending prediction to runner: {req:?}");
		let _ = self.sender.send((tx, req)).await;
		tracing::debug!("Waiting for prediction response...");
		let result = rx.await.unwrap();
		tracing::debug!("Prediction response received: {result:?}");

		RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);

		result
	}
}

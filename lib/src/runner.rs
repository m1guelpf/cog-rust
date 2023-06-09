use anyhow::Result;
use atomic_enum::atomic_enum;
use jsonschema::JSONSchema;
use schemars::{schema_for, JsonSchema};
use serde_json::Value;
use std::{
	sync::{atomic::Ordering, Arc},
	time::Duration,
};
use tokio::sync::{mpsc, oneshot};

use crate::{
	errors::ValidationErrorSet, helpers::with_timing, shutdown::Shutdown, spec::Cog, CogResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Runner is busy")]
	Busy,

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
	sender: mpsc::Sender<(ResponseSender, Value)>,
}

impl Runner {
	pub fn new<T: Cog + 'static>(shutdown: Shutdown) -> Self {
		RUNNER_HEALTH.swap(Health::Starting, Ordering::SeqCst);

		let (sender, mut rx) = mpsc::channel::<(ResponseSender, Value)>(1);

		let handle_shutdown = shutdown.clone();
		let handle = tokio::spawn(async move {
			let Ok(cog) = T::setup().await else {
                RUNNER_HEALTH.swap(Health::SetupFailed, Ordering::SeqCst);
                handle_shutdown.start();
                return;
            };

			RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);

			while let Some((tx, input)) = rx.recv().await {
				RUNNER_HEALTH.swap(Health::Busy, Ordering::SeqCst);

				tx.send(
					match with_timing(|| cog.predict(serde_json::from_value(input).unwrap())) {
						(Ok(response), predict_time) => {
							Ok((response.into_response(), predict_time))
						},
						(Err(error), _) => Err(Error::Prediction(error)),
					},
				)
				.unwrap();

				RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);
			}
		});

		tokio::spawn(async move {
			shutdown.handle().await;
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

	pub async fn run(&self, input: Value) -> Result<(Value, Duration), Error> {
		if !matches!(RUNNER_HEALTH.load(Ordering::SeqCst), Health::Ready) {
			return Err(Error::Busy);
		}

		self.validate(&input).map_err(Error::Validation)?;
		RUNNER_HEALTH.swap(Health::Busy, Ordering::SeqCst);

		let (tx, rx) = oneshot::channel();

		self.sender.send((tx, input)).await.unwrap_or_default();

		let result = rx.await.unwrap();

		RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);

		result
	}
}

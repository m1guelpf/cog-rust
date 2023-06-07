use anyhow::Result;
use atomic_enum::atomic_enum;
use axum::Extension;
use schemars::JsonSchema;
use serde_json::Value;
use std::sync::atomic::Ordering;
use tokio::sync::{mpsc, oneshot};

use crate::{shutdown::Shutdown, spec::Cog, CogResponse};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Runner is busy")]
    Busy,

    #[error("Failed to serialize input: {0}")]
    Serialization(#[from] serde_json::Error),

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

#[derive(Clone)]
pub struct Runner {
    sender: mpsc::Sender<(oneshot::Sender<Result<Value, Error>>, Value)>,
}

impl Runner {
    pub fn new<T: Cog + 'static>(shutdown: Shutdown) -> Self {
        RUNNER_HEALTH.swap(Health::Starting, Ordering::SeqCst);

        let (sender, mut rx) = mpsc::channel::<(oneshot::Sender<Result<Value, Error>>, Value)>(1);

        let handle_shutdown = shutdown.clone();
        let handle = tokio::spawn(async move {
            let Ok(cog) = T::setup().await else {
                RUNNER_HEALTH.swap(Health::SetupFailed, Ordering::SeqCst);
                handle_shutdown.start();
                return;
            };

            RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);

            while let Some((tx, input)) = rx.recv().await {
                let input = match serde_json::from_value(input) {
                    Ok(input) => input,
                    Err(e) => {
                        tx.send(Err(Error::Serialization(e))).unwrap();
                        continue;
                    }
                };

                let Ok(response) = cog.predict(input) else {
                    tx.send(Err(Error::Prediction(anyhow::anyhow!(
                        "Failed to run prediction"
                    )))).unwrap();
                    continue;
                };

                tx.send(Ok(response.into_response())).unwrap();
            }
        });

        tokio::spawn(async move {
            shutdown.handle().await;
            handle.abort();
        });

        Self { sender }
    }

    pub async fn run(&self, input: Value) -> Result<Value, Error> {
        if !matches!(RUNNER_HEALTH.load(Ordering::SeqCst), Health::Ready) {
            return Err(Error::Busy);
        }

        RUNNER_HEALTH.swap(Health::Busy, Ordering::SeqCst);

        let (tx, rx) = oneshot::channel();

        self.sender.send((tx, input)).await.unwrap_or_default();

        let result = rx.await.unwrap();

        RUNNER_HEALTH.swap(Health::Ready, Ordering::SeqCst);

        result
    }

    pub fn extension(&self) -> Extension<Self> {
        Extension(self.clone())
    }
}
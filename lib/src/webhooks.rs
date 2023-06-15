use std::env;

use anyhow::Result;
use axum::http::{HeaderMap, HeaderValue};
use cog_core::http::WebhookEvent;
use reqwest::Client;
use url::Url;

use crate::prediction::{Prediction, ResponseHelpers};

pub struct WebhookSender {
	client: Client,
}

impl WebhookSender {
	pub fn new() -> Result<Self> {
		let mut headers = HeaderMap::new();
		let client = Client::builder();

		if let Ok(token) = env::var("WEBHOOK_AUTH_TOKEN") {
			let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))?;
			authorization.set_sensitive(true);
			headers.insert("Authorization", authorization);
		}

		Ok(Self {
			client: client
				.user_agent(format!("cog-worker/{}", env!("CARGO_PKG_VERSION")))
				.default_headers(headers)
				.build()?,
		})
	}

	pub async fn starting(&self, prediction: &Prediction) -> Result<()> {
		let request = prediction.request.clone().unwrap();
		if !Self::should_send(&request, WebhookEvent::Start) {
			return Ok(());
		}

		self.send(
			request.webhook.clone().unwrap(),
			cog_core::http::Response::starting(prediction.id.clone(), request),
		)
		.await?;

		Ok(())
	}

	pub async fn finished(
		&self,
		prediction: &Prediction,
		response: cog_core::http::Response,
	) -> Result<()> {
		let request = prediction.request.clone().unwrap();
		if !Self::should_send(&request, WebhookEvent::Completed) {
			return Ok(());
		}

		self.send(request.webhook.clone().unwrap(), response)
			.await?;

		Ok(())
	}

	fn should_send(req: &cog_core::http::Request, event: WebhookEvent) -> bool {
		req.webhook.is_some()
			&& req
				.webhook_event_filters
				.as_ref()
				.map_or(true, |filters| filters.contains(&event))
	}

	async fn send(
		&self,
		url: Url,
		res: cog_core::http::Response,
	) -> Result<reqwest::Response, reqwest::Error> {
		tracing::debug!("Sending webhook to {url}");
		tracing::trace!("{res:?}");

		self.client.post(url).json(&res).send().await
	}
}

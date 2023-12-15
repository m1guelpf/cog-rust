use anyhow::{bail, Context};
use cog_core::http::{HTTPValidationError, Response};
use indoc::formatdoc;
use map_macro::hash_map;
use reqwest::StatusCode;
use schemars::schema::SchemaObject;
use serde_json::{json, Value};
use std::{
	collections::HashMap,
	time::{Duration, Instant},
};
use tokio::time::sleep;

use super::RunOptions;
use crate::docker::Docker;

#[derive(Debug)]
pub struct Predictor {
	image: String,
	port: Option<u16>,
	container_id: Option<String>,
}

impl Predictor {
	pub const fn new(image: String) -> Self {
		Self {
			image,
			port: None,
			container_id: None,
		}
	}

	pub fn get_schema(&self) -> anyhow::Result<SchemaObject> {
		let image = Docker::inspect_image(&self.image)?;

		serde_json::from_str::<SchemaObject>(
			image
				.as_array()
				.and_then(|v| v.first())
				.and_then(|v| {
					v.get("Config")
						.and_then(Value::as_object)
						.and_then(|v| v.get("Labels"))
						.and_then(Value::as_object)
						.and_then(|v| v.get("org.cogmodel.openapi_schema"))
						.and_then(Value::as_str)
				})
				.context("Failed to get schema label. Is this a Cog model?")?,
		)
		.context("Failed to parse schema")
	}

	pub async fn start(&mut self) {
		let container_id = Docker::run(RunOptions {
			detach: true,
			image: self.image.clone(),
			ports: hash_map! { 0 => 5000 },
			env: vec!["RUST_LOG=error".to_string()],
			..RunOptions::default()
		})
		.unwrap();

		self.container_id = Some(container_id.clone());
		self.port = Some(Docker::find_port(&container_id, 5000).unwrap());

		Docker::tail_logs(&container_id).unwrap();
		self.wait_for_server().await.unwrap();
	}

	pub async fn predict(&self, inputs: HashMap<String, String>) -> anyhow::Result<Response> {
		let port = self
			.port
			.as_ref()
			.context("Trying to predict with non-running container.")?;

		let client = reqwest::Client::new();

		let res = client
			.post(format!("http://localhost:{port}/predictions"))
			.json(&json!({ "input": inputs }))
			.send()
			.await
			.context("Failed to send request")?;

		if matches!(res.status(), StatusCode::UNPROCESSABLE_ENTITY) {
			let text = res.text().await?;
			let errors = serde_json::from_str::<HTTPValidationError>(&text).context(
                format!("/predictions call returned status 422, and the response body failed to decode: {text}")
            )?;

			bail!(formatdoc! {"
                The inputs you passed to cog predict could not be validated:

                {}

                You can provide an input with -i. For example:

                    cog predict -i blur=3.5

                If your input is a local file, you need to prefix the path with @ to tell Cog to read the file contents. For example:

                    cog predict -i path=@image.jpg
            ", errors.detail.iter().map(|e| e.msg.clone()).collect::<Vec<_>>().join("\n")});
		}

		if !matches!(res.status(), StatusCode::OK) {
			bail!("/predictions call returned status {}", res.status());
		}

		let text = res.text().await?;
		serde_json::from_str::<Response>(&text)
			.context("Failed to decode prediction response: {text}")
	}

	pub fn stop(&self) -> anyhow::Result<()> {
		let container_id = self
			.container_id
			.as_ref()
			.context("Trying to stop non-running container.")?;

		Ok(Docker::stop(container_id)?)
	}

	pub async fn wait_for_server(&self) -> anyhow::Result<()> {
		let start = Instant::now();

		let container_id = self
			.container_id
			.as_ref()
			.context("Waiting for non-running container.")?;

		let client = reqwest::Client::new();

		loop {
			if start.elapsed().as_secs() > 300 {
				return Err(anyhow::anyhow!("Timed out"));
			}

			sleep(Duration::from_millis(100)).await;

			let container = Docker::inspect_container(container_id)
				.context("Failed to get container status")?;

			let state = container
				.as_array()
				.and_then(|v| v.first())
				.and_then(Value::as_object)
				.and_then(|v| v.get("State"))
				.and_then(Value::as_object)
				.and_then(|v| v.get("Status"))
				.and_then(Value::as_str)
				.expect("Container exited unexpectedly");

			if state == "exited" || state == "dead" {
				bail!("Container exited unexpectedly");
			}

			let res = client
				.get(format!(
					"http://localhost:{}/health-check",
					self.port.unwrap()
				))
				.send()
				.await
				.and_then(reqwest::Response::error_for_status);

			let Ok(res) = res else {
				continue;
			};

			let status = res
				.json::<Value>()
				.await
				.ok()
				.and_then(|v| v.get("status").cloned())
				.and_then(|s| s.as_str().map(str::to_string))
				.context("Container healthcheck returned invalid response")?;

			match status.as_str() {
				"STARTING" => continue,
				"READY" => return Ok(()),
				"SETUP_FAILED" => bail!("Model setup failed"),
				_ => bail!("Container healthcheck returned unexpected status: {status}"),
			}
		}
	}
}

impl Drop for Predictor {
	fn drop(&mut self) {
		let _ = self.stop();
	}
}

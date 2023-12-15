use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum WebhookEvent {
	Start,
	Output,
	Logs,
	Completed,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct Request<T = Value> {
	pub webhook: Option<Url>,
	pub webhook_event_filters: Option<Vec<WebhookEvent>>,

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

	pub metrics: Option<HashMap<String, Value>>,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct HTTPValidationError {
	pub detail: Vec<ValidationError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidationError {
	pub msg: String,
	pub loc: Vec<String>,
}

use anyhow::Result;
use core::fmt::Debug;
use mime_guess::Mime;
use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::Serialize;
use std::{
	env::{self, temp_dir},
	fs::File,
	path::PathBuf,
	str::FromStr,
};
use url::Url;
use uuid::Uuid;

use crate::helpers::{base64_decode, base64_encode, url_join};

#[derive(Debug, serde::Deserialize)]
struct UploadResponse {
	url: Url,
}

#[derive(Debug)]
pub struct Path(PathBuf);

impl Path {
	/// Create a new path from a url
	///
	/// # Errors
	///
	/// Returns an error if the url cannot be downloaded or a temporary file cannot be created.
	pub(crate) fn new(url: &Url) -> Result<Self> {
		if url.scheme() == "data" {
			return Self::from_dataurl(url);
		}

		tracing::debug!("Downloading file from {url}");
		let file_path = temp_dir().join(url.path().split('/').last().unwrap_or_else(|| url.path()));
		let request = reqwest::blocking::get(url.as_str())?.bytes()?;

		std::io::copy(&mut request.as_ref(), &mut File::create(&file_path)?)?;
		tracing::debug!("Downloaded file to {}", file_path.display());

		Ok(Self(file_path))
	}

	/// Create a new path from a data url
	///
	/// # Errors
	///
	/// Returns an error if the url cannot be decoded or a temporary file cannot be created.
	pub(crate) fn from_dataurl(url: &Url) -> Result<Self> {
		let data = url.path().split(',').last().unwrap_or_else(|| url.path());

		let file_bytes = base64_decode(data)?;
		let mime_type = Mime::from_str(tree_magic_mini::from_u8(&file_bytes))
			.unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM);
		let file_ext = mime_guess::get_mime_extensions(&mime_type)
			.and_then(<[&str]>::last)
			.map_or_else(String::new, |e| format!(".{e}"));

		let file_path = temp_dir().join(format!("{}{file_ext}", Uuid::new_v4()));

		std::fs::write(&file_path, file_bytes)?;
		Ok(Self(file_path))
	}

	/// PUT the file to the given endpoint and return the url
	///
	/// # Errors
	///
	/// Returns an error if the file cannot be read or the upload fails.
	///
	/// # Panics
	///
	/// Panics if the file name is not valid unicode.
	pub(crate) fn upload_put(&self, upload_url: &Url) -> Result<String> {
		let url = url_join(upload_url, self.0.file_name().unwrap().to_str().unwrap());
		tracing::debug!("Uploading file to {url}");

		let file_bytes = std::fs::read(&self.0)?;
		let mime_type = tree_magic_mini::from_u8(&file_bytes);

		let response = reqwest::blocking::Client::new()
			.put(url.clone())
			.header("Content-Type", mime_type)
			.body(file_bytes)
			.send()?;

		if !response.status().is_success() {
			anyhow::bail!(
				"Failed to upload file to {url}: got {}. {}",
				response.status(),
				response.text().unwrap_or_default()
			);
		}

		let response = response.text().unwrap_or_default();

		let Ok(UploadResponse { mut url }) = serde_json::from_str::<UploadResponse>(&response)
		else {
			anyhow::bail!("Failed to parse response from {url}: got {}", response);
		};

		url.set_query(None);

		tracing::debug!("Uploaded file to {url}");
		Ok(url.to_string())
	}

	/// Convert the file to a data url
	///
	/// # Errors
	///
	/// Returns an error if the file cannot be read.
	pub(crate) fn to_dataurl(&self) -> Result<String> {
		let file_bytes = std::fs::read(&self.0)?;
		let mime_type = tree_magic_mini::from_u8(&file_bytes);

		Ok(format!(
			"data:{mime_type};base64,{base64}",
			base64 = base64_encode(&file_bytes)
		))
	}
}

impl AsRef<std::path::Path> for Path {
	fn as_ref(&self) -> &std::path::Path {
		self.0.as_ref()
	}
}

impl JsonSchema for Path {
	fn schema_name() -> String {
		"Path".to_string()
	}

	fn json_schema(gen: &mut SchemaGenerator) -> Schema {
		Url::json_schema(gen)
	}
}

impl Drop for Path {
	fn drop(&mut self) {
		tracing::debug!("Removing temporary file at path {:?}", self.0);

		std::fs::remove_file(&self.0).unwrap();
	}
}

impl<'de> serde::Deserialize<'de> for Path {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let url = String::deserialize(deserializer)?;

		Self::new(&Url::parse(&url).map_err(serde::de::Error::custom)?)
			.map_err(serde::de::Error::custom)
	}
}

impl Serialize for Path {
	fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let url = env::var("UPLOAD_URL")
			.map(|url| url.parse().ok())
			.ok()
			.flatten()
			.map_or_else(
				|| self.to_dataurl(),
				|upload_url| self.upload_put(&upload_url),
			);

		serializer.serialize_str(&url.map_err(serde::ser::Error::custom)?)
	}
}

impl From<PathBuf> for Path {
	fn from(path: PathBuf) -> Self {
		Self(path)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[derive(Debug, serde::Deserialize)]
	struct StructWithPath {
		file: Path,
	}

	#[test]
	fn test_path_deserialize() {
		let r#struct: StructWithPath = serde_json::from_value(json!({
			"file": "https://raw.githubusercontent.com/m1guelpf/cog-rust/main/README.md"
		}))
		.unwrap();

		let path = r#struct.file;
		let underlying_path = path.0.clone();

		assert!(
			underlying_path.exists(),
			"File does not exist at path {:?}",
			path.0
		);
		assert!(
			underlying_path.metadata().unwrap().len() > 0,
			"File is empty"
		);

		drop(path);

		assert!(
			!underlying_path.exists(),
			"File still exists at path {underlying_path:?}",
		);
	}

	#[test]
	fn test_dataurl_serialize() {
		let r#struct: StructWithPath = serde_json::from_value(json!({
			"file": "https://upload.wikimedia.org/wikipedia/commons/thumb/1/1b/Square_200x200.png/120px-Square_200x200.png"
		}))
		.unwrap();

		let path = r#struct.file;
		let dataurl = path.to_dataurl().unwrap();

		assert!(dataurl.starts_with("data:image/png;base64,"));
	}
}

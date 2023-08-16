use base64::{engine::general_purpose::STANDARD as Base64, DecodeError, Engine};
use url::Url;

pub mod headers;
pub mod openapi;

pub fn base64_encode<T: AsRef<[u8]>>(bytes: T) -> String {
	Base64.encode(bytes)
}

pub fn base64_decode<T: AsRef<[u8]>>(bytes: T) -> Result<Vec<u8>, DecodeError> {
	Base64.decode(bytes)
}

/// Append a path to a URL.
/// This is a workaround for the fact that `Url::join` will get rid of the last path segment if it doesn't end with a slash.
pub fn url_join(url: &Url, path: &str) -> Url {
	let mut url = url.clone();
	let mut path_parts = url.path_segments_mut().unwrap();

	path_parts.push(path);
	drop(path_parts);

	url
}

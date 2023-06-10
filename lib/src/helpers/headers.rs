use axum::{
	headers::{Error, Header},
	http::{HeaderName, HeaderValue},
};
use itertools::Itertools;
use lazy_static::lazy_static;
use percent_encoding::{percent_decode_str, percent_encode, NON_ALPHANUMERIC};
use std::{borrow::Cow, collections::HashMap};

lazy_static! {
	static ref PREFER: HeaderName = HeaderName::from_lowercase(b"prefer").unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prefer(pub HashMap<String, String>);

impl Prefer {
	pub fn has(&self, key: &str) -> bool {
		self.0.contains_key(key)
	}
}

impl Header for Prefer {
	fn name() -> &'static HeaderName {
		&PREFER
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i axum::http::HeaderValue>,
	{
		let value = values.next().ok_or_else(Error::invalid)?;

		let preferences = value
			.to_str()
			.map_err(|_| Error::invalid())?
			.split(',')
			.map(str::trim)
			.map(|s| {
				let mut split = s.splitn(2, '=');
				let (key, value) = (split.next().unwrap(), split.next().unwrap_or_default());

				(
					key.to_string(),
					percent_decode_str(value)
						.decode_utf8()
						.unwrap_or(Cow::Borrowed(value))
						.to_string(),
				)
			})
			.collect::<HashMap<_, _>>();

		Ok(Self(preferences))
	}

	fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
		let value = self
			.0
			.iter()
			.sorted()
			.map(|(key, value)| {
				format!(
					"{key}{}{}",
					if value.is_empty() { "" } else { "=" },
					percent_encode(value.as_bytes(), NON_ALPHANUMERIC)
				)
			})
			.collect::<Vec<_>>()
			.join(",");

		let value = HeaderValue::from_bytes(value.as_bytes()).unwrap();
		values.extend(std::iter::once(value));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use axum::http::HeaderMap;
	use map_macro::hash_map;

	#[test]
	fn header_is_parsed_correctly() {
		let mut headers = HeaderMap::new();
		headers.insert(
			"Prefer",
			HeaderValue::from_static("wait=10, timeout=5, respond-async"),
		);

		let prefer = Prefer::decode(&mut headers.get_all("Prefer").iter()).unwrap();

		assert_eq!(
			prefer,
			Prefer(hash_map! {
				"wait".to_string() => "10".to_string(),
				"timeout".to_string() => "5".to_string(),
				"respond-async".to_string() => String::new(),
			})
		);
	}

	#[test]
	fn header_is_encoded_correctly() {
		let prefer = Prefer(hash_map! {
			"wait".to_string() => "10".to_string(),
			"timeout".to_string() => "5".to_string(),
			"respond-async".to_string() => String::new(),
		});

		let mut values = Vec::new();
		prefer.encode(&mut values);

		assert_eq!(
			values,
			vec![HeaderValue::from_static("respond-async,timeout=5,wait=10")]
		);
	}

	#[test]
	fn has_returns_true_if_key_exists() {
		let prefer = Prefer(hash_map! {
			"wait".to_string() => "10".to_string(),
			"timeout".to_string() => "5".to_string(),
			"respond-async".to_string() => String::new(),
		});

		assert!(prefer.has("wait"));
		assert!(prefer.has("timeout"));
		assert!(prefer.has("respond-async"));
		assert!(!prefer.has("foo"));
	}
}

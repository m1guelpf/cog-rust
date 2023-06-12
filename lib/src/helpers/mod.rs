use base64::{engine::general_purpose::STANDARD as Base64, DecodeError, Engine};

pub mod headers;
pub mod openapi;

pub fn base64_encode<T: AsRef<[u8]>>(bytes: T) -> String {
	Base64.encode(bytes)
}

pub fn base64_decode<T: AsRef<[u8]>>(bytes: T) -> Result<Vec<u8>, DecodeError> {
	Base64.decode(bytes)
}

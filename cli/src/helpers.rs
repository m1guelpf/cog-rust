use std::io::{self, Read};

#[must_use]
pub fn wait_for_input() -> String {
	let mut input = String::new();
	io::stdin()
		.read_line(&mut input)
		.expect("Failed to read line");

	input.trim().to_string()
}

#[must_use]
pub fn load_from_stdin() -> String {
	let mut input = String::new();
	io::stdin()
		.read_to_string(&mut input)
		.expect("Failed to load from stdin");

	input.trim().to_string()
}

#[must_use]
pub fn is_m1_mac() -> bool {
	std::env::consts::OS == "macos" && std::env::consts::ARCH == "aarch64"
}

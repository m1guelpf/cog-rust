mod auth;
mod builder;

pub use auth::store_credentials;
pub use builder::Builder;

#[allow(clippy::module_name_repetitions)]
pub fn ensure_docker() {
	let output = std::process::Command::new("docker")
		.arg("--version")
		.output()
		.expect("Failed to run 'docker --version'");

	if !output.status.success() {
		eprintln!("Failed to run 'docker --version'");
		std::process::exit(1);
	}
}

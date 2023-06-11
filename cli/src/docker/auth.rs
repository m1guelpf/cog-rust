use base64::{engine::general_purpose::STANDARD as Base64, Engine};
use serde_json::{json, Value};
use std::{
	fs,
	io::Write,
	process::{Command, Stdio},
};

pub fn store_credentials(registry: &str, username: &str, token: &str) -> Result<(), String> {
	let docker_config_path = dirs::home_dir()
		.unwrap()
		.join(".docker")
		.join("config.json");

	if !docker_config_path.exists() {
		return Err(format!(
			"Couldn't find Docker config file at {}",
			docker_config_path.display()
		));
	}

	let mut docker_config: Value =
		serde_json::from_str(&fs::read_to_string(&docker_config_path).unwrap()).unwrap();
	let credential_store = docker_config
		.get_mut("credsStore")
		.and_then(|v| Some(v.as_str()?.to_string()));

	if let Some(credential_store) = credential_store {
		save_in_store(&credential_store, registry, username, token)?;
	} else {
		save_in_config(&mut docker_config, registry, username, token);

		fs::write(
			docker_config_path,
			serde_json::to_string_pretty(&docker_config).unwrap(),
		)
		.expect("Failed to save Docker config.");
	}

	Ok(())
}

fn save_in_store(store: &str, registry: &str, username: &str, token: &str) -> Result<(), String> {
	let binary = format!("docker-credential-{store}");

	let mut cmd = Command::new(&binary).stdin(Stdio::piped()).spawn().unwrap();

	let stdin = cmd.stdin.as_mut().unwrap();
	stdin
		.write_all(
			json!({ "ServerURL": registry, "Username": username, "Secret": token })
				.to_string()
				.as_bytes(),
		)
		.unwrap();

	let output = cmd.wait_with_output().unwrap();

	if !output.status.success() {
		return Err(format!(
			"Failed to store credentials using {}: {}",
			binary,
			String::from_utf8_lossy(&output.stderr)
		));
	}

	Ok(())
}

fn save_in_config(docker_config: &mut Value, registry: &str, username: &str, token: &str) {
	let auths = docker_config
		.get_mut("auths")
		.unwrap()
		.as_object_mut()
		.unwrap();

	auths.insert(
		registry.to_string(),
		serde_json::json!({ "auth": Base64.encode(format!("{username}:{token}")) }),
	);
}

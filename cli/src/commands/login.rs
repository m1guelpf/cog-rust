use anyhow::Result;
use reqwest::StatusCode;
use serde_json::Value;

use crate::{
	docker,
	helpers::{load_from_stdin, wait_for_input},
	Context,
};

pub async fn handle(_: Context, token_stdin: bool, registry: String) {
	let token = if token_stdin {
		load_from_stdin()
	} else {
		get_token_interactive(&registry).await
	};

	let username = verify_token(&registry, &token).await.unwrap();

	docker::store_credentials(&registry, &username, &token).unwrap();
	println!("You've successfully authenticated as {username}! You can now use the '{registry}' registry.");
}

async fn get_token_interactive(registry: &str) -> String {
	let token_url = get_display_token_url(registry).await.unwrap();

	println!("This command will authenticate Docker with Replicate's '{registry}' Docker registry. You will need a Replicate account.");
	println!("Hit enter to get started. A browser will open with an authentication token that you need to paste here.");
	let _ = wait_for_input();
	println!("If it didn't open automatically, open this URL in a web browser:\n{token_url}");
	let _ = webbrowser::open(&token_url);
	println!("Once you've signed in, copy the authentication token from that web page, paste it here, then hit enter:");

	wait_for_input()
}

async fn get_display_token_url(registry_host: &str) -> Result<String, String> {
	let resp = reqwest::get(format!(
		"{}/cog/v1/display-token-url",
		if registry_host.contains("://") {
			registry_host.to_string()
		} else {
			format!("https://{registry_host}")
		},
	))
	.await
	.unwrap();

	if matches!(resp.status(), StatusCode::NOT_FOUND) {
		return Err(format!(
			"{registry_host} is not the Replicate registry\nPlease log in using 'docker login'",
		));
	}

	if !resp.status().is_success() {
		return Err(format!(
			"{registry_host} returned HTTP status {}",
			resp.status()
		));
	}

	let body: Value = resp.json().await.unwrap();
	Ok(body
		.get("url")
		.and_then(|v| Some(v.as_str()?.to_string()))
		.unwrap())
}

async fn verify_token(registry_host: &str, token: &str) -> Result<String, String> {
	let resp = reqwest::Client::new()
		.post(format!(
			"{}/cog/v1/verify-token",
			if registry_host.contains("://") {
				registry_host.to_string()
			} else {
				format!("https://{registry_host}")
			},
		))
		.form(&[("token", token)])
		.send()
		.await
		.unwrap();

	if matches!(resp.status(), StatusCode::NOT_FOUND) {
		return Err("User does not exist".to_string());
	}

	if !resp.status().is_success() {
		return Err(format!(
			"Failed to verify token, got status {}",
			resp.status()
		));
	}

	let body: Value = resp.json().await.unwrap();
	Ok(body
		.get("username")
		.and_then(|v| Some(v.as_str()?.to_string()))
		.unwrap())
}

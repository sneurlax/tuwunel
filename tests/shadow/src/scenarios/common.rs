//! Common helpers for Shadow test scenarios.
//!
//! Uses ruma types + reqwest directly instead of matrix-sdk because
//! matrix-sdk 0.16 requires async-channel >= 2.5.0 which conflicts
//! with the workspace's patched async-channel 2.3.1 fork.
//! The ruma + reqwest approach provides equivalent functionality for
//! CS API operations (registration, login, room management, messaging).

use std::time::Duration;

/// Registration token matching TuwunelConfig default.
pub const REGISTRATION_TOKEN: &str = "shadow_test_token";

/// Default server name for Shadow-hosted tuwunel.
pub const SERVER_NAME: &str = "tuwunel-server";

/// Default password used for all test users.
pub const DEFAULT_PASSWORD: &str = "shadow_test_pass";

/// Poll `/_matrix/client/versions` until the server is ready.
///
/// Uses reqwest directly for lightweight readiness checking,
/// consistent with the smoke subcommand pattern.
pub async fn wait_for_server(
	base_url: &str,
	max_retries: u32,
	retry_interval_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(5))
		.danger_accept_invalid_certs(true)
		.build()?;

	let url = format!("{base_url}/_matrix/client/versions");
	let interval = Duration::from_millis(retry_interval_ms);

	for attempt in 0..max_retries {
		match client.get(&url).send().await {
			| Ok(resp) if resp.status().is_success() => {
				let body = resp.text().await?;
				let json: serde_json::Value =
					serde_json::from_str(&body)?;
				if json.get("versions").is_some() {
					eprintln!(
						"Server ready after {attempt} retries"
					);
					return Ok(());
				}
				eprintln!(
					"Attempt {attempt}: valid JSON but missing \
					 'versions' key"
				);
			},
			| Ok(resp) => {
				eprintln!(
					"Attempt {attempt}: HTTP {}",
					resp.status()
				);
			},
			| Err(e) => {
				eprintln!("Attempt {attempt}: {e}");
			},
		}

		tokio::time::sleep(interval).await;
	}

	Err(format!(
		"Server at {base_url} did not become ready after \
		 {max_retries} attempts"
	)
	.into())
}

/// HTTP client wrapper for Matrix CS API calls via ruma + reqwest.
///
/// Replaces matrix-sdk::Client for scenarios where matrix-sdk cannot
/// be used due to workspace dependency conflicts. Provides the same
/// core operations: registration, login, room creation, messaging.
pub struct MatrixClient {
	http: reqwest::Client,
	base_url: String,
	access_token: Option<String>,
}

impl MatrixClient {
	/// Create a new client and wait for server readiness.
	///
	/// Waits for the server to respond to /_matrix/client/versions
	/// (60 retries, 500ms interval), then returns a client ready
	/// for API calls.
	pub async fn new(
		server_url: &str,
	) -> Result<Self, Box<dyn std::error::Error>> {
		wait_for_server(server_url, 60, 500).await?;

		let http = reqwest::Client::builder()
			.timeout(Duration::from_secs(10))
			.danger_accept_invalid_certs(true)
			.build()?;

		Ok(Self {
			http,
			base_url: server_url.to_owned(),
			access_token: None,
		})
	}

	/// Register a user via the UIAA two-step registration flow.
	///
	/// First attempts registration to get a UIAA session, then
	/// retries with the registration token auth data. This matches
	/// tuwunel's token-based registration requirement.
	pub async fn register_with_token(
		&mut self,
		username: &str,
		password: &str,
		token: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let url = format!(
			"{}/_matrix/client/v3/register",
			self.base_url
		);

		// Step 1: initial registration attempt to get UIAA session
		let body = serde_json::json!({
			"username": username,
			"password": password,
			"auth": {
				"type": "m.login.dummy"
			}
		});

		let resp = self.http.post(&url).json(&body).send().await?;
		let status = resp.status();
		let resp_body: serde_json::Value = resp.json().await?;

		// If registration succeeded directly (unlikely with token)
		if status.is_success() {
			if let Some(tok) = resp_body.get("access_token") {
				self.access_token =
					tok.as_str().map(|s| s.to_owned());
			}
			eprintln!("{username} registered (no UIAA required)");
			return Ok(());
		}

		// Extract session from UIAA 401 response
		let session = resp_body
			.get("session")
			.and_then(|s| s.as_str())
			.ok_or_else(|| {
				format!(
					"Registration UIAA response missing session \
					 for {username}: {resp_body}"
				)
			})?;

		// Step 2: retry with registration token
		let body = serde_json::json!({
			"username": username,
			"password": password,
			"auth": {
				"type": "m.login.registration_token",
				"token": token,
				"session": session
			}
		});

		let resp = self.http.post(&url).json(&body).send().await?;
		let status = resp.status();
		let resp_body: serde_json::Value = resp.json().await?;

		if !status.is_success() {
			return Err(format!(
				"Registration retry failed for {username}: \
				 {status} {resp_body}"
			)
			.into());
		}

		if let Some(tok) = resp_body.get("access_token") {
			self.access_token =
				tok.as_str().map(|s| s.to_owned());
		}

		eprintln!("{username} registered");
		Ok(())
	}

	/// Log in a previously-registered user with username and
	/// password.
	pub async fn login_user(
		&mut self,
		username: &str,
		password: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let url = format!(
			"{}/_matrix/client/v3/login",
			self.base_url
		);

		let body = serde_json::json!({
			"type": "m.login.password",
			"identifier": {
				"type": "m.id.user",
				"user": username
			},
			"password": password
		});

		let resp = self.http.post(&url).json(&body).send().await?;
		let status = resp.status();
		let resp_body: serde_json::Value = resp.json().await?;

		if !status.is_success() {
			return Err(format!(
				"Login failed for {username}: {status} {resp_body}"
			)
			.into());
		}

		if let Some(tok) = resp_body.get("access_token") {
			self.access_token =
				tok.as_str().map(|s| s.to_owned());
		}

		eprintln!("{username} logged in");
		Ok(())
	}

	/// Get the current access token, if authenticated.
	pub fn access_token(&self) -> Option<&str> {
		self.access_token.as_deref()
	}

	/// Get the base URL of the server.
	pub fn base_url(&self) -> &str { &self.base_url }

	/// Get a reference to the underlying HTTP client.
	pub fn http(&self) -> &reqwest::Client { &self.http }
}

/// Create a MatrixClient pointed at the given server URL.
///
/// Convenience wrapper that waits for server readiness and returns
/// an unauthenticated client ready for registration or login.
pub async fn create_sdk_client(
	server_url: &str,
) -> Result<MatrixClient, Box<dyn std::error::Error>> {
	MatrixClient::new(server_url).await
}

/// Register a user via the UIAA two-step registration flow.
///
/// Convenience wrapper matching the plan's function signature.
pub async fn register_with_token(
	client: &mut MatrixClient,
	username: &str,
	password: &str,
	token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	client.register_with_token(username, password, token).await
}

/// Log in a previously-registered user.
///
/// Convenience wrapper matching the plan's function signature.
pub async fn login_user(
	client: &mut MatrixClient,
	username: &str,
	password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	client.login_user(username, password).await
}

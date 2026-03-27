//! Embeddable tuwunel homeserver for in-process testing.
//!
//! Provides [`EmbeddedHomeserver`] and [`Builder`] for launching one or more
//! tuwunel instances inside the caller's process, sharing the caller's tokio
//! runtime.
//!
//! # In-Memory Transport
//!
//! In-memory HTTP transport via extracted axum Router (EMBD-10) is
//! deferred to v2. For v1, connect to [`EmbeddedHomeserver::base_url`]
//! via TCP using reqwest or any HTTP client.

pub mod config;

use std::sync::Arc;

use tuwunel_core::Server;
use tuwunel_service::Services;

pub use self::config::Builder;

/// Credentials returned after registering a user via
/// [`EmbeddedHomeserver::register_user`].
pub struct RegisteredUser {
	/// Full Matrix user ID, e.g. `@alice:example.localhost`.
	pub user_id: String,

	/// Access token for authenticating subsequent requests.
	pub access_token: String,
}

/// An embedded tuwunel homeserver running in the current process.
///
/// Created via [`Builder::start`] or the convenience
/// [`EmbeddedHomeserver::start`]. The server runs as background tokio
/// tasks and is accessible at [`base_url`](Self::base_url).
pub struct EmbeddedHomeserver {
	/// Core server state.
	server: Arc<Server>,

	/// Running services; held to keep them alive.
	services: Arc<Services>,

	/// Background task running the server listener.
	run_handle: tokio::task::JoinHandle<tuwunel_core::Result>,

	/// URL with actual bound port (e.g. `http://127.0.0.1:12345`).
	base_url: String,

	/// Tempdir ownership for RocksDB; dropped on stop.
	_db_dir: Option<tempfile::TempDir>,

	/// Logging flame guard; held for lifetime.
	_flame_guard: tuwunel::logging::TracingFlameGuard,
}

impl EmbeddedHomeserver {
	/// Returns the base URL of the running server.
	pub fn base_url(&self) -> &str { &self.base_url }

	/// Convenience: start a server with default settings for the given
	/// server name.
	pub async fn start(
		server_name: &str,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		Builder::new(server_name).start().await
	}

	/// Gracefully stop the server and wait for shutdown to complete.
	///
	/// This signals shutdown via the broadcast channel, waits for the
	/// background run task to finish processing the signal, then
	/// performs final service cleanup. The RocksDB tempdir (if
	/// auto-provisioned) is deleted when `self` drops.
	pub async fn stop(
		self,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		// Signal shutdown via the broadcast channel
		self.server.shutdown()?;

		// Wait for the run task to process the shutdown signal and
		// return. This ensures admin fini and listener cleanup happen
		// before we attempt to unwrap the services Arc.
		self.run_handle.await??;

		// Final service cleanup: stops service workers and drops the
		// Arc (checking for dangling references).
		tuwunel_router::stop(self.services).await?;

		// _db_dir (TempDir) drops here, cleaning up RocksDB tempdir
		// _flame_guard drops here
		Ok(())
	}

	/// Register a local user account via the Matrix UIAA registration
	/// flow.
	///
	/// Performs the two-step UIAA dance:
	/// 1. POST `/register` with `m.login.dummy` to obtain a session
	/// 2. POST `/register` with `m.login.registration_token` and the
	///    session to complete registration
	///
	/// Returns [`RegisteredUser`] with the user ID and access token.
	pub async fn register_user(
		&self,
		username: &str,
		password: &str,
		registration_token: &str,
	) -> Result<RegisteredUser, Box<dyn std::error::Error + Send + Sync>>
	{
		let client = reqwest::Client::builder()
			.timeout(std::time::Duration::from_secs(10))
			.build()?;

		let url =
			format!("{}/_matrix/client/v3/register", self.base_url);

		// Step 1: initial registration attempt to get UIAA session
		let body = serde_json::json!({
			"username": username,
			"password": password,
			"auth": { "type": "m.login.dummy" }
		});

		let resp = client.post(&url).json(&body).send().await?;
		let status = resp.status();
		let resp_body: serde_json::Value = resp.json().await?;

		// If registration succeeded directly (no UIAA required)
		if status.is_success() {
			return Ok(extract_registered_user(&resp_body)?);
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
				"token": registration_token,
				"session": session
			}
		});

		let resp = client.post(&url).json(&body).send().await?;
		let status = resp.status();
		let resp_body: serde_json::Value = resp.json().await?;

		if !status.is_success() {
			return Err(format!(
				"Registration failed for {username}: \
				 {status} {resp_body}"
			)
			.into());
		}

		Ok(extract_registered_user(&resp_body)?)
	}
}

/// Extract user_id and access_token from a successful registration
/// response.
fn extract_registered_user(
	body: &serde_json::Value,
) -> Result<RegisteredUser, Box<dyn std::error::Error + Send + Sync>> {
	let user_id = body
		.get("user_id")
		.and_then(|v| v.as_str())
		.ok_or("Registration response missing user_id")?
		.to_owned();

	let access_token = body
		.get("access_token")
		.and_then(|v| v.as_str())
		.ok_or("Registration response missing access_token")?
		.to_owned();

	Ok(RegisteredUser {
		user_id,
		access_token,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	// Integration tests require a full tuwunel build (RocksDB, etc.).
	// Run with: cargo test -p tuwunel-embed -- --ignored

	#[tokio::test]
	#[ignore = "requires full tuwunel build with RocksDB"]
	async fn test_single_instance_lifecycle() {
		let server = EmbeddedHomeserver::start("test-single.localhost")
			.await
			.expect("server should start");

		// Verify base_url is reachable
		let resp = reqwest::get(&format!(
			"{}/_matrix/client/versions",
			server.base_url()
		))
		.await
		.expect("HTTP request should succeed");
		assert!(resp.status().is_success());

		server.stop().await.expect("server should stop cleanly");
	}

	#[tokio::test]
	#[ignore = "requires full tuwunel build with RocksDB"]
	async fn test_multi_instance_concurrent() {
		let server1 =
			EmbeddedHomeserver::start("test-multi1.localhost")
				.await
				.expect("server 1 should start");

		let server2 =
			EmbeddedHomeserver::start("test-multi2.localhost")
				.await
				.expect("server 2 should start");

		// Both should be reachable
		let resp1 = reqwest::get(&format!(
			"{}/_matrix/client/versions",
			server1.base_url()
		))
		.await
		.expect("server 1 reachable");
		assert!(resp1.status().is_success());

		let resp2 = reqwest::get(&format!(
			"{}/_matrix/client/versions",
			server2.base_url()
		))
		.await
		.expect("server 2 reachable");
		assert!(resp2.status().is_success());

		// Different ports (port 0 assigns unique ports)
		assert_ne!(server1.base_url(), server2.base_url());

		server1.stop().await.expect("server 1 stops");
		server2.stop().await.expect("server 2 stops");
	}

	#[tokio::test]
	#[ignore = "requires full tuwunel build with RocksDB"]
	async fn test_register_user() {
		let server = Builder::new("test-register.localhost")
			.registration_token("test_token_123")
			.start()
			.await
			.expect("server should start");

		let user = server
			.register_user("alice", "password123", "test_token_123")
			.await
			.expect("registration should succeed");

		assert!(!user.user_id.is_empty());
		assert!(!user.access_token.is_empty());
		assert!(user.user_id.contains("alice"));

		server.stop().await.expect("server should stop");
	}
}

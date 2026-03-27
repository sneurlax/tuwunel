pub mod config;

use std::sync::Arc;

use tuwunel_core::Server;
use tuwunel_service::Services;

pub use self::config::Builder;

/// An embedded tuwunel homeserver running in the current process.
///
/// Created via [`Builder::start`] or the convenience [`EmbeddedHomeserver::start`].
/// The server runs as background tokio tasks and is accessible at [`base_url`](Self::base_url).
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

	/// Stop the server and wait for it to shut down.
	pub async fn stop(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		self.server.shutdown()?;
		tuwunel_router::stop(self.services).await?;
		self.run_handle.await??;
		Ok(())
	}

	/// Register a local user account via the admin API.
	pub async fn register_user(
		&self,
		_username: &str,
		_password: &str,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		todo!("register_user will be implemented in plan 02")
	}
}

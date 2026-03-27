use std::{net::TcpListener, path::PathBuf, sync::Arc, time::Duration};

use figment::{Figment, providers::Serialized};
use tuwunel_core::config::Config;

use crate::EmbeddedHomeserver;

/// Builder for configuring and starting an embedded tuwunel homeserver.
///
/// Uses a fluent API to set configuration values, then calls
/// [`start`](Self::start) to launch the server.
pub struct Builder {
	server_name: String,
	port: u16,
	address: String,
	registration_token: Option<String>,
	log_level: String,
	database_path: Option<PathBuf>,
	worker_threads: usize,
}

impl Builder {
	/// Create a new builder with the given Matrix server name.
	///
	/// Defaults: port 0 (auto-assign), address 127.0.0.1, log level warn,
	/// auto tempdir for database, 2 worker threads.
	pub fn new(server_name: &str) -> Self {
		Self {
			server_name: server_name.to_owned(),
			port: 0,
			address: "127.0.0.1".to_owned(),
			registration_token: None,
			log_level: "warn".to_owned(),
			database_path: None,
			worker_threads: 2,
		}
	}

	/// Set the TCP port to listen on. Use 0 for auto-assignment.
	pub fn port(mut self, port: u16) -> Self {
		self.port = port;
		self
	}

	/// Set the listen address.
	pub fn address(mut self, address: &str) -> Self {
		self.address = address.to_owned();
		self
	}

	/// Set a registration token (enables open registration with token).
	pub fn registration_token(mut self, token: &str) -> Self {
		self.registration_token = Some(token.to_owned());
		self
	}

	/// Set the log level filter string.
	pub fn log_level(mut self, level: &str) -> Self {
		self.log_level = level.to_owned();
		self
	}

	/// Set an explicit database path. If not set, a temporary directory
	/// is created automatically.
	pub fn database_path(mut self, path: PathBuf) -> Self {
		self.database_path = Some(path);
		self
	}

	/// Set the number of tokio worker threads.
	pub fn worker_threads(mut self, n: usize) -> Self {
		self.worker_threads = n;
		self
	}

	/// Build a [`Figment`] configuration from the builder settings.
	///
	/// This constructs the config programmatically without reading
	/// environment variables or CLI arguments.
	pub fn build_figment(&self, db_path: &std::path::Path) -> Figment {
		let mut figment = Figment::new()
			.merge(Serialized::default("server_name", &self.server_name))
			.merge(Serialized::default(
				"database_path",
				db_path.to_str().unwrap_or_default(),
			))
			.merge(Serialized::default("port", [self.port]))
			.merge(Serialized::default("address", [&self.address]))
			.merge(Serialized::default("listening", true))
			.merge(Serialized::default(
				"allow_registration",
				self.registration_token.is_some(),
			))
			.merge(Serialized::default("startup_netburst", false))
			.merge(Serialized::default("log", &self.log_level))
			.merge(Serialized::default("log_global_default", false))
			.merge(Serialized::default("allow_federation", false));

		if let Some(ref token) = self.registration_token {
			figment =
				figment.merge(Serialized::default("registration_token", token));
		}

		figment
	}

	/// Start the embedded homeserver with the configured settings.
	///
	/// This will:
	/// 1. Provision a temp directory for RocksDB if no database_path was set
	/// 2. Pre-bind a port if port 0 was requested (to discover the actual
	///    port)
	/// 3. Build config via figment (no env vars or CLI args)
	/// 4. Initialize logging
	/// 5. Start the server and wait for it to become ready
	pub async fn start(
		mut self,
	) -> Result<EmbeddedHomeserver, Box<dyn std::error::Error + Send + Sync>>
	{
		// 1. Database path: use provided or create tempdir
		let db_dir = if self.database_path.is_none() {
			Some(tempfile::TempDir::new()?)
		} else {
			None
		};

		let db_path = match self.database_path {
			| Some(ref p) => p.clone(),
			| None => db_dir.as_ref().expect("tempdir created").path().to_owned(),
		};

		// 2. Port 0 pre-bind: discover actual port
		if self.port == 0 {
			let listener =
				TcpListener::bind(format!("{}:0", self.address))?;
			let actual_port = listener.local_addr()?.port();
			self.port = actual_port;
			drop(listener);
		}

		// 3. Build figment config
		let figment = self.build_figment(&db_path);
		let config = Config::new(&figment)?;

		// 4. Initialize logging
		let (flame_guard, logger) = tuwunel::logging::init(&config)?;

		// 5. Create server
		let server = Arc::new(tuwunel_core::Server::new(
			config,
			Some(&tokio::runtime::Handle::current()),
			logger,
		));

		// 6. Start services
		let services = tuwunel_router::start(&server).await?;

		// 7. Spawn background run task
		let run_services = services.clone();
		let run_handle =
			tokio::spawn(async move { tuwunel_router::run(&run_services).await });

		// 8. Build base URL
		let base_url = format!("http://{}:{}", self.address, self.port);

		// 9. Wait for readiness
		wait_for_ready(&base_url).await?;

		Ok(EmbeddedHomeserver {
			server,
			services,
			run_handle,
			base_url,
			_db_dir: db_dir,
			_flame_guard: flame_guard,
		})
	}
}

/// Poll the server's versions endpoint until it responds successfully.
async fn wait_for_ready(
	base_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let url = format!("{base_url}/_matrix/client/versions");
	let client = reqwest::Client::new();

	for _ in 0..60 {
		match client.get(&url).send().await {
			| Ok(resp) if resp.status().is_success() => return Ok(()),
			| _ => tokio::time::sleep(Duration::from_millis(200)).await,
		}
	}

	Err(format!(
		"Server at {base_url} did not become ready within 12 seconds"
	)
	.into())
}

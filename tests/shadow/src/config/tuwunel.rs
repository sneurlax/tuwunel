use serde::Serialize;

/// Minimal tuwunel config for Shadow testing.
/// Per SHAD-04: constructed programmatically, no on-disk TOML template.
/// Per Pitfall 3: address must be IPv4 (Shadow has no IPv6).
/// Per Pitfall 6/CONF-01: port 8448 hardcoded (Shadow virtual IPs avoid
/// conflicts).
#[derive(Serialize, Clone, Debug)]
pub struct TuwunelConfig {
	pub global: TuwunelGlobal,
}

#[derive(Serialize, Clone, Debug)]
pub struct TuwunelGlobal {
	pub server_name: String,
	pub database_path: String,
	pub address: String,
	pub port: u16,
	pub allow_registration: bool,
	pub registration_token: String,
	pub log: String,
	/// Disable federation startup burst -- no external servers in Shadow.
	pub startup_netburst: bool,
	/// Allow E2EE encryption (default true for E2EE test scenarios).
	pub allow_encryption: bool,
}

impl TuwunelConfig {
	/// Create a config for a Shadow-hosted tuwunel instance.
	///
	/// - `server_name`: Matrix server name (typically the Shadow hostname)
	/// - `database_path`: Relative path for RocksDB (relative to Shadow host
	///   CWD). Per CONF-03: each instance gets isolated tempdir. Per Pitfall
	///   4: relative paths resolve from shadow.data/hosts/<hostname>/
	pub fn new(server_name: &str, database_path: &str) -> Self {
		Self {
			global: TuwunelGlobal {
				server_name: server_name.to_owned(),
				database_path: database_path.to_owned(),
				address: "0.0.0.0".to_owned(),
				port: 8448,
				allow_registration: true,
				registration_token: "shadow_test_token".to_owned(),
				log: "info".to_owned(),
				startup_netburst: false,
			allow_encryption: true,
			},
		}
	}

	/// Serialize to TOML string for writing to a config file.
	/// Per CONF-02: config generated as tempfile TOML for Shadow process
	/// args.
	pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
		toml::to_string(self)
	}
}

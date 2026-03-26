use std::{collections::BTreeMap, path::Path};

use serde::Serialize;

/// Top-level Shadow simulation config.
/// Serializes to Shadow's YAML format via serde_yaml.
#[derive(Serialize, Clone, Debug)]
pub struct ShadowConfig {
	pub general: General,
	pub network: Network,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub host_option_defaults: Option<HostOptionDefaults>,
	pub hosts: BTreeMap<String, Host>,
}

#[derive(Serialize, Clone, Debug)]
pub struct General {
	pub stop_time: String,
	pub seed: u32,
	pub model_unblocked_syscall_latency: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data_directory: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub log_level: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Network {
	pub graph: NetworkGraph,
}

#[derive(Serialize, Clone, Debug)]
pub struct NetworkGraph {
	#[serde(rename = "type")]
	pub graph_type: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub inline: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct HostOptionDefaults {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pcap_enabled: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pcap_capture_size: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Host {
	pub network_node_id: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub host_options: Option<HostOptions>,
	pub processes: Vec<Process>,
}

#[derive(Serialize, Clone, Debug)]
pub struct HostOptions {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pcap_enabled: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pcap_capture_size: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Process {
	pub path: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub args: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub start_time: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expected_final_state: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub environment: Option<BTreeMap<String, String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub shutdown_time: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub shutdown_signal: Option<String>,
}

impl ShadowConfig {
	/// Serialize to Shadow YAML format.
	pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
		serde_yaml::to_string(self)
	}
}

/// Create a default General config for deterministic testing.
/// Per SHAD-07: explicit seed and stop_time required.
/// Per research Pitfall 1: model_unblocked_syscall_latency must be true.
impl Default for General {
	fn default() -> Self {
		Self {
			stop_time: "30s".to_owned(),
			seed: 42,
			model_unblocked_syscall_latency: true,
			data_directory: None,
			log_level: Some("info".to_owned()),
		}
	}
}

/// Create a default 1 Gbit switch network.
impl Default for Network {
	fn default() -> Self {
		Self {
			graph: NetworkGraph {
				graph_type: "1_gbit_switch".to_owned(),
				inline: None,
			},
		}
	}
}

/// Create default host options with PCAP enabled.
/// Per SHAD-08: pcap capture available per host.
impl Default for HostOptionDefaults {
	fn default() -> Self {
		Self {
			pcap_enabled: Some(true),
			pcap_capture_size: None,
		}
	}
}

/// Build a Shadow config with tuwunel-server + alice-host + bob-host.
///
/// Per D-07: separate Shadow hosts with own virtual IPs.
/// Per D-08: deterministic naming conventions.
///
/// # Arguments
///
/// * `tuwunel_bin` - Path to the tuwunel server binary
/// * `client_bin` - Path to the matrix-test-client binary
/// * `config_path` - Path to the tuwunel TOML config file
/// * `data_dir` - Shadow data directory path
/// * `subcommand` - Client subcommand to run (e.g., "cs-api")
/// * `stop_time` - Shadow simulation stop time (e.g., "120s")
/// * `seed` - Deterministic RNG seed
/// * `alice_start` - Start time for alice's client process
/// * `bob_start` - Start time for bob's client process
#[expect(clippy::too_many_arguments)]
pub fn three_host_config(
	tuwunel_bin: &Path,
	client_bin: &Path,
	config_path: &Path,
	data_dir: &Path,
	subcommand: &str,
	stop_time: &str,
	seed: u32,
	alice_start: &str,
	bob_start: &str,
) -> ShadowConfig {
	let tuwunel_path = tuwunel_bin
		.to_str()
		.expect("tuwunel_bin path must be valid UTF-8")
		.to_owned();
	let client_path = client_bin
		.to_str()
		.expect("client_bin path must be valid UTF-8")
		.to_owned();
	let config_str = config_path
		.to_str()
		.expect("config_path must be valid UTF-8")
		.to_owned();
	let data_str = data_dir
		.to_str()
		.expect("data_dir path must be valid UTF-8")
		.to_owned();

	let mut server_env = BTreeMap::new();
	server_env
		.insert("TUWUNEL_CONFIG".to_owned(), config_str);
	server_env
		.insert("TUWUNEL_LOG".to_owned(), "info".to_owned());

	let server_host = Host {
		network_node_id: 0,
		host_options: None,
		processes: vec![Process {
			path: tuwunel_path,
			args: None,
			start_time: Some("1s".to_owned()),
			expected_final_state: Some("running".to_owned()),
			environment: Some(server_env),
			shutdown_time: None,
			shutdown_signal: Some("SIGTERM".to_owned()),
		}],
	};

	let alice_host = Host {
		network_node_id: 0,
		host_options: None,
		processes: vec![Process {
			path: client_path.clone(),
			args: Some(format!(
				"{subcommand} --server-url \
				 http://tuwunel-server:8448 --role alice"
			)),
			start_time: Some(alice_start.to_owned()),
			expected_final_state: Some("exited".to_owned()),
			environment: None,
			shutdown_time: None,
			shutdown_signal: None,
		}],
	};

	let bob_host = Host {
		network_node_id: 0,
		host_options: None,
		processes: vec![Process {
			path: client_path,
			args: Some(format!(
				"{subcommand} --server-url \
				 http://tuwunel-server:8448 --role bob"
			)),
			start_time: Some(bob_start.to_owned()),
			expected_final_state: Some("exited".to_owned()),
			environment: None,
			shutdown_time: None,
			shutdown_signal: None,
		}],
	};

	let mut hosts = BTreeMap::new();
	hosts.insert("tuwunel-server".to_owned(), server_host);
	hosts.insert("alice-host".to_owned(), alice_host);
	hosts.insert("bob-host".to_owned(), bob_host);

	ShadowConfig {
		general: General {
			stop_time: stop_time.to_owned(),
			seed,
			model_unblocked_syscall_latency: true,
			data_directory: Some(data_str),
			log_level: Some("info".to_owned()),
		},
		network: Network::default(),
		host_option_defaults: Some(HostOptionDefaults::default()),
		hosts,
	}
}

/// Named network topology fixture for Shadow simulations.
/// Per D-01: builder functions, type-safe, composable.
/// Per D-02: sensible defaults with optional overrides.
#[derive(Clone, Debug)]
pub struct TopologyFixture {
	/// One-way latency in milliseconds (RTT = 2x this value for
	/// self-loop topology).
	pub latency_ms: u32,
	/// Packet loss rate as a fraction (0.0 to 1.0).
	pub packet_loss: f64,
	/// Download bandwidth (e.g., "5 Mbit").
	pub bandwidth_down: String,
	/// Upload bandwidth (e.g., "1 Mbit").
	pub bandwidth_up: String,
}

impl TopologyFixture {
	/// Slow mobile network: 150ms one-way latency (300ms RTT),
	/// 1% packet loss, 5 Mbit down / 1 Mbit up.
	pub fn slow_mobile() -> Self {
		Self {
			latency_ms: 150,
			packet_loss: 0.01,
			bandwidth_down: "5 Mbit".to_owned(),
			bandwidth_up: "1 Mbit".to_owned(),
		}
	}

	/// High latency link: 500ms one-way (1000ms RTT),
	/// no packet loss, 100 Mbit symmetric.
	pub fn high_latency() -> Self {
		Self {
			latency_ms: 500,
			packet_loss: 0.0,
			bandwidth_down: "100 Mbit".to_owned(),
			bandwidth_up: "100 Mbit".to_owned(),
		}
	}

	/// Lossy link: 50ms one-way (100ms RTT),
	/// 5% packet loss, 10 Mbit symmetric.
	pub fn lossy_link() -> Self {
		Self {
			latency_ms: 50,
			packet_loss: 0.05,
			bandwidth_down: "10 Mbit".to_owned(),
			bandwidth_up: "10 Mbit".to_owned(),
		}
	}

	/// Override the one-way latency (ms).
	pub fn with_latency(mut self, ms: u32) -> Self {
		self.latency_ms = ms;
		self
	}

	/// Override the packet loss rate (0.0 to 1.0).
	pub fn with_loss(mut self, loss: f64) -> Self {
		self.packet_loss = loss;
		self
	}

	/// Override the download bandwidth (e.g., "10 Mbit").
	pub fn with_bandwidth_down(mut self, bw: &str) -> Self {
		self.bandwidth_down = bw.to_owned();
		self
	}

	/// Override the upload bandwidth (e.g., "5 Mbit").
	pub fn with_bandwidth_up(mut self, bw: &str) -> Self {
		self.bandwidth_up = bw.to_owned();
		self
	}

	/// Build the GML graph string for this topology.
	/// Per Pitfall 6: packet_loss is always included even when 0.0.
	pub fn to_gml(&self) -> String {
		format!(
			"graph [\n\
			 \x20 directed 0\n\
			 \x20 node [\n\
			 \x20   id 0\n\
			 \x20   host_bandwidth_down \"{bd}\"\n\
			 \x20   host_bandwidth_up \"{bu}\"\n\
			 \x20 ]\n\
			 \x20 edge [\n\
			 \x20   source 0\n\
			 \x20   target 0\n\
			 \x20   latency \"{lat} ms\"\n\
			 \x20   packet_loss {loss}\n\
			 \x20 ]\n\
			 ]",
			bd = self.bandwidth_down,
			bu = self.bandwidth_up,
			lat = self.latency_ms,
			loss = self.packet_loss,
		)
	}

	/// Build a complete Network struct from this topology.
	pub fn to_network(&self) -> Network {
		Network {
			graph: NetworkGraph {
				graph_type: "gml".to_owned(),
				inline: Some(self.to_gml()),
			},
		}
	}
}

/// Build a Shadow config with tuwunel-server + alice-host + bob-host
/// using a custom network topology.
///
/// Same as [`three_host_config`] but replaces the default 1 Gbit
/// switch with the given [`TopologyFixture`].
#[expect(clippy::too_many_arguments)]
pub fn three_host_config_with_topology(
	tuwunel_bin: &Path,
	client_bin: &Path,
	config_path: &Path,
	data_dir: &Path,
	subcommand: &str,
	stop_time: &str,
	seed: u32,
	alice_start: &str,
	bob_start: &str,
	topology: &TopologyFixture,
) -> ShadowConfig {
	let mut config = three_host_config(
		tuwunel_bin,
		client_bin,
		config_path,
		data_dir,
		subcommand,
		stop_time,
		seed,
		alice_start,
		bob_start,
	);
	config.network = topology.to_network();
	config
}

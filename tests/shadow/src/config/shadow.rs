use std::collections::BTreeMap;

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

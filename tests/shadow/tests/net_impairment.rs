use shadow_test_harness::{
	config::{
		shadow::{three_host_config_with_topology, TopologyFixture},
		tuwunel::TuwunelConfig,
	},
	runner::run_shadow,
};

mod common;

#[test]
#[ignore] // Run explicitly: cargo test -p shadow-test-harness --test net_impairment -- --ignored
fn shadow_e2ee_under_impairment() {
	let (tuwunel_bin, client_bin) =
		common::build_shadow_binaries();

	let tmp =
		tempfile::tempdir().expect("failed to create tempdir");

	let tuwunel_config =
		TuwunelConfig::new("tuwunel-server", "data");
	let config_toml = tuwunel_config
		.to_toml()
		.expect("failed to serialize tuwunel config");
	let config_path = tmp.path().join("tuwunel.toml");
	std::fs::write(&config_path, &config_toml)
		.expect("failed to write tuwunel config");

	// NET-05: 200ms RTT = 100ms one-way latency, 2% packet loss.
	// Per Pitfall 1: Shadow edge latency is one-way, so for 200ms
	// RTT we set 100ms. Use slow_mobile as base, override to
	// exact NET-05 values.
	let topology = TopologyFixture::slow_mobile()
		.with_latency(100) // 100ms one-way = 200ms RTT
		.with_loss(0.02); // 2% packet loss

	// stop_time 180s per RESEARCH.md estimate for E2EE under
	// impairment.
	let data_dir = tmp.path().join("shadow.data");
	let shadow_config = three_host_config_with_topology(
		&tuwunel_bin,
		&client_bin,
		&config_path,
		&data_dir,
		"e2ee-messaging",
		"180s",
		42,
		"5s",
		"15s",
		&topology,
	);

	let shadow_yaml = shadow_config
		.to_yaml()
		.expect("failed to serialize shadow config");
	let shadow_yaml_path = tmp.path().join("shadow.yaml");
	std::fs::write(&shadow_yaml_path, &shadow_yaml)
		.expect("failed to write shadow config");

	let result = run_shadow(
		&shadow_yaml_path,
		42,
		Some(&data_dir),
		None,
	);

	// D-09: binary pass/fail
	assert!(
		result.success(),
		"Shadow E2EE-under-impairment test failed with exit \
		 code {}.\nSeed: {}\nData dir: {}\nShadow \
		 stdout:\n{}\nShadow stderr:\n{}",
		result.exit_code,
		result.seed,
		result.data_dir.display(),
		result.stdout,
		result.stderr,
	);

	// Verify alice completed E2EE scenario
	let alice_stderrs = result.find_host_stderrs("alice-host");
	assert!(
		!alice_stderrs.is_empty(),
		"No stderr files found for alice-host"
	);
	let alice_stderr = std::fs::read_to_string(&alice_stderrs[0])
		.expect("failed to read alice stderr");
	assert!(
		alice_stderr
			.contains("alice: e2ee-messaging scenario complete"),
		"Alice did not complete E2EE scenario under \
		 impairment.\nAlice stderr:\n{alice_stderr}"
	);

	// Verify bob completed E2EE scenario
	let bob_stderrs = result.find_host_stderrs("bob-host");
	assert!(
		!bob_stderrs.is_empty(),
		"No stderr files found for bob-host"
	);
	let bob_stderr = std::fs::read_to_string(&bob_stderrs[0])
		.expect("failed to read bob stderr");
	assert!(
		bob_stderr
			.contains("bob: e2ee-messaging scenario complete"),
		"Bob did not complete E2EE scenario under \
		 impairment.\nBob stderr:\n{bob_stderr}"
	);

	// Verify encrypted message was received (E2EE-03 under
	// impairment)
	assert!(
		bob_stderr.contains("encrypted secret from alice"),
		"Bob did not receive encrypted message under \
		 impairment.\nBob stderr:\n{bob_stderr}"
	);

	eprintln!(
		"Shadow E2EE under impairment test PASSED (seed={}, \
		 topology: 100ms latency, 2% loss)",
		result.seed,
	);
}

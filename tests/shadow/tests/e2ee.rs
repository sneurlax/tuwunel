use std::{path::PathBuf, process::Command};

use shadow_test_harness::{
	config::{
		shadow::three_host_config,
		tuwunel::TuwunelConfig,
	},
	runner::run_shadow,
};

/// Build tuwunel and matrix-test-client with the shadow profile.
/// This is slow (release build) but ensures binaries are available.
fn build_shadow_binaries() -> (PathBuf, PathBuf) {
	let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../..")
		.canonicalize()
		.expect("failed to find workspace root");

	// Build tuwunel with shadow profile
	let status = Command::new("cargo")
		.current_dir(&workspace_root)
		.args([
			"build",
			"--profile",
			"shadow",
			"--no-default-features",
			"--features",
			"shadow_features",
		])
		.status()
		.expect("failed to run cargo build for tuwunel");
	assert!(
		status.success(),
		"Failed to build tuwunel with shadow profile"
	);

	// Build matrix-test-client with shadow profile
	let status = Command::new("cargo")
		.current_dir(&workspace_root)
		.args([
			"build",
			"--profile",
			"shadow",
			"-p",
			"shadow-test-harness",
			"--bin",
			"matrix-test-client",
		])
		.status()
		.expect(
			"failed to run cargo build for matrix-test-client",
		);
	assert!(
		status.success(),
		"Failed to build matrix-test-client with shadow profile"
	);

	let tuwunel_bin = workspace_root.join("target/shadow/tuwunel");
	let client_bin =
		workspace_root.join("target/shadow/matrix-test-client");

	assert!(
		tuwunel_bin.exists(),
		"tuwunel binary not found at {}",
		tuwunel_bin.display()
	);
	assert!(
		client_bin.exists(),
		"matrix-test-client not found at {}",
		client_bin.display()
	);

	(tuwunel_bin, client_bin)
}

#[test]
#[ignore] // Run explicitly: cargo test -p shadow-test-harness --test e2ee -- --ignored
fn shadow_e2ee_messaging() {
	let (tuwunel_bin, client_bin) = build_shadow_binaries();

	// Create temp directory for this test run
	let tmp =
		tempfile::tempdir().expect("failed to create tempdir");

	// Generate tuwunel config with encryption enabled
	let tuwunel_config =
		TuwunelConfig::new("tuwunel-server", "data");
	let config_toml = tuwunel_config
		.to_toml()
		.expect("failed to serialize tuwunel config");
	let config_path = tmp.path().join("tuwunel.toml");
	std::fs::write(&config_path, &config_toml)
		.expect("failed to write tuwunel config");

	// Build Shadow config for E2EE 3-host simulation.
	// stop_time "120s": E2EE needs more time for key exchange
	// round trips. alice at "5s", bob at "15s": bob needs alice
	// to create the encrypted room first. seed 42: deterministic.
	let data_dir = tmp.path().join("shadow.data");
	let shadow_config = three_host_config(
		&tuwunel_bin,
		&client_bin,
		&config_path,
		&data_dir,
		"e2ee-messaging",
		"120s",
		42,
		"5s",
		"15s",
	);

	let shadow_yaml = shadow_config
		.to_yaml()
		.expect("failed to serialize shadow config");
	let shadow_yaml_path = tmp.path().join("shadow.yaml");
	std::fs::write(&shadow_yaml_path, &shadow_yaml)
		.expect("failed to write shadow config");

	// Run Shadow simulation
	let result = run_shadow(
		&shadow_yaml_path,
		42,
		Some(&data_dir),
		None,
	);

	// Assert success with diagnostics on failure
	assert!(
		result.success(),
		"Shadow E2EE test failed with exit code {}.\n\
		 Seed: {}\n\
		 Data dir: {}\n\
		 Shadow stdout:\n{}\n\
		 Shadow stderr:\n{}",
		result.exit_code,
		result.seed,
		result.data_dir.display(),
		result.stdout,
		result.stderr,
	);

	// Read alice stderr
	let alice_stderrs = result.find_host_stderrs("alice-host");
	assert!(
		!alice_stderrs.is_empty(),
		"No stderr files found for alice-host"
	);
	let alice_stderr = std::fs::read_to_string(&alice_stderrs[0])
		.expect("failed to read alice stderr");

	// Read bob stderr
	let bob_stderrs = result.find_host_stderrs("bob-host");
	assert!(
		!bob_stderrs.is_empty(),
		"No stderr files found for bob-host"
	);
	let bob_stderr = std::fs::read_to_string(&bob_stderrs[0])
		.expect("failed to read bob stderr");

	// Assert alice completed the full E2EE flow
	assert!(
		alice_stderr
			.contains("alice: e2ee-messaging scenario complete"),
		"Alice did not complete E2EE scenario.\nAlice \
		 stderr:\n{alice_stderr}"
	);

	// Assert bob completed the full E2EE flow
	assert!(
		bob_stderr
			.contains("bob: e2ee-messaging scenario complete"),
		"Bob did not complete E2EE scenario.\nBob \
		 stderr:\n{bob_stderr}"
	);

	// Assert bob received the encrypted message (E2EE-03)
	assert!(
		bob_stderr.contains("encrypted secret from alice"),
		"Bob did not receive encrypted message.\nBob \
		 stderr:\n{bob_stderr}"
	);

	// Assert alice uploaded device keys (E2EE-01)
	assert!(
		alice_stderr.contains("device keys uploaded"),
		"Alice did not upload device keys.\nAlice \
		 stderr:\n{alice_stderr}"
	);

	// Assert bob uploaded device keys (E2EE-01)
	assert!(
		bob_stderr.contains("device keys uploaded"),
		"Bob did not upload device keys.\nBob \
		 stderr:\n{bob_stderr}"
	);

	// Assert bob completed key claim (E2EE-02)
	assert!(
		bob_stderr.contains("key claim completed"),
		"Bob did not complete key claim.\nBob \
		 stderr:\n{bob_stderr}"
	);

	eprintln!(
		"Shadow E2EE messaging test PASSED (seed={})",
		result.seed,
	);
}

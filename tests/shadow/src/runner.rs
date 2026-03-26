use std::{
	path::{Path, PathBuf},
	process::Command,
};

/// Result of a Shadow simulation run.
#[derive(Debug)]
pub struct ShadowResult {
	/// Shadow process exit code (0 = success).
	pub exit_code: i32,
	/// Path to Shadow's output data directory.
	pub data_dir: PathBuf,
	/// Deterministic seed used for this run.
	pub seed: u32,
	/// Shadow's stdout.
	pub stdout: String,
	/// Shadow's stderr.
	pub stderr: String,
}

impl ShadowResult {
	/// Check if the simulation completed successfully.
	pub fn success(&self) -> bool { self.exit_code == 0 }

	/// Get the hosts directory containing per-host output.
	/// Per SHAD-06: Shadow writes to data_dir/hosts/<hostname>/
	pub fn hosts_dir(&self) -> PathBuf { self.data_dir.join("hosts") }

	/// Get stdout file for a specific host and process.
	/// Shadow names files as <procname>.<pid>.stdout
	/// Per Pitfall 7: PID starts at 1000 for first process on each
	/// host.
	pub fn host_stdout(
		&self,
		hostname: &str,
		process_name: &str,
		pid: u32,
	) -> PathBuf {
		self.data_dir
			.join("hosts")
			.join(hostname)
			.join(format!("{process_name}.{pid}.stdout"))
	}

	/// Get stderr file for a specific host and process.
	pub fn host_stderr(
		&self,
		hostname: &str,
		process_name: &str,
		pid: u32,
	) -> PathBuf {
		self.data_dir
			.join("hosts")
			.join(hostname)
			.join(format!("{process_name}.{pid}.stderr"))
	}

	/// Find all stdout files for a hostname by globbing.
	/// Use when PID is unknown.
	pub fn find_host_stdouts(&self, hostname: &str) -> Vec<PathBuf> {
		let host_dir = self.data_dir.join("hosts").join(hostname);
		Self::glob_files(&host_dir, "stdout")
	}

	/// Find all stderr files for a hostname by globbing.
	pub fn find_host_stderrs(&self, hostname: &str) -> Vec<PathBuf> {
		let host_dir = self.data_dir.join("hosts").join(hostname);
		Self::glob_files(&host_dir, "stderr")
	}

	/// Get PCAP file for a host (per SHAD-08).
	/// Shadow writes to data_dir/hosts/<hostname>/eth0.pcap
	pub fn host_pcap(&self, hostname: &str) -> PathBuf {
		self.data_dir
			.join("hosts")
			.join(hostname)
			.join("eth0.pcap")
	}

	fn glob_files(dir: &Path, extension: &str) -> Vec<PathBuf> {
		let Ok(entries) = std::fs::read_dir(dir) else {
			return Vec::new();
		};
		let mut files: Vec<PathBuf> = entries
			.filter_map(|e| e.ok())
			.map(|e| e.path())
			.filter(|p| {
				p.extension().is_some_and(|ext| ext == extension)
			})
			.collect();
		files.sort();
		files
	}

	/// Print failure diagnostics to stderr.
	/// Per SHAD-09: on failure, print seed and log paths.
	pub fn print_failure_diagnostics(&self) {
		eprintln!("=== Shadow simulation FAILED ===");
		eprintln!("Seed: {}", self.seed);
		eprintln!(
			"Data directory: {}",
			self.data_dir.display()
		);
		eprintln!(
			"Host logs: {}/",
			self.hosts_dir().display()
		);
		if !self.stderr.is_empty() {
			eprintln!("--- Shadow stderr ---");
			// Print last 50 lines to avoid overwhelming output
			let lines: Vec<&str> =
				self.stderr.lines().collect();
			let start = lines.len().saturating_sub(50);
			for line in &lines[start..] {
				eprintln!("{line}");
			}
		}
	}
}

/// Run a Shadow simulation with the given config file and seed.
///
/// The `shadow_bin` parameter allows overriding the shadow binary path
/// (defaults to "shadow" which assumes it is on PATH or at
/// ~/.local/bin/shadow).
///
/// `data_dir` is where Shadow writes its output. If None, defaults to
/// `<config_dir>/shadow.data`.
pub fn run_shadow(
	config_path: &Path,
	seed: u32,
	data_dir: Option<&Path>,
	shadow_bin: Option<&str>,
) -> ShadowResult {
	let shadow = shadow_bin.unwrap_or("shadow");
	let default_data_dir = config_path
		.parent()
		.unwrap_or(Path::new("."))
		.join("shadow.data");
	let data_dir = data_dir
		.map(Path::to_path_buf)
		.unwrap_or(default_data_dir);

	// Clean previous data dir if it exists (Shadow does not
	// overwrite)
	if data_dir.exists() {
		let _ = std::fs::remove_dir_all(&data_dir);
	}

	let mut cmd = Command::new(shadow);
	cmd.arg("--seed")
		.arg(seed.to_string())
		.arg("--data-directory")
		.arg(&data_dir)
		.arg(config_path);

	let output = cmd.output().unwrap_or_else(|e| {
		panic!(
			"Failed to execute shadow binary '{shadow}': {e}. \
			 Is Shadow installed? Check ~/.local/bin/shadow",
		);
	});

	let result = ShadowResult {
		exit_code: output.status.code().unwrap_or(-1),
		data_dir,
		seed,
		stdout: String::from_utf8_lossy(&output.stdout)
			.into_owned(),
		stderr: String::from_utf8_lossy(&output.stderr)
			.into_owned(),
	};

	// Per SHAD-09: automatically print diagnostics on failure
	if !result.success() {
		result.print_failure_diagnostics();
	}

	result
}

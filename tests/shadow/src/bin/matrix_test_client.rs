use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "matrix-test-client")]
#[command(about = "Test client for Shadow simulation of tuwunel")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Smoke test: verify server responds to
	/// /_matrix/client/versions
	Smoke {
		/// Base URL of the tuwunel server (e.g.,
		/// http://tuwunel-server:8448)
		#[arg(long)]
		server_url: String,

		/// Maximum number of retry attempts for server readiness
		#[arg(long, default_value_t = 60)]
		max_retries: u32,

		/// Milliseconds between retry attempts
		#[arg(long, default_value_t = 500)]
		retry_interval_ms: u64,
	},
}

fn main() -> ExitCode {
	let cli = Cli::parse();

	match cli.command {
		| Commands::Smoke {
			server_url,
			max_retries,
			retry_interval_ms,
		} => {
			let rt = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.expect("failed to build tokio runtime");

			match rt.block_on(run_smoke(
				&server_url,
				max_retries,
				retry_interval_ms,
			)) {
				| Ok(()) => ExitCode::SUCCESS,
				| Err(e) => {
					eprintln!("Smoke test failed: {e}");
					ExitCode::FAILURE
				},
			}
		},
	}
}

async fn run_smoke(
	base_url: &str,
	max_retries: u32,
	retry_interval_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
	let client = reqwest::Client::builder()
		.timeout(std::time::Duration::from_secs(5))
		.danger_accept_invalid_certs(true)
		.build()?;

	let url = format!("{base_url}/_matrix/client/versions");
	let interval = std::time::Duration::from_millis(retry_interval_ms);

	// Per D-04: poll /_matrix/client/versions in retry loop with
	// backoff. Per research Pitfall 1: tokio::time::sleep advances
	// Shadow's simulated time.
	for attempt in 0..max_retries {
		match client.get(&url).send().await {
			| Ok(resp) if resp.status().is_success() => {
				let body = resp.text().await?;
				// Validate response is valid JSON with "versions"
				// key
				let json: serde_json::Value =
					serde_json::from_str(&body)?;
				if json.get("versions").is_some() {
					eprintln!(
						"Server ready after {attempt} retries. \
						 Versions: {body}"
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

//! Load test scenario for Shadow simulation.
//!
//! Tests tuwunel under concurrent client load. Each client process
//! runs a minimal flow: register, login, join shared room, send
//! message.
//! Validates: LOAD-01, LOAD-02, LOAD-03.

use super::common::{
	MatrixClient, DEFAULT_PASSWORD, REGISTRATION_TOKEN, SERVER_NAME,
};

/// Entry point for the load-test subcommand.
///
/// Dispatches to creator or joiner role based on arguments.
pub async fn run_load_test(
	server_url: &str,
	role: &str,
	client_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let username = format!("loaduser-{client_id}");
	match role {
		| "creator" => run_creator(server_url, &username).await,
		| "joiner" => run_joiner(server_url, &username).await,
		| other => Err(format!(
			"unknown role: {other} (expected creator or joiner)"
		)
		.into()),
	}
}

/// Creator flow: register, login, create room with alias, send
/// message.
///
/// LOAD-01: First client creates the shared room.
async fn run_creator(
	server_url: &str,
	username: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	eprintln!("{username}: starting load-test creator flow");

	let mut client = MatrixClient::new(server_url).await?;

	client
		.register_with_token(
			username,
			DEFAULT_PASSWORD,
			REGISTRATION_TOKEN,
		)
		.await?;

	client.login_user(username, DEFAULT_PASSWORD).await?;

	let room_id =
		client.create_room(Some("load-test")).await?;
	eprintln!(
		"{username}: created room {room_id} \
		 with alias #load-test:{SERVER_NAME}"
	);

	client
		.send_text_message(
			&room_id,
			&format!("load-test message from {username}"),
		)
		.await?;

	eprintln!(
		"{username}: load-test creator scenario complete"
	);
	Ok(())
}

/// Joiner flow: register, login, join room by alias, send message.
///
/// LOAD-02: Each joiner client joins the shared room and sends a
/// message. LOAD-03: Server must remain responsive under load.
async fn run_joiner(
	server_url: &str,
	username: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	eprintln!("{username}: starting load-test joiner flow");

	let mut client = MatrixClient::new(server_url).await?;

	client
		.register_with_token(
			username,
			DEFAULT_PASSWORD,
			REGISTRATION_TOKEN,
		)
		.await?;

	client.login_user(username, DEFAULT_PASSWORD).await?;

	let room_alias =
		format!("#load-test:{SERVER_NAME}");
	let room_id = client
		.join_room_with_retry(&room_alias, 60, 1000)
		.await?;
	eprintln!("{username}: joined room {room_id}");

	client
		.send_text_message(
			&room_id,
			&format!("load-test message from {username}"),
		)
		.await?;

	eprintln!(
		"{username}: load-test joiner scenario complete"
	);
	Ok(())
}

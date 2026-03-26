//! SAS verification scenario for Shadow simulation.
//!
//! Tests tuwunel's server-side to-device message routing for SAS
//! (Short Authentication String) key verification between two clients.
//! Since matrix-sdk is unavailable due to workspace dependency conflicts,
//! this scenario exercises the verification protocol flow via raw CS API
//! to-device messaging endpoints.
//!
//! Validates: E2EE-05 (SAS verification message routing under Shadow).
//! The server's responsibility is to correctly route each to-device
//! message between Alice and Bob. The actual cryptographic operations
//! (HKDF, commitment hashes, MAC computation) belong in matrix-sdk tests.

use std::time::Duration;

use super::common::{
	MatrixClient, DEFAULT_PASSWORD, REGISTRATION_TOKEN, SERVER_NAME,
};

/// Entry point for the sas-verify subcommand.
///
/// Dispatches to alice or bob role based on the `role` argument.
pub async fn run_sas_verify(
	server_url: &str,
	role: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	match role {
		| "alice" => run_alice(server_url).await,
		| "bob" => run_bob(server_url).await,
		| other =>
			Err(format!("unknown role: {other} (expected alice or bob)")
				.into()),
	}
}

/// Generate a fake Ed25519 device signing key (base64-encoded).
///
/// For testing server-side to-device routing, we only need the server
/// to accept and store the key material. Actual cryptographic validity
/// is not required since we are testing message routing, not crypto.
fn fake_device_key_base64() -> String {
	use std::collections::hash_map::DefaultHasher;
	use std::hash::{Hash, Hasher};

	let mut hasher = DefaultHasher::new();
	"sas-verify-test-device-key".hash(&mut hasher);
	let hash = hasher.finish();
	let bytes = hash.to_le_bytes();

	let mut key_bytes = [0u8; 32];
	for i in 0usize..4 {
		let start = i.checked_mul(8).unwrap_or(0);
		let end = start.checked_add(8).unwrap_or(32).min(32);
		key_bytes[start..end].copy_from_slice(&bytes);
	}

	ruma::serde::Base64::<ruma::serde::base64::Standard>::new(
		key_bytes.to_vec(),
	)
	.to_string()
}

/// Upload device keys to the server.
async fn upload_device_keys(
	client: &MatrixClient,
	user_id: &str,
	device_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let token = client
		.access_token()
		.ok_or("not authenticated for key upload")?;
	let url = format!(
		"{}/_matrix/client/v3/keys/upload",
		client.base_url()
	);

	let fake_key = fake_device_key_base64();

	let device_keys = serde_json::json!({
		"user_id": user_id,
		"device_id": device_id,
		"algorithms": [
			"m.olm.v1.curve25519-aes-sha2",
			"m.megolm.v1.aes-sha2"
		],
		"keys": {
			format!("curve25519:{device_id}"): fake_key,
			format!("ed25519:{device_id}"): fake_key,
		},
		"signatures": {
			user_id: {
				format!("ed25519:{device_id}"): fake_key,
			}
		}
	});

	let mut one_time_keys = serde_json::Map::new();
	for i in 0..5u32 {
		let key_id = format!("signed_curve25519:AAAAAQ{i}");
		one_time_keys.insert(
			key_id,
			serde_json::json!({
				"key": fake_key,
				"signatures": {
					user_id: {
						format!("ed25519:{device_id}"): fake_key,
					}
				}
			}),
		);
	}

	let body = serde_json::json!({
		"device_keys": device_keys,
		"one_time_keys": one_time_keys,
	});

	let resp = client
		.http()
		.post(&url)
		.bearer_auth(token)
		.json(&body)
		.send()
		.await?;

	let status = resp.status();
	let resp_body: serde_json::Value = resp.json().await?;

	if !status.is_success() {
		return Err(format!(
			"keys/upload failed: {status} {resp_body}"
		)
		.into());
	}

	eprintln!(
		"{user_id}: device keys uploaded (one_time_key_counts: \
		 {})",
		resp_body
			.get("one_time_key_counts")
			.map_or("unknown".to_owned(), |v| v.to_string())
	);

	Ok(())
}

/// Send a to-device message via the Matrix CS API.
///
/// PUT /_matrix/client/v3/sendToDevice/{eventType}/{txnId}
///
/// The `messages` map is: { user_id: { device_id: content } }
async fn send_to_device(
	client: &MatrixClient,
	event_type: &str,
	txn_id: &str,
	messages: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
	let token = client
		.access_token()
		.ok_or("not authenticated for sendToDevice")?;

	let url = format!(
		"{}/_matrix/client/v3/sendToDevice/{}/{}",
		client.base_url(),
		urlencoding::encode(event_type),
		urlencoding::encode(txn_id),
	);

	let body = serde_json::json!({ "messages": messages });

	let resp = client
		.http()
		.put(&url)
		.bearer_auth(token)
		.json(&body)
		.send()
		.await?;

	let status = resp.status();
	if !status.is_success() {
		let resp_body: serde_json::Value = resp.json().await?;
		return Err(format!(
			"sendToDevice {event_type} failed: {status} \
			 {resp_body}"
		)
		.into());
	}

	eprintln!("sent to-device: {event_type} (txn={txn_id})");
	Ok(())
}

/// Perform a /sync call and return the response.
async fn sync_once(
	client: &MatrixClient,
	since: Option<&str>,
	timeout_ms: u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
	let token = client
		.access_token()
		.ok_or("not authenticated for sync")?;

	let mut url = format!(
		"{}/_matrix/client/v3/sync?timeout={timeout_ms}",
		client.base_url(),
	);
	if let Some(since_token) = since {
		url.push_str(&format!(
			"&since={}",
			urlencoding::encode(since_token)
		));
	}

	let resp = client
		.http()
		.get(&url)
		.bearer_auth(token)
		.send()
		.await?;

	let status = resp.status();
	let resp_body: serde_json::Value = resp.json().await?;

	if !status.is_success() {
		return Err(
			format!("sync failed: {status} {resp_body}").into()
		);
	}

	Ok(resp_body)
}

/// Extract the next_batch token from a sync response.
fn next_batch(sync_resp: &serde_json::Value) -> Option<String> {
	sync_resp
		.get("next_batch")
		.and_then(|v| v.as_str())
		.map(|s| s.to_owned())
}

/// Extract to-device events of a given type from a sync response.
fn extract_to_device_events(
	sync_resp: &serde_json::Value,
	event_type: &str,
) -> Vec<serde_json::Value> {
	sync_resp
		.get("to_device")
		.and_then(|td| td.get("events"))
		.and_then(|e| e.as_array())
		.map(|events| {
			events
				.iter()
				.filter(|ev| {
					ev.get("type").and_then(|t| t.as_str())
						== Some(event_type)
				})
				.cloned()
				.collect()
		})
		.unwrap_or_default()
}

/// Create an encrypted room with the given alias.
async fn create_encrypted_room(
	client: &MatrixClient,
	alias_local: &str,
) -> Result<String, Box<dyn std::error::Error>> {
	let token = client
		.access_token()
		.ok_or("not authenticated for room creation")?;
	let url = format!(
		"{}/_matrix/client/v3/createRoom",
		client.base_url()
	);

	let body = serde_json::json!({
		"room_alias_name": alias_local,
		"visibility": "private",
		"preset": "private_chat",
		"initial_state": [
			{
				"type": "m.room.encryption",
				"state_key": "",
				"content": {
					"algorithm": "m.megolm.v1.aes-sha2"
				}
			}
		]
	});

	let resp = client
		.http()
		.post(&url)
		.bearer_auth(token)
		.json(&body)
		.send()
		.await?;

	let status = resp.status();
	let resp_body: serde_json::Value = resp.json().await?;

	if !status.is_success() {
		return Err(format!(
			"createRoom failed: {status} {resp_body}"
		)
		.into());
	}

	let room_id = resp_body
		.get("room_id")
		.and_then(|v| v.as_str())
		.ok_or("createRoom response missing room_id")?
		.to_owned();

	eprintln!("created encrypted room: {room_id}");
	Ok(room_id)
}

/// Invite a user to a room.
async fn invite_user(
	client: &MatrixClient,
	room_id: &str,
	user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let token = client
		.access_token()
		.ok_or("not authenticated for invite")?;
	let url = format!(
		"{}/_matrix/client/v3/rooms/{}/invite",
		client.base_url(),
		urlencoding::encode(room_id),
	);

	let body = serde_json::json!({ "user_id": user_id });

	let resp = client
		.http()
		.post(&url)
		.bearer_auth(token)
		.json(&body)
		.send()
		.await?;

	let status = resp.status();
	if !status.is_success() {
		let resp_body: serde_json::Value = resp.json().await?;
		return Err(format!(
			"invite failed: {status} {resp_body}"
		)
		.into());
	}

	eprintln!("invited {user_id} to {room_id}");
	Ok(())
}

/// Join a room by alias with retry.
async fn join_room_by_alias(
	client: &MatrixClient,
	alias: &str,
	max_retries: u32,
	retry_interval: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
	let token = client
		.access_token()
		.ok_or("not authenticated for join")?;

	for attempt in 0..max_retries {
		let url = format!(
			"{}/_matrix/client/v3/join/{}",
			client.base_url(),
			urlencoding::encode(alias),
		);

		let resp = client
			.http()
			.post(&url)
			.bearer_auth(token)
			.json(&serde_json::json!({}))
			.send()
			.await?;

		let status = resp.status();
		let resp_body: serde_json::Value = resp.json().await?;

		if status.is_success() {
			let room_id = resp_body
				.get("room_id")
				.and_then(|v| v.as_str())
				.ok_or("join response missing room_id")?
				.to_owned();
			eprintln!(
				"joined {alias} after {attempt} retries: \
				 {room_id}"
			);
			return Ok(room_id);
		}

		eprintln!(
			"join attempt {attempt} for {alias}: {status}"
		);
		tokio::time::sleep(retry_interval).await;
	}

	Err(format!(
		"failed to join {alias} after {max_retries} retries"
	)
	.into())
}

/// Query room members to check if a user has joined.
async fn get_joined_members_count(
	client: &MatrixClient,
	room_id: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
	let token = client
		.access_token()
		.ok_or("not authenticated for members query")?;
	let url = format!(
		"{}/_matrix/client/v3/rooms/{}/joined_members",
		client.base_url(),
		urlencoding::encode(room_id),
	);

	let resp = client
		.http()
		.get(&url)
		.bearer_auth(token)
		.send()
		.await?;

	let status = resp.status();
	let resp_body: serde_json::Value = resp.json().await?;

	if !status.is_success() {
		return Err(format!(
			"joined_members failed: {status} {resp_body}"
		)
		.into());
	}

	let count = resp_body
		.get("joined")
		.and_then(|v| v.as_object())
		.map(|m| m.len())
		.unwrap_or(0);

	#[expect(clippy::as_conversions)]
	Ok(count as u64)
}

/// Shared transaction ID for the verification flow.
/// All verification messages in a single flow share the same
/// transaction_id.
const VERIFICATION_TXN_ID: &str = "sas_verify_txn_001";

/// Alice flow: register, upload keys, create encrypted room, invite
/// bob, wait for bob to join, initiate SAS verification via to-device
/// messages, drive the verification protocol forward through sync.
async fn run_alice(
	server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut client = MatrixClient::new(server_url).await?;

	// Register and login
	client
		.register_with_token(
			"alice",
			DEFAULT_PASSWORD,
			REGISTRATION_TOKEN,
		)
		.await?;
	client.login_user("alice", DEFAULT_PASSWORD).await?;

	let alice_user_id = format!("@alice:{SERVER_NAME}");
	let bob_user_id = format!("@bob:{SERVER_NAME}");

	// Initial sync to establish device presence
	let sync_resp = sync_once(&client, None, 1000).await?;
	let mut batch = next_batch(&sync_resp);
	eprintln!("alice: initial sync complete");

	// Upload device keys
	upload_device_keys(&client, &alice_user_id, "ALICE_DEV")
		.await?;

	// Create encrypted room with alias
	let room_id =
		create_encrypted_room(&client, "sas-room").await?;

	// Invite bob
	invite_user(&client, &room_id, &bob_user_id).await?;

	// Wait for bob to join (poll member count)
	let poll_interval = Duration::from_millis(2000);
	for attempt in 0..30u32 {
		let count =
			get_joined_members_count(&client, &room_id).await?;
		if count >= 2 {
			eprintln!(
				"alice: bob has joined (attempt {attempt})"
			);
			break;
		}
		if attempt == 29 {
			return Err(
				"alice: timed out waiting for bob to join"
					.into(),
			);
		}
		eprintln!(
			"alice: waiting for bob (attempt {attempt}, \
			 members: {count})"
		);
		tokio::time::sleep(poll_interval).await;
	}

	// Sync to pick up bob's join
	let sync_resp =
		sync_once(&client, batch.as_deref(), 1000).await?;
	batch = next_batch(&sync_resp);
	eprintln!("alice: synced after bob joined");

	// Step 1: Send m.key.verification.request to bob's device.
	// This initiates the SAS verification protocol.
	let request_content = serde_json::json!({
		&bob_user_id: {
			"BOB_DEV": {
				"from_device": "ALICE_DEV",
				"methods": ["m.sas.v1"],
				"timestamp": 1000,
				"transaction_id": VERIFICATION_TXN_ID
			}
		}
	});

	send_to_device(
		&client,
		"m.key.verification.request",
		"sas_txn_01",
		&request_content,
	)
	.await?;
	eprintln!("alice: sent verification request to bob");

	// Poll sync for bob's response messages.
	// Drive the verification protocol forward by processing
	// incoming to-device events and sending appropriate responses.
	let mut got_ready = false;
	let mut got_key = false;
	let mut got_mac = false;

	for attempt in 0..60u32 {
		let sync_resp =
			sync_once(&client, batch.as_deref(), 5000).await?;
		batch = next_batch(&sync_resp);

		// Check for m.key.verification.ready from bob
		if !got_ready {
			let ready_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.ready",
			);
			if !ready_events.is_empty() {
				eprintln!(
					"alice: received verification.ready \
					 from bob"
				);
				got_ready = true;

				// Step 3: Send m.key.verification.start
				let start_content = serde_json::json!({
					&bob_user_id: {
						"BOB_DEV": {
							"from_device": "ALICE_DEV",
							"method": "m.sas.v1",
							"transaction_id":
								VERIFICATION_TXN_ID,
							"key_agreement_protocols":
								["curve25519-hkdf-sha256"],
							"hashes": ["sha256"],
							"message_authentication_codes":
								["hkdf-hmac-sha256.v2"],
							"short_authentication_string":
								["emoji", "decimal"]
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.start",
					"sas_txn_03",
					&start_content,
				)
				.await?;
				eprintln!(
					"alice: sent verification.start"
				);

				// Also send m.key.verification.key
				// (fake key material for routing test)
				let key_content = serde_json::json!({
					&bob_user_id: {
						"BOB_DEV": {
							"transaction_id":
								VERIFICATION_TXN_ID,
							"key": fake_device_key_base64()
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.key",
					"sas_txn_04",
					&key_content,
				)
				.await?;
				eprintln!("alice: sent verification.key");
			}
		}

		// Check for m.key.verification.key from bob
		if got_ready && !got_key {
			let key_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.key",
			);
			if !key_events.is_empty() {
				eprintln!(
					"alice: received verification.key \
					 from bob"
				);
				got_key = true;

				// Step 5: Send m.key.verification.mac
				let mac_content = serde_json::json!({
					&bob_user_id: {
						"BOB_DEV": {
							"transaction_id":
								VERIFICATION_TXN_ID,
							"keys": "fake_key_mac_value",
							"mac": {
								format!(
									"ed25519:ALICE_DEV"
								):
									"fake_mac_value"
							}
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.mac",
					"sas_txn_06",
					&mac_content,
				)
				.await?;
				eprintln!("alice: sent verification.mac");
			}
		}

		// Check for m.key.verification.mac from bob
		if got_key && !got_mac {
			let mac_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.mac",
			);
			if !mac_events.is_empty() {
				eprintln!(
					"alice: received verification.mac \
					 from bob"
				);
				got_mac = true;

				// Step 7: Send m.key.verification.done
				let done_content = serde_json::json!({
					&bob_user_id: {
						"BOB_DEV": {
							"transaction_id":
								VERIFICATION_TXN_ID
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.done",
					"sas_txn_08",
					&done_content,
				)
				.await?;
				eprintln!(
					"alice: sent verification.done"
				);
				break;
			}
		}

		eprintln!(
			"alice: sync poll {attempt} (ready={got_ready}, \
			 key={got_key}, mac={got_mac})"
		);
		tokio::time::sleep(poll_interval).await;
	}

	if !got_mac {
		return Err(
			"alice: timed out waiting for SAS verification \
			 to complete"
				.into(),
		);
	}

	eprintln!("alice: sas verification complete");
	Ok(())
}

/// Bob flow: register, upload keys, join encrypted room, wait for
/// verification request from alice, auto-accept and drive the protocol
/// to completion by responding to each step.
async fn run_bob(
	server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut client = MatrixClient::new(server_url).await?;

	// Register and login
	client
		.register_with_token(
			"bob",
			DEFAULT_PASSWORD,
			REGISTRATION_TOKEN,
		)
		.await?;
	client.login_user("bob", DEFAULT_PASSWORD).await?;

	let alice_user_id = format!("@alice:{SERVER_NAME}");
	let bob_user_id = format!("@bob:{SERVER_NAME}");

	// Initial sync to establish device presence
	let sync_resp = sync_once(&client, None, 1000).await?;
	let mut batch = next_batch(&sync_resp);
	eprintln!("bob: initial sync complete");

	// Upload device keys
	upload_device_keys(&client, &bob_user_id, "BOB_DEV").await?;

	// Join encrypted room by alias with retry
	let sas_alias = format!("#sas-room:{SERVER_NAME}");
	let _room_id = join_room_by_alias(
		&client,
		&sas_alias,
		30,
		Duration::from_millis(1000),
	)
	.await?;
	eprintln!("bob: joined sas room");

	// Sync after join
	let sync_resp =
		sync_once(&client, batch.as_deref(), 1000).await?;
	batch = next_batch(&sync_resp);
	eprintln!("bob: synced after join");

	// Poll sync for verification events from alice and respond.
	let poll_interval = Duration::from_millis(2000);
	let mut got_request = false;
	let mut got_start = false;
	let mut got_key = false;
	let mut got_mac = false;
	let mut got_done = false;

	for attempt in 0..60u32 {
		let sync_resp =
			sync_once(&client, batch.as_deref(), 5000).await?;
		batch = next_batch(&sync_resp);

		// Check for m.key.verification.request from alice
		if !got_request {
			let req_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.request",
			);
			if !req_events.is_empty() {
				eprintln!(
					"bob: received verification.request \
					 from alice"
				);
				got_request = true;

				// Step 2: Send m.key.verification.ready
				let ready_content = serde_json::json!({
					&alice_user_id: {
						"ALICE_DEV": {
							"from_device": "BOB_DEV",
							"methods": ["m.sas.v1"],
							"transaction_id":
								VERIFICATION_TXN_ID
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.ready",
					"sas_txn_02",
					&ready_content,
				)
				.await?;
				eprintln!(
					"bob: sent verification.ready"
				);
			}
		}

		// Check for m.key.verification.start from alice
		// (may arrive in same sync or next)
		if got_request && !got_start {
			let start_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.start",
			);
			if !start_events.is_empty() {
				eprintln!(
					"bob: received verification.start \
					 from alice"
				);
				got_start = true;
			}
		}

		// Check for m.key.verification.key from alice
		if got_request && !got_key {
			let key_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.key",
			);
			if !key_events.is_empty() {
				eprintln!(
					"bob: received verification.key \
					 from alice"
				);
				got_key = true;

				// Step 4: Send m.key.verification.key back
				let key_content = serde_json::json!({
					&alice_user_id: {
						"ALICE_DEV": {
							"transaction_id":
								VERIFICATION_TXN_ID,
							"key": fake_device_key_base64()
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.key",
					"sas_txn_05",
					&key_content,
				)
				.await?;
				eprintln!("bob: sent verification.key");
			}
		}

		// Check for m.key.verification.mac from alice
		if got_key && !got_mac {
			let mac_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.mac",
			);
			if !mac_events.is_empty() {
				eprintln!(
					"bob: received verification.mac \
					 from alice"
				);
				got_mac = true;

				// Step 6: Send m.key.verification.mac back
				let mac_content = serde_json::json!({
					&alice_user_id: {
						"ALICE_DEV": {
							"transaction_id":
								VERIFICATION_TXN_ID,
							"keys": "fake_key_mac_value",
							"mac": {
								"ed25519:BOB_DEV":
									"fake_mac_value"
							}
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.mac",
					"sas_txn_07",
					&mac_content,
				)
				.await?;
				eprintln!("bob: sent verification.mac");
			}
		}

		// Check for m.key.verification.done from alice
		if got_mac && !got_done {
			let done_events = extract_to_device_events(
				&sync_resp,
				"m.key.verification.done",
			);
			if !done_events.is_empty() {
				eprintln!(
					"bob: received verification.done \
					 from alice"
				);
				got_done = true;

				// Step 8: Send m.key.verification.done back
				let done_content = serde_json::json!({
					&alice_user_id: {
						"ALICE_DEV": {
							"transaction_id":
								VERIFICATION_TXN_ID
						}
					}
				});

				send_to_device(
					&client,
					"m.key.verification.done",
					"sas_txn_09",
					&done_content,
				)
				.await?;
				eprintln!("bob: sent verification.done");
				break;
			}
		}

		eprintln!(
			"bob: sync poll {attempt} (request={got_request}, \
			 start={got_start}, key={got_key}, \
			 mac={got_mac}, done={got_done})"
		);
		tokio::time::sleep(poll_interval).await;
	}

	if !got_done {
		return Err(
			"bob: timed out waiting for SAS verification \
			 to complete"
				.into(),
		);
	}

	eprintln!("bob: sas verification complete");
	Ok(())
}

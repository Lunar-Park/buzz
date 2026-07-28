use super::*;
use crate::managed_agents::spawn_key_refusal;

const AGENT_HEX: &str = "3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d";
const AGENT_NPUB: &str = "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6";

fn sample_record() -> ManagedAgentRecord {
    connected_record(
        "lunar02",
        AGENT_HEX,
        "Scout",
        Some("claude".to_string()),
        "2026-07-28T00:00:00Z",
    )
}

#[test]
fn npub_and_hex_normalize_to_the_same_stored_form() {
    // Both are forms a user legitimately has on hand. If they normalized
    // differently, connecting the same agent twice — once from each form —
    // would pass the pubkey collision check and produce two records for one
    // identity.
    assert_eq!(normalize_agent_pubkey(AGENT_NPUB).unwrap(), AGENT_HEX);
    assert_eq!(normalize_agent_pubkey(AGENT_HEX).unwrap(), AGENT_HEX);
}

#[test]
fn uppercase_hex_is_normalized_rather_than_stored_verbatim() {
    let shouty = AGENT_HEX.to_uppercase();
    assert_eq!(normalize_agent_pubkey(&shouty).unwrap(), AGENT_HEX);
}

#[test]
fn surrounding_whitespace_is_tolerated() {
    // Pasted from a terminal, an npub routinely arrives with a trailing
    // newline.
    assert_eq!(
        normalize_agent_pubkey(&format!("  {AGENT_NPUB}\n")).unwrap(),
        AGENT_HEX
    );
}

#[test]
fn a_pasted_secret_key_is_refused_with_a_specific_message() {
    // The whole point of this feature is that the agent's secret stays on its
    // own machine. A user who pastes an nsec has made a serious mistake, and
    // "invalid pubkey" would not tell them what it was.
    let error =
        normalize_agent_pubkey("nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5")
            .expect_err("an nsec must never be accepted as an agent pubkey");
    assert!(
        error.contains("secret key"),
        "message must name the mistake: {error}"
    );
    assert!(
        !error.contains("nsec1vl029"),
        "the secret must not be echoed back into an error string: {error}"
    );
}

#[test]
fn malformed_pubkeys_are_refused() {
    for bad in [
        "",
        "   ",
        "not-a-key",
        "npub1truncated",
        // 63 hex chars — one short.
        "3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459",
    ] {
        assert!(
            normalize_agent_pubkey(bad).is_err(),
            "expected {bad:?} to be refused"
        );
    }
}

#[test]
fn names_are_trimmed_and_bounded() {
    assert_eq!(validate_connected_name("  Scout  ").unwrap(), "Scout");
    assert!(validate_connected_name("").is_err());
    assert!(validate_connected_name("   ").is_err());
    assert!(validate_connected_name(&"n".repeat(65)).is_err());
    assert!(validate_connected_name(&"n".repeat(64)).is_ok());
    // A newline in a name would break every single-line list rendering it.
    assert!(validate_connected_name("Sco\nut").is_err());
}

#[test]
fn a_connected_record_carries_remote_custody_and_no_key() {
    let record = sample_record();
    assert_eq!(
        record.key_custody,
        KeyCustody::Remote {
            host: "lunar02".to_string()
        }
    );
    assert!(
        record.private_key_nsec.is_empty(),
        "Buzz must never hold a self-hosted agent's key"
    );
    assert_eq!(record.pubkey, AGENT_HEX);
    assert_eq!(record.runtime.as_deref(), Some("claude"));
}

#[test]
fn a_connected_record_is_not_an_auto_start_candidate() {
    // Belt and braces with the custody filter in `load_managed_agents`: if a
    // future reader ever hands this record to the restore path, the
    // `start_on_app_launch` gate there must also reject it.
    let record = sample_record();
    assert!(!record.start_on_app_launch);
    assert!(!record.auto_restart_on_config_change);
    assert!(record.runtime_pid.is_none());
}

#[test]
fn a_connected_record_has_no_spawn_plumbing() {
    // Nothing in Buzz should be able to assemble a command line for an agent
    // it does not run. `runtime` holds the observed harness id for display,
    // which is deliberately not the same thing as an executable command.
    let record = sample_record();
    assert!(record.agent_command.is_empty());
    assert!(record.agent_command_override.is_none());
    assert!(record.agent_args.is_empty());
    assert!(record.env_vars.is_empty());
    assert!(record.system_prompt.is_none());
}

#[test]
fn spawn_is_refused_with_the_custody_reason_not_a_keyring_error() {
    // A connected record is filtered out of `load_managed_agents`, so no
    // ordinary path reaches the refusal. When some other path does, the
    // message must state the fact ("its identity lives on lunar02") rather
    // than blame the keyring — that would send the user to fix a component
    // that is working correctly.
    let record = sample_record();
    let refusal =
        spawn_key_refusal(&record).expect("a record with no local key must never be spawned");
    assert!(
        refusal.contains("lunar02"),
        "the refusal must name the host: {refusal}"
    );
    assert!(
        !refusal.to_lowercase().contains("keyring"),
        "an absent key is the design here, not an outage: {refusal}"
    );
}

#[test]
fn the_summary_projection_omits_lifecycle_and_secrets() {
    // `ConnectedAgentSummary` is intentionally narrower than
    // `ManagedAgentSummary`. Serialize it and assert the absent fields stay
    // absent: a later widening that reintroduces `status` or `pid` would give
    // the UI something to render a start button from.
    let record = sample_record();
    let summary = to_connected_summary(&record);
    let json = serde_json::to_value(&summary).unwrap();
    let object = json.as_object().unwrap();

    assert_eq!(object.get("host").unwrap(), "lunar02");
    assert_eq!(object.get("harness").unwrap(), "claude");
    assert_eq!(object.get("pubkey").unwrap(), AGENT_HEX);
    for absent in [
        "status",
        "pid",
        "logPath",
        "log_path",
        "needsRestart",
        "startOnAppLaunch",
        "privateKeyNsec",
        "private_key_nsec",
        "relayUrl",
    ] {
        assert!(
            !object.contains_key(absent),
            "{absent} must not reach the connected-agent surface"
        );
    }
}

#[test]
fn a_local_record_serializes_without_a_custody_key() {
    // `skip_serializing_if` keeps the new field out of every existing agent's
    // stored JSON, so adding key custody does not rewrite the whole store.
    let mut record = sample_record();
    record.key_custody = KeyCustody::Local;
    let json = serde_json::to_value(&record).unwrap();
    assert!(!json.as_object().unwrap().contains_key("key_custody"));
}

#[test]
fn an_existing_store_record_deserializes_as_local_custody() {
    // The no-migration guarantee: a record written by a build that had never
    // heard of key custody must load as Buzz-owned.
    let stored = serde_json::json!({
        "pubkey": AGENT_HEX,
        "name": "Legacy",
        "relay_url": "wss://relay.example.com",
        "acp_command": "buzz-acp",
        "agent_command": "goose",
        "agent_args": [],
        "mcp_command": "",
        "turn_timeout_seconds": 120,
        "system_prompt": null,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "last_started_at": null,
        "last_stopped_at": null,
        "last_exit_code": null,
        "last_error": null,
    });
    let record: ManagedAgentRecord = serde_json::from_value(stored).unwrap();
    assert_eq!(record.key_custody, KeyCustody::Local);
    assert!(record.key_custody.is_local());
    assert!(record.key_custody.remote_host().is_none());
}

#[test]
fn custody_round_trips_through_the_store_format() {
    let record = sample_record();
    let json = serde_json::to_string(&record).unwrap();
    let restored: ManagedAgentRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.key_custody, record.key_custody);
    assert_eq!(
        restored.key_custody.remote_host(),
        Some("lunar02"),
        "the host must survive a store write/read cycle — it is the probe target"
    );
}

#[test]
fn an_unknown_ssh_host_is_refused_with_the_fix_named() {
    // The host is a probe target. Accepting a free-form string would create a
    // row whose reachability can never be reported, which reads as a broken
    // feature rather than a missing config entry.
    let error = resolve_connect_host("definitely-not-in-any-ssh-config-xyzzy")
        .expect_err("an unknown alias must be refused");
    assert!(
        error.contains("~/.ssh/config"),
        "the message must name the fix: {error}"
    );
}

#[test]
fn a_blank_host_is_refused() {
    assert!(resolve_connect_host("").is_err());
    assert!(resolve_connect_host("   ").is_err());
}

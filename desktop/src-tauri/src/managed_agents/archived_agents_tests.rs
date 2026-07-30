//! Tests for the archived-agent store.
//!
//! What these cover is that stage one is genuinely reversible and genuinely
//! invisible: an archived agent keeps everything a restore needs, cannot be
//! loaded by any managed-agent path, and its store fails loud rather than
//! swallowing a parse error — because this file is the only place a removed
//! agent can be recovered from.

use std::fs;

use super::*;

fn agent(pubkey: &str, name: &str, nsec: &str) -> ManagedAgentRecord {
    serde_json::from_str(&format!(
        r#"{{
            "pubkey": "{pubkey}",
            "name": "{name}",
            "private_key_nsec": "{nsec}",
            "relay_url": "wss://localhost:3000",
            "acp_command": "buzz-acp",
            "agent_command": "goose",
            "agent_args": [],
            "mcp_command": "",
            "turn_timeout_seconds": 320,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }}"#
    ))
    .expect("sample record")
}

fn archived(pubkey: &str, name: &str) -> ArchivedAgentRecord {
    ArchivedAgentRecord {
        archived_at: "2026-07-30T00:00:00Z".to_string(),
        agent: agent(pubkey, name, ""),
    }
}

#[test]
fn an_absent_store_is_an_empty_list_not_an_error() {
    // Every install before the first archive. This must not surface as a load
    // failure in the agents view.
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("archived-agents.json");
    assert_eq!(load_archived_agents_at(&path).unwrap(), Vec::new());
}

#[test]
fn an_archived_agent_round_trips_with_everything_a_restore_needs() {
    // The point of stage one: this is a move, not a rebuild. Anything dropped
    // here is something the user silently cannot get back.
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("archived-agents.json");
    let record = archived("pubkey-a", "Scout");

    save_archived_agents_at(&path, std::slice::from_ref(&record)).expect("save");
    let loaded = load_archived_agents_at(&path).expect("load");

    assert_eq!(loaded, vec![record.clone()]);
    assert_eq!(loaded[0].agent.agent_command, record.agent.agent_command);
    assert_eq!(loaded[0].agent.relay_url, record.agent.relay_url);
    assert_eq!(loaded[0].archived_at, "2026-07-30T00:00:00Z");
}

#[test]
fn records_are_sorted_for_stable_diffs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("archived-agents.json");
    save_archived_agents_at(
        &path,
        &[archived("pubkey-z", "zeta"), archived("pubkey-a", "Alpha")],
    )
    .expect("save");

    let names: Vec<String> = load_archived_agents_at(&path)
        .unwrap()
        .into_iter()
        .map(|record| record.agent.name)
        .collect();
    assert_eq!(names, ["Alpha", "zeta"], "case-insensitive name order");
}

#[test]
fn a_malformed_store_fails_loudly_and_preserves_the_evidence() {
    // This file is the only place a removed agent can be recovered from, so
    // parsing it as an empty list would turn a recoverable archive into a
    // permanent loss on the next save.
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("archived-agents.json");
    fs::write(&path, b"{ not an array").expect("seed");

    let error =
        load_archived_agents_at(&path).expect_err("a malformed store must not load as empty");
    assert!(error.contains(".invalid"), "message must name the backup");
    assert!(
        path.with_extension("json.invalid").exists(),
        "the malformed content must survive for the user to recover"
    );
}

#[test]
fn an_empty_file_is_an_empty_list() {
    // A truncated write leaves a zero-byte file; treating that as malformed
    // would block the archived list behind an error it cannot clear.
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("archived-agents.json");
    fs::write(&path, b"   ").expect("seed");
    assert_eq!(load_archived_agents_at(&path).unwrap(), Vec::new());
}

#[test]
fn the_store_is_written_owner_only() {
    // An archived record is a full `ManagedAgentRecord`, which keeps its nsec
    // inline whenever the keyring is unreachable. Unlike the connected store,
    // this file can therefore contain a secret and must not be group-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("archived-agents.json");
        save_archived_agents_at(&path, &[archived("pubkey-a", "Scout")]).expect("save");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(
            mode & 0o077,
            0,
            "archived store must be owner-only: {mode:o}"
        );
    }
}

#[test]
fn an_archived_record_cannot_be_read_as_a_managed_agent() {
    // The invisibility invariant, stated as data. Even a reader pointed at the
    // wrong file cannot produce a `ManagedAgentRecord` from an archived row: the
    // wrapper's shape does not satisfy it, so no lifecycle path can act on an
    // archived agent by accident.
    let record = archived("pubkey-a", "Scout");
    let json = serde_json::to_value(&record).unwrap();
    assert!(
        serde_json::from_value::<ManagedAgentRecord>(json).is_err(),
        "an archived row must not satisfy ManagedAgentRecord"
    );
}

#[test]
fn the_summary_reports_what_a_permanent_delete_would_destroy() {
    let record = archived("pubkey-a", "Scout");
    let summary = ArchivedAgentSummary::from(&record);
    assert_eq!(summary.pubkey, "pubkey-a");
    assert_eq!(summary.name, "Scout");
    assert_eq!(summary.archived_at, "2026-07-30T00:00:00Z");
    assert!(
        summary.retains_identity,
        "stage one keeps the identity — the archived list must say so"
    );
}

#[test]
fn retains_identity_does_not_depend_on_key_hydration() {
    // Archived records load without hydration, so an un-hydrated row must still
    // report a recoverable identity. Reading `private_key_nsec` here would report
    // false negatives for every agent whose key is safely in the keyring.
    let record = archived("pubkey-a", "Scout");
    assert!(record.agent.private_key_nsec.is_empty(), "un-hydrated");
    assert!(ArchivedAgentSummary::from(&record).retains_identity);
}

#[test]
fn the_summary_omits_the_secret() {
    let mut record = archived("pubkey-a", "Scout");
    record.agent.private_key_nsec = "nsec1leaked".to_string();
    let json = serde_json::to_string(&ArchivedAgentSummary::from(&record)).expect("serialize");
    assert!(!json.contains("nsec1leaked"), "{json}");
    assert!(!json.contains("privateKey"), "{json}");
}

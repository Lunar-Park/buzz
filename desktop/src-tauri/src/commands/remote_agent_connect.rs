//! Connecting a self-hosted agent — one that already runs on a machine the
//! user owns, supervises itself, and holds its own key.
//!
//! This is *connect*, not create. Every other agent path in Buzz mints an
//! identity, writes a key, and takes responsibility for a process. Here Buzz
//! learns about an identity that already exists and records where it lives.
//! The resulting record carries [`KeyCustody::Remote`], which keeps it out of
//! [`load_managed_agents`] and therefore out of every spawn, deploy,
//! auto-start, profile-republish, and tombstone path.
//!
//! Three things this deliberately does not do:
//!
//! - **No key transport.** The agent's nsec never crosses the network, is
//!   never requested, and is never stored. Buzz holds the public half only.
//! - **No published claim.** Connecting does not emit an owner-signed
//!   kind:30177 "I manage this agent" event. That event is what
//!   `delete_managed_agent` tombstones, so publishing it would let Buzz
//!   assert — and later revoke — the directory entry for an agent it cannot
//!   restart. A self-hosted agent's directory presence is its own replaceable
//!   kind:10100, signed with the key Buzz has never held.
//! - **No lifecycle.** There is no start, stop, restart, or deploy here, and
//!   `disconnect` removes Buzz's local pointer without touching the remote
//!   process. Disconnecting an agent that is happily running is expected to
//!   leave it running.

use tauri::{AppHandle, Manager};

use nostr::nips::nip19::FromBech32;

use crate::app_state::AppState;
use crate::managed_agents::ssh_config::parse_ssh_config;
use crate::managed_agents::{
    load_agent_store, load_connected_agents, save_connected_agents, ConnectedAgentSummary,
    KeyCustody, ManagedAgentRecord, DEFAULT_ACP_COMMAND,
};
use crate::util::now_iso;

/// Longest accepted local label. Matches nothing on the wire — this name is
/// Buzz-local, so the limit only needs to keep the list readable.
const MAX_CONNECTED_NAME_LEN: usize = 64;

/// Project a stored record onto the connected-agent view.
///
/// A record that reached here under local custody would be a bug in the
/// caller's filtering, so `host` falls back to the empty string rather than
/// silently borrowing a plausible-looking value.
fn to_connected_summary(record: &ManagedAgentRecord) -> ConnectedAgentSummary {
    ConnectedAgentSummary {
        pubkey: record.pubkey.clone(),
        name: record.name.clone(),
        host: record
            .key_custody
            .remote_host()
            .unwrap_or_default()
            .to_string(),
        harness: record.runtime.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    }
}

/// Normalize a user-supplied agent pubkey to 64-char lowercase hex.
///
/// Both `npub1…` and bare hex are accepted because both are things a user
/// legitimately has on hand: `npub` is what an agent's own tooling prints,
/// hex is what appears in event tags and relay queries. Normalizing at this
/// one boundary means the stored record and every comparison downstream sees
/// a single form — a mixed-case hex duplicate of an already-connected agent
/// would otherwise slip past the collision check below.
pub(crate) fn normalize_agent_pubkey(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("agent pubkey is required".to_string());
    }
    if let Some(stripped) = trimmed.strip_prefix("nsec") {
        // Refuse loudly and specifically. A user who pastes a secret key here
        // has made a serious mistake, and "invalid pubkey" would not tell them
        // what it was. The value itself is never echoed back.
        let _ = stripped;
        return Err(
            "that is a secret key (nsec), not a pubkey — a self-hosted agent's secret must \
             never leave its own machine. Paste the agent's npub instead."
                .to_string(),
        );
    }
    let parsed = if trimmed.starts_with("npub") {
        nostr::PublicKey::from_bech32(trimmed)
            .map_err(|_| "invalid npub — check for a truncated or mistyped value".to_string())?
    } else {
        nostr::PublicKey::from_hex(trimmed).map_err(|_| {
            "invalid agent pubkey — expected an npub or 64 hex characters".to_string()
        })?
    };
    Ok(parsed.to_hex())
}

/// Validate the Buzz-local label for a connected agent.
pub(crate) fn validate_connected_name(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("agent name is required".to_string());
    }
    if trimmed.chars().count() > MAX_CONNECTED_NAME_LEN {
        return Err(format!(
            "agent name must be at most {MAX_CONNECTED_NAME_LEN} characters"
        ));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("agent name must not contain control characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// Resolve a host alias against the user's own `~/.ssh/config`.
///
/// Requiring a real alias is not gratuitous strictness. The host is a probe
/// target: `probe_agent_host` re-resolves it through this same parsed config
/// and refuses anything it cannot find, so a free-form host string would
/// produce a connected agent whose reachability could never be reported — a
/// row that silently never works. Failing at connect time, with the fix named,
/// is the honest alternative.
fn resolve_connect_host(host: &str) -> Result<String, String> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err("host is required".to_string());
    }
    let known = parse_ssh_config();
    known
        .iter()
        .find(|candidate| candidate.host == trimmed)
        .map(|candidate| candidate.host.clone())
        .ok_or_else(|| {
            format!(
                "'{trimmed}' is not a Host in ~/.ssh/config. Add a stanza for it (Buzz reaches \
                 self-hosted agents through your own ssh config) and try again."
            )
        })
}

/// List the self-hosted agents this machine is connected to.
#[tauri::command]
pub async fn list_connected_agents(app: AppHandle) -> Result<Vec<ConnectedAgentSummary>, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;
        let records = load_connected_agents(&app)?;
        Ok(records.iter().map(to_connected_summary).collect())
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

/// Record a self-hosted agent that already runs on `host`.
///
/// `harness` is the id observed by the host probe (e.g. `"claude"`). It is
/// stored as an observation for display; nothing in Buzz executes it.
#[tauri::command]
pub async fn connect_remote_agent(
    host: String,
    pubkey: String,
    name: String,
    harness: Option<String>,
    app: AppHandle,
) -> Result<ConnectedAgentSummary, String> {
    tokio::task::spawn_blocking(move || {
        // Validate everything before taking the store lock: none of these
        // checks need the store, and a bad input should not serialize behind
        // an unrelated agent save.
        let host = resolve_connect_host(&host)?;
        let pubkey = normalize_agent_pubkey(&pubkey)?;
        let name = validate_connected_name(&name)?;
        let harness = harness.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        });

        let state = app.state::<AppState>();

        // Connecting your own identity would make you an agent that replies to
        // your own messages. The relay- and desktop-side loop guards key off
        // author identity, so this is the one collision they cannot help with.
        // An unavailable identity is not a reason to block the connect.
        if let Ok(keys) = state.signing_keys() {
            if keys.public_key().to_hex() == pubkey {
                return Err(
                    "that is your own pubkey. Connect the agent's identity, not yours — \
                     an agent sharing your key would answer your own messages."
                        .to_string(),
                );
            }
        }

        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;

        // Collision checks run against the WHOLE store, not just the connected
        // half. A pubkey Buzz already manages locally must not also appear as
        // connected: that would be one identity with two records, one of which
        // claims custody Buzz does not have.
        let existing = load_agent_store(&app)?;
        if let Some(clash) = existing.iter().find(|record| record.pubkey == pubkey) {
            return Err(match clash.key_custody.remote_host() {
                Some(existing_host) => format!(
                    "that agent is already connected as '{}' on {existing_host}",
                    clash.name
                ),
                None => format!(
                    "'{}' is an agent Buzz already manages on this machine — it holds that \
                     agent's key, so it cannot also be connected as self-hosted",
                    clash.name
                ),
            });
        }
        if existing
            .iter()
            .any(|record| record.name.eq_ignore_ascii_case(&name))
        {
            return Err(format!(
                "an agent named '{name}' already exists — names are how agents are mentioned, \
                 so pick a different one"
            ));
        }

        let now = now_iso();
        let record = connected_record(&host, &pubkey, &name, harness, &now);
        let summary = to_connected_summary(&record);

        let mut connected = load_connected_agents(&app)?;
        connected.push(record);
        save_connected_agents(&app, &connected)?;

        Ok(summary)
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

/// Build the stored record for a connected agent.
///
/// Split out as a pure function so the invariants that matter — no key, no
/// auto-start, `Remote` custody — are directly testable without a Tauri app
/// handle.
pub(crate) fn connected_record(
    host: &str,
    pubkey: &str,
    name: &str,
    harness: Option<String>,
    now: &str,
) -> ManagedAgentRecord {
    ManagedAgentRecord {
        pubkey: pubkey.to_string(),
        name: name.to_string(),
        display_name: None,
        persona_id: None,
        team_id: None,
        // Empty, and it stays empty. There is no code path that fills this for
        // a connected agent; `spawn_key_refusal` reports the custody reason
        // rather than the keyring one precisely because of this record shape.
        private_key_nsec: String::new(),
        auth_tag: None,
        // Resolved from the active workspace at read time for every agent
        // (`effective_agent_relay_url`), so a stored pin would be dead weight.
        relay_url: String::new(),
        avatar_url: None,
        acp_command: DEFAULT_ACP_COMMAND.to_string(),
        // No spawn plumbing: Buzz never builds a command line for this agent.
        // The observed harness lives in `runtime` as a display label.
        agent_command: String::new(),
        agent_command_override: None,
        agent_args: Vec::new(),
        mcp_command: String::new(),
        turn_timeout_seconds: 0,
        idle_timeout_seconds: None,
        max_turn_duration_seconds: None,
        parallelism: 1,
        system_prompt: None,
        model: None,
        provider: None,
        persona_source_version: None,
        env_vars: Default::default(),
        // Nothing to auto-start. Redundant with the custody filter, and
        // deliberately so: if a future reader ever hands this record to the
        // restore path, it must not be a candidate there either.
        start_on_app_launch: false,
        auto_restart_on_config_change: false,
        runtime_pid: None,
        backend: crate::managed_agents::BackendKind::Local,
        backend_agent_id: None,
        provider_binary_path: None,
        key_custody: KeyCustody::Remote {
            host: host.to_string(),
        },
        persona_team_dir: None,
        persona_name_in_team: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        last_started_at: None,
        last_stopped_at: None,
        last_exit_code: None,
        last_error: None,
        last_error_code: None,
        respond_to: Default::default(),
        respond_to_allowlist: Vec::new(),
        slug: None,
        runtime: harness,
        name_pool: Vec::new(),
        is_builtin: false,
        is_active: true,
        source_team: None,
        source_team_persona_slug: None,
        definition_respond_to: None,
        definition_respond_to_allowlist: Vec::new(),
        definition_parallelism: None,
        relay_mesh: None,
    }
}

/// Forget a connected agent.
///
/// Local-only by construction: this removes Buzz's pointer and nothing else.
/// It deliberately does not take the paths `delete_managed_agent` takes —
/// no process stop (Buzz owns no process), no keyring delete (Buzz holds no
/// key), and above all no kind:30177 tombstone or NIP-IA archive. Those
/// publish the owner's assertion that an agent is gone; running them for an
/// agent that is still alive on its own machine would remove a working agent
/// from every member picker and autocomplete on the relay.
#[tauri::command]
pub async fn disconnect_remote_agent(pubkey: String, app: AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let pubkey = normalize_agent_pubkey(&pubkey)?;
        let state = app.state::<AppState>();
        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;

        let mut connected = load_connected_agents(&app)?;
        let before = connected.len();
        connected.retain(|record| record.pubkey != pubkey);
        if connected.len() == before {
            return Err(format!("connected agent {pubkey} not found"));
        }
        save_connected_agents(&app, &connected)
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

#[cfg(test)]
#[path = "remote_agent_connect_tests.rs"]
mod tests;

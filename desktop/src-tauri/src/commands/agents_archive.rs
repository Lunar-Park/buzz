//! Two-stage removal for agents Buzz owns.
//!
//! `delete_managed_agent` is irreversible in ways a confirmation dialog cannot
//! undo: it deletes the key from the keyring, tombstones the kind:30177 record,
//! and publishes a NIP-IA archive. This module puts a reversible step in front of
//! it, and gates the irreversible one behind that step.
//!
//! - [`describe_managed_agent_removal`] — what stage one would affect, so the
//!   confirmation can name it instead of asking the user to guess.
//! - [`archive_managed_agent`] — stop it, move it to the archived store, keep the
//!   identity. Reversible.
//! - [`restore_archived_agent`] — move it back, intact.
//! - [`permanently_delete_archived_agent`] — the irreversible half, reachable only
//!   from the archived state.
//!
//! What permanent deletion cannot do is erase already-published events, relay
//! audit records, or copies other clients hold. Callers must say that rather than
//! implying the identity disappears from the network.

use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::managed_agents::{
    load_archived_agents, load_managed_agents, load_teams, save_archived_agents,
    save_managed_agents, stop_managed_agent_process, ArchivedAgentRecord, ArchivedAgentSummary,
};
use crate::util::now_iso;

/// What archiving an agent would touch.
///
/// Read-only. Exists because "are you sure?" is not a question a user can answer
/// without knowing whether the agent is running and which teams reference it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAgentRemovalPreview {
    pub pubkey: String,
    pub name: String,
    /// True when Buzz has a live local process for this agent that archiving
    /// will stop.
    pub is_running: bool,
    /// Names of teams that list this agent, so the confirmation can say what
    /// loses a member.
    pub team_names: Vec<String>,
    /// Whether Buzz holds signing material that a later permanent delete would
    /// destroy. Read from the hydrated managed record, so unlike the archived
    /// summary's `retains_identity` this one does reflect real key material.
    pub has_local_key: bool,
}

/// Describe what `Remove from My Agents` would do to `pubkey`.
#[tauri::command]
pub async fn describe_managed_agent_removal(
    pubkey: String,
    app: AppHandle,
) -> Result<ManagedAgentRemovalPreview, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;

        let records = load_managed_agents(&app)?;
        let record = records
            .iter()
            .find(|record| record.pubkey == pubkey)
            .ok_or_else(|| format!("agent {pubkey} not found"))?;

        // Keyed by (pubkey, relay_url), so match on the pubkey half: the same
        // identity can have a runtime under whichever relay it was started
        // against, and the question here is only "is anything running".
        let is_running = state
            .managed_agent_processes
            .lock()
            .map(|runtimes| runtimes.keys().any(|key| key.pubkey == pubkey))
            .unwrap_or(false);

        // A managed instance is tied to a team through the definition it was
        // deployed from (`persona_id`) or the team it was deployed for
        // (`team_id`) — there is no pubkey list on a team to consult.
        let team_names = load_teams(&app)
            .unwrap_or_default()
            .into_iter()
            .filter(|team| {
                let by_definition = record
                    .persona_id
                    .as_deref()
                    .is_some_and(|id| team.persona_ids.iter().any(|member| member == id));
                let by_deployment = record.team_id.as_deref() == Some(team.id.as_str());
                by_definition || by_deployment
            })
            .map(|team| team.name)
            .collect();

        Ok(ManagedAgentRemovalPreview {
            pubkey: record.pubkey.clone(),
            name: record.name.clone(),
            is_running,
            team_names,
            has_local_key: !record.private_key_nsec.is_empty(),
        })
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

/// Hide an agent Buzz owns without destroying it.
///
/// Stops any local process, then moves the record out of `managed-agents.json`
/// and into the archived store. The identity, key, and definition are all
/// retained, so this is fully reversible by [`restore_archived_agent`].
///
/// Deliberately publishes nothing. A tombstone or NIP-IA archive would tell the
/// whole relay the agent is gone, which is not true yet and is not undoable by
/// restoring a local record.
#[tauri::command]
pub async fn archive_managed_agent(
    pubkey: String,
    app: AppHandle,
) -> Result<ArchivedAgentSummary, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let summary = {
            let _store_guard = state
                .managed_agents_store_lock
                .lock()
                .map_err(|error| error.to_string())?;

            let mut records = load_managed_agents(&app)?;
            let index = records
                .iter()
                .position(|record| record.pubkey == pubkey)
                .ok_or_else(|| format!("agent {pubkey} not found"))?;

            // Stop before moving: a running process whose record has already
            // left the store cannot be found by any later stop path.
            {
                let mut runtimes = state
                    .managed_agent_processes
                    .lock()
                    .map_err(|error| error.to_string())?;
                stop_managed_agent_process(&app, &mut records[index], &mut runtimes)?;
            }
            state.clear_agent_session_caches(&pubkey);

            let agent = records.remove(index);
            let mut archived = load_archived_agents(&app)?;
            // Re-archiving an agent that is somehow already archived replaces the
            // stale row rather than creating a second one, so a restore has a
            // single answer.
            archived.retain(|record| record.agent.pubkey != pubkey);
            let record = ArchivedAgentRecord {
                archived_at: now_iso(),
                agent,
            };
            let summary = ArchivedAgentSummary::from(&record);
            archived.push(record);

            // Archived store first. If this write fails the agent stays exactly
            // where it was; the reverse order could drop it from both stores.
            save_archived_agents(&app, &archived)?;
            save_managed_agents(&app, &records)?;
            summary
        };
        crate::managed_agents::try_regenerate_nest(&app);
        Ok(summary)
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

/// The agents currently archived on this machine.
#[tauri::command]
pub async fn list_archived_agents(app: AppHandle) -> Result<Vec<ArchivedAgentSummary>, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;
        Ok(load_archived_agents(&app)?
            .iter()
            .map(ArchivedAgentSummary::from)
            .collect())
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

/// Move an archived agent back into normal use, intact.
///
/// Refuses on a name or pubkey collision rather than resurrecting a duplicate:
/// names are how agents are mentioned, and two records for one identity would be
/// two answers to "who is this pubkey".
#[tauri::command]
pub async fn restore_archived_agent(pubkey: String, app: AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        {
            let _store_guard = state
                .managed_agents_store_lock
                .lock()
                .map_err(|error| error.to_string())?;

            let mut archived = load_archived_agents(&app)?;
            let index = archived
                .iter()
                .position(|record| record.agent.pubkey == pubkey)
                .ok_or_else(|| format!("archived agent {pubkey} not found"))?;

            let mut records = load_managed_agents(&app)?;
            let candidate = &archived[index].agent;
            if records
                .iter()
                .any(|record| record.pubkey == candidate.pubkey)
            {
                return Err(format!(
                    "an agent with this identity is already active — restore would create a \
                     second record for {pubkey}"
                ));
            }
            if let Some(clash) = records
                .iter()
                .find(|record| record.name.eq_ignore_ascii_case(&candidate.name))
            {
                return Err(format!(
                    "an agent named '{}' already exists — rename it before restoring this one, \
                     because names are how agents are mentioned",
                    clash.name
                ));
            }

            let restored = archived.remove(index).agent;
            records.push(restored);

            // Managed store first here: the mirror of archiving. A failure leaves
            // the agent archived rather than absent from both stores.
            save_managed_agents(&app, &records)?;
            save_archived_agents(&app, &archived)?;
        }
        crate::managed_agents::try_regenerate_nest(&app);
        Ok(())
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

/// Destroy an archived agent's identity. Irreversible.
///
/// Only reachable from the archived state, which is the gate: an agent cannot go
/// from visible to key-destroyed in one action. Removes the archived record, the
/// keyring entry, and publishes the kind:30177 tombstone and NIP-IA archive that
/// take the identity out of relay pickers.
///
/// Cannot erase already-published signed events, relay audit history, or copies
/// other clients hold. The caller must say so.
#[tauri::command]
pub async fn permanently_delete_archived_agent(
    pubkey: String,
    app: AppHandle,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        {
            let _store_guard = state
                .managed_agents_store_lock
                .lock()
                .map_err(|error| error.to_string())?;

            let mut archived = load_archived_agents(&app)?;
            let before = archived.len();
            archived.retain(|record| record.agent.pubkey != pubkey);
            if archived.len() == before {
                // Refusing an unarchived pubkey is the gate itself, not a
                // convenience check: it is what makes "archive first" impossible
                // to bypass from an IPC caller.
                return Err(format!(
                    "agent {pubkey} is not archived. Remove it from My Agents first — permanent \
                     deletion is only available from the archived state."
                ));
            }
            save_archived_agents(&app, &archived)?;

            crate::managed_agents::delete_agent_key(&pubkey);
            super::agents::tombstone_managed_agent_pending(&app, &state, &pubkey);
            super::agents::archive_managed_agent_pending(&app, &state, &pubkey);
        }
        crate::managed_agents::try_regenerate_nest(&app);
        Ok(())
    })
    .await
    .map_err(|error| format!("spawn_blocking failed: {error}"))?
}

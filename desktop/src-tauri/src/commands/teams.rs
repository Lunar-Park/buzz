use tauri::AppHandle;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    managed_agents::{
        delete_team_with_cascade, ensure_persona_ids_are_active, load_connected_agents,
        load_personas, load_teams, save_teams, try_regenerate_nest, CreateTeamRequest, TeamRecord,
        UpdateTeamRequest,
    },
    util::now_iso,
};

fn trim_required(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }
    Ok(trimmed.to_string())
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|candidate| {
        let trimmed = candidate.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn validate_connected_agent_pubkeys(
    requested: Vec<String>,
    connected: &[crate::managed_agents::ConnectedAgentRecord],
) -> Result<Vec<String>, String> {
    let available = connected
        .iter()
        .map(|agent| agent.pubkey.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut normalized = Vec::with_capacity(requested.len());
    let mut seen = std::collections::HashSet::new();

    for pubkey in requested {
        let pubkey = pubkey.trim().to_lowercase();
        if pubkey.len() != 64 || !pubkey.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("Connected agent public key is invalid: {pubkey}"));
        }
        if !available.contains(pubkey.as_str()) {
            return Err(format!(
                "Connected agent {pubkey} is not connected on this device."
            ));
        }
        if seen.insert(pubkey.clone()) {
            normalized.push(pubkey);
        }
    }

    Ok(normalized)
}

/// Retain a freshly authored team event in the local store, flagged for relay
/// sync. Called inside a command's `managed_agents_store_lock`-held body after
/// `save_teams`; the background flush loop publishes it out-of-band.
///
/// Mirrors `commands::personas::retain_persona_pending`. Built-in teams are not
/// owner-authored, so the caller skips them — this helper assumes the team is
/// publishable. Best-effort: a failure here is logged and swallowed so a
/// retention hiccup never blocks the disk-authoritative write.
///
/// Unlike `retain_managed_agent_pending`, this has no projection-equality
/// short-circuit: teams have no start/stop runtime churn, so a republish only
/// happens on an actual user edit. The guard is intentionally omitted.
pub(super) fn retain_team_pending(app: &AppHandle, state: &AppState, team: &TeamRecord) {
    use crate::managed_agents::{
        persona_events::monotonic_created_at,
        retention::{get_retained_event, open_retention_db, retain_event, RetainedEvent},
        team_events::build_team_event,
    };
    use buzz_core_pkg::kind::KIND_TEAM;
    use nostr::JsonUtil;

    let result = (|| -> Result<(), String> {
        let scope = crate::managed_agents::retention::active_retention_scope(app, state)?;
        let conn = open_retention_db(&scope.db_path)?;
        let pubkey = scope.owner_keys.public_key().to_hex();
        // Monotonic created_at: bump past the retained head (NIP-AP step 3).
        let prior =
            get_retained_event(&conn, KIND_TEAM, &pubkey, &team.id)?.map(|row| row.created_at);
        let event = build_team_event(team)?
            .custom_created_at(monotonic_created_at(prior))
            .sign_with_keys(&scope.owner_keys)
            .map_err(|e| format!("failed to sign team event: {e}"))?;
        retain_event(
            &conn,
            &RetainedEvent {
                kind: KIND_TEAM,
                pubkey,
                d_tag: team.id.clone(),
                content: event.content.to_string(),
                created_at: event.created_at.as_secs() as i64,
                raw_event: event.as_json(),
                pending_sync: true,
            },
        )
    })();
    if let Err(e) = result {
        eprintln!("buzz-desktop: team-retain: {e}");
    }
}

/// Purge a deleted team's pending row and enqueue a NIP-09 tombstone, both
/// inside the `managed_agents_store_lock`-held delete body.
///
/// Mirrors `commands::personas::tombstone_persona_pending`: the team row is
/// purged first so an unpublished edit can never resurrect it after the
/// tombstone publishes, then the kind:5 tombstone is retained at its own
/// `(5, pubkey, d_tag)` coordinate with `pending_sync = 1`. Best-effort: a
/// failure is logged and swallowed so a retention hiccup never blocks the
/// disk-authoritative delete.
fn tombstone_team_pending(app: &AppHandle, state: &AppState, d_tag: &str) {
    use crate::managed_agents::{
        retention::{
            delete_retained_event, open_retention_db, retain_event, tombstone_retention_d_tag,
            RetainedEvent,
        },
        team_events::build_team_delete,
    };
    use buzz_core_pkg::kind::KIND_TEAM;
    use nostr::JsonUtil;

    const KIND_DELETE: u32 = 5;

    let result = (|| -> Result<(), String> {
        let scope = crate::managed_agents::retention::active_retention_scope(app, state)?;
        let pubkey = scope.owner_keys.public_key().to_hex();
        let event = build_team_delete(d_tag, &pubkey)?
            .sign_with_keys(&scope.owner_keys)
            .map_err(|e| format!("failed to sign team tombstone: {e}"))?;
        let conn = open_retention_db(&scope.db_path)?;
        delete_retained_event(&conn, KIND_TEAM, &pubkey, d_tag)?;
        retain_event(
            &conn,
            &RetainedEvent {
                kind: KIND_DELETE,
                pubkey,
                // Key by the target coordinate so cross-kind d-tag tombstones
                // occupy distinct rows (F2c).
                d_tag: tombstone_retention_d_tag(KIND_TEAM, d_tag),
                content: event.content.to_string(),
                created_at: event.created_at.as_secs() as i64,
                raw_event: event.as_json(),
                pending_sync: true,
            },
        )
    })();
    if let Err(e) = result {
        eprintln!("buzz-desktop: team-tombstone: {e}");
    }
}

#[tauri::command]
pub async fn list_teams(app: AppHandle) -> Result<Vec<TeamRecord>, String> {
    use tauri::Manager;
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;
        load_teams(&app)
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

#[tauri::command]
pub async fn create_team(input: CreateTeamRequest, app: AppHandle) -> Result<TeamRecord, String> {
    use tauri::Manager;
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let name = trim_required(&input.name, "Team name")?;
        let description = trim_optional(input.description);
        let instructions = trim_optional(input.instructions);
        let now = now_iso();

        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;
        let personas = load_personas(&app)?;
        ensure_persona_ids_are_active(&personas, &input.persona_ids)?;
        let connected_agent_pubkeys = validate_connected_agent_pubkeys(
            input.connected_agent_pubkeys,
            &load_connected_agents(&app)?,
        )?;
        let mut teams = load_teams(&app)?;
        let team = TeamRecord {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            instructions,
            persona_ids: input.persona_ids,
            connected_agent_pubkeys,
            is_builtin: false,
            source_dir: None,
            is_symlink: false,
            symlink_target: None,
            version: None,
            created_at: now.clone(),
            updated_at: now,
        };
        teams.push(team.clone());
        save_teams(&app, &teams)?;
        // Created teams are always non-builtin; publish to the relay.
        retain_team_pending(&app, &state, &team);
        Ok(team)
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

#[tauri::command]
pub async fn update_team(input: UpdateTeamRequest, app: AppHandle) -> Result<TeamRecord, String> {
    use tauri::Manager;
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let name = trim_required(&input.name, "Team name")?;
        let description = trim_optional(input.description);
        let instructions = trim_optional(input.instructions);

        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;
        let personas = load_personas(&app)?;
        ensure_persona_ids_are_active(&personas, &input.persona_ids)?;
        let connected_agent_pubkeys = validate_connected_agent_pubkeys(
            input.connected_agent_pubkeys,
            &load_connected_agents(&app)?,
        )?;
        let mut teams = load_teams(&app)?;
        let team = teams
            .iter_mut()
            .find(|record| record.id == input.id)
            .ok_or_else(|| format!("team {} not found", input.id))?;

        team.name = name;
        team.description = description;
        team.instructions = instructions;
        team.persona_ids = input.persona_ids;
        team.connected_agent_pubkeys = connected_agent_pubkeys;
        team.updated_at = now_iso();

        let updated = team.clone();
        save_teams(&app, &teams)?;
        // Built-in teams are not owner-authored — never publish them.
        if !updated.is_builtin {
            retain_team_pending(&app, &state, &updated);
        }
        Ok(updated)
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

#[tauri::command]
pub async fn delete_team(id: String, app: AppHandle) -> Result<(), String> {
    use tauri::Manager;
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _store_guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|error| error.to_string())?;
        let cascaded_persona_d_tags = delete_team_with_cascade(&app, &id)?;
        // delete_team_with_cascade rejects built-in teams via validate_team_deletion,
        // so reaching here means this team was owner-published — tombstone it. The
        // d_tag is the team id, captured before the record left the store.
        tombstone_team_pending(&app, &state, &id);
        // Tombstone the cascaded personas too, so their orphaned kind:30175 heads
        // don't linger on the relay (F4). Each d-tag was captured pre-removal.
        for persona_d_tag in &cascaded_persona_d_tags {
            super::personas::tombstone_persona_pending(&app, &state, persona_d_tag);
        }
        try_regenerate_nest(&app);
        Ok(())
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::validate_connected_agent_pubkeys;
    use crate::managed_agents::ConnectedAgentRecord;

    const SELENE: &str = "4687f50de3a9e235e28eb58d68b0746062d7be6401bbf78a766bbd6f96ffe3c9";

    fn connected(pubkey: &str) -> ConnectedAgentRecord {
        ConnectedAgentRecord {
            pubkey: pubkey.to_string(),
            name: "Selene".to_string(),
            host: "lunar01".to_string(),
            harness: Some("openclaw".to_string()),
            created_at: "2026-07-29T00:00:00Z".to_string(),
            updated_at: "2026-07-29T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn connected_team_members_are_normalized_and_deduplicated() {
        let normalized = validate_connected_agent_pubkeys(
            vec![SELENE.to_uppercase(), SELENE.to_string()],
            &[connected(SELENE)],
        )
        .unwrap();

        assert_eq!(normalized, vec![SELENE.to_string()]);
    }

    #[test]
    fn connected_team_members_must_be_configured_on_this_device() {
        let error = validate_connected_agent_pubkeys(vec![SELENE.to_string()], &[]).unwrap_err();

        assert!(error.contains("not connected on this device"));
    }
}

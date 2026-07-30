//! Archived Buzz-managed agents: stage one of a two-stage removal.
//!
//! Removing an agent Buzz owns used to be one irreversible action — it stopped
//! the process, dropped the record, deleted the key from the keyring, and
//! published a tombstone. There was no way back, and no way to find out what you
//! were about to lose. The specification splits that in two: a reversible
//! `Remove from My Agents`, then a separately gated
//! `Permanently delete identity`.
//!
//! # Why archiving moves the record instead of flagging it
//!
//! An `archived` flag on [`ManagedAgentRecord`] would need every reader to
//! respect it, and `load_managed_agents` has 61 call sites. One missed filter and
//! an archived agent gets auto-started, redeployed, or republished — which is
//! precisely the failure mode that made connected agents their own store rather
//! than a custody field. Moving the record out of `managed-agents.json` makes
//! invisibility structural: no lifecycle path can act on a record it cannot load,
//! and none of those 61 sites needed changing.
//!
//! # Why the archive metadata is a wrapper
//!
//! `archived_at` lives on [`ArchivedAgentRecord`], not on the agent. Adding a
//! field to `ManagedAgentRecord` would oblige every one of its ~20 struct
//! literals in the tree to name a concept that only applies while archived —
//! the same tax the retired `KeyCustody` design paid.
//!
//! # What archiving deliberately does not do
//!
//! No keyring delete, no kind:30177 tombstone, and no NIP-IA archive. Those are
//! what make removal irreversible and visible to the whole relay, so they belong
//! to the second stage. An archived agent keeps its identity, its key, and its
//! definition, and can be restored intact.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::storage::{atomic_write_json_restricted, backup_invalid_store, managed_agents_base_dir};
use super::ManagedAgentRecord;

/// An agent Buzz owns, hidden from normal surfaces but fully recoverable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchivedAgentRecord {
    /// When the agent was archived, ISO-8601.
    pub archived_at: String,
    /// The agent exactly as it was, so a restore is a move rather than a rebuild.
    pub agent: ManagedAgentRecord,
}

/// The archived-agent view handed to the frontend.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedAgentSummary {
    pub pubkey: String,
    pub name: String,
    pub archived_at: String,
    /// Whether this row still represents a recoverable Buzz identity.
    ///
    /// Archiving retains the key by construction, so this is true for every
    /// archived instance and false only for a key-less definition row. It is
    /// deliberately **not** a keyring liveness probe: archived records load
    /// without key hydration (a hidden agent must not prompt for a secret), so
    /// inspecting `private_key_nsec` here would report false negatives for every
    /// agent whose key is safely in the keyring — which is all of them.
    pub retains_identity: bool,
}

impl From<&ArchivedAgentRecord> for ArchivedAgentSummary {
    fn from(record: &ArchivedAgentRecord) -> Self {
        Self {
            pubkey: record.agent.pubkey.clone(),
            name: record.agent.name.clone(),
            archived_at: record.archived_at.clone(),
            retains_identity: !record.agent.pubkey.is_empty(),
        }
    }
}

/// Path of the archived-agent store, beside `managed-agents.json`.
pub(crate) fn archived_agents_store_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(managed_agents_base_dir(app)?.join("archived-agents.json"))
}

pub(crate) fn load_archived_agents(app: &AppHandle) -> Result<Vec<ArchivedAgentRecord>, String> {
    load_archived_agents_at(&archived_agents_store_path(app)?)
}

/// Path-based seam so the store is testable over a tempdir, mirroring the
/// connected-agent store.
///
/// Deliberately no key hydration. An archived agent is not going to be spawned,
/// so reaching into the keyring for its secret would prompt or fail for no
/// benefit — and a keyring outage must not make the archived list unreadable,
/// which is the one place a user goes to recover an agent.
pub(crate) fn load_archived_agents_at(path: &Path) -> Result<Vec<ArchivedAgentRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read archived agent store: {error}"))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw).map_err(|error| {
        // Fail loud with the evidence kept, matching the other two stores: a
        // later save rewrites this file wholesale, and swallowing a parse error
        // into an empty list would silently destroy every recoverable agent.
        backup_invalid_store(path);
        format!("failed to parse archived agent store (preserved as .invalid): {error}")
    })
}

pub(crate) fn save_archived_agents(
    app: &AppHandle,
    records: &[ArchivedAgentRecord],
) -> Result<(), String> {
    save_archived_agents_at(&archived_agents_store_path(app)?, records)
}

/// Write the archived store with owner-only permissions.
///
/// Restricted, unlike the connected store: an archived record is a full
/// `ManagedAgentRecord`, and when the keyring is unreachable that type keeps its
/// `private_key_nsec` inline. The connected store can never contain a secret by
/// construction; this one can, so it gets the stricter mode.
pub(crate) fn save_archived_agents_at(
    path: &Path,
    records: &[ArchivedAgentRecord],
) -> Result<(), String> {
    let mut sorted = records.to_vec();
    sorted.sort_by(|left, right| {
        left.agent
            .name
            .to_lowercase()
            .cmp(&right.agent.name.to_lowercase())
            .then_with(|| left.agent.pubkey.cmp(&right.agent.pubkey))
    });
    let payload = serde_json::to_vec_pretty(&sorted)
        .map_err(|error| format!("failed to serialize archived agents: {error}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create agent store directory: {error}"))?;
    }
    if !path.exists() {
        std::fs::File::create(path)
            .map_err(|error| format!("failed to create archived agent store: {error}"))?;
    }
    atomic_write_json_restricted(path, &payload)
}

#[cfg(test)]
#[path = "archived_agents_tests.rs"]
mod tests;

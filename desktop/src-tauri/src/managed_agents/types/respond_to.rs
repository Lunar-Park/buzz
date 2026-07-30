// ── Inbound author gate ──────────────────────────────────────────────────────
//
// Mirrors `buzz-acp`'s `--respond-to` CLI flag and the related
// `--respond-to-allowlist` option. Persisted per agent so the desktop can
// translate the user's choice into `BUZZ_ACP_RESPOND_TO` /
// `BUZZ_ACP_RESPOND_TO_ALLOWLIST` env vars at spawn time.
//
// Wire format is kebab-case (`owner-only`, `allowlist`, `anyone`) to match
// the harness CLI vocabulary and the strings the GUI emits.
//
// `nobody` is a heartbeat-only mode — the agent never responds to messages
// but can still run heartbeats for self-prompted tasks.

use serde::{Deserialize, Serialize};

/// Who the agent should respond to. Defaults to `Nobody` so agents only act
/// when explicitly triggered (no auto-firing on messages).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RespondTo {
    #[default]
    Nobody,
    OwnerOnly,
    Allowlist,
    Anyone,
}

impl RespondTo {
    /// CLI/env wire string (matches `buzz-acp`'s `--respond-to`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nobody => "nobody",
            Self::OwnerOnly => "owner-only",
            Self::Allowlist => "allowlist",
            Self::Anyone => "anyone",
        }
    }

    /// Parse the NIP-AP wire string. Definitions carry `respond_to` as
    /// opaque data everywhere else; this is the single parse boundary
    /// (instance mint), and an unrecognized mode fails LOUDLY here rather
    /// than silently defaulting — a typo'd definition must not mint an
    /// agent with a different audience than its author intended.
    pub fn parse_wire(value: &str) -> Result<Self, String> {
        match value {
            "nobody" => Ok(Self::Nobody),
            "owner-only" => Ok(Self::OwnerOnly),
            "allowlist" => Ok(Self::Allowlist),
            "anyone" => Ok(Self::Anyone),
            other => Err(format!(
                "definition respond_to '{other}' is not a recognized mode (expected 'nobody', 'owner-only', 'allowlist', or 'anyone')"
            )),
        }
    }
}

/// Validate and normalize a respond-to allowlist.
///
/// Rules mirror `buzz-acp/src/config.rs::validate_allowlist`:
/// - Each entry is exactly 64 hex chars (any case in, lowercase out).
/// - Duplicates removed, insertion order preserved.
///
/// Empty input is allowed here — the boundary check (allowlist mode requires
/// at least one entry) is the caller's job, because an `UpdateManagedAgentRequest`
/// may want to validate a list without yet knowing the final mode.
pub fn validate_respond_to_allowlist(input: &[String]) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(input.len());
    for entry in input {
        let trimmed = entry.trim();
        if trimmed.len() != 64 || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "invalid pubkey in respond-to allowlist: '{trimmed}' (must be 64 hex chars)"
            ));
        }
        let lower = trimmed.to_ascii_lowercase();
        if seen.insert(lower.clone()) {
            out.push(lower);
        }
    }
    Ok(out)
}

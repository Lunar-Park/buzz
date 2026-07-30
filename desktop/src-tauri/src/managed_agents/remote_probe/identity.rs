//! Resolving and generating a resident agent's Buzz identity on its own host.
//!
//! Connecting an agent used to mean pasting an npub the user had to go and find.
//! This closes that gap from both ends: read the identity the harness is already
//! configured to sign as, and — only on an explicit confirmed action — mint a
//! fresh one **on the host**, where the secret stays.
//!
//! # The invariant that shapes everything here
//!
//! No private key returns to Desktop. Generation runs `buzz keys generate --out`,
//! which writes the secret to a mode-`0600` file on the host and prints only the
//! public half plus the path. Buzz never passes `--stdout`, never reads the file,
//! and never receives the secret over the ssh channel. A secret in Desktop's
//! memory would defeat the entire point of a *connected* agent.
//!
//! # Why generation cannot overwrite
//!
//! `buzz keys generate` refuses an existing destination unless `--force`, and
//! this module never passes it. Re-running generation for an agent that already
//! has a key therefore fails loudly instead of destroying a live identity — the
//! failure a connect wizard is most likely to cause and least likely to notice.
//!
//! # The one place user-influenced text reaches a remote shell
//!
//! Every other remote command in this tree is a compile-time literal. Generation
//! needs a per-agent filename, so [`validate_identity_slug`] gates it against a
//! strict charset and *refuses* anything outside it rather than trying to escape
//! it. Rejecting is checkable; escaping is a thing you get wrong once.

use std::process::Command;
use std::time::Duration;

use serde::Serialize;

use super::{
    classify_ssh_failure, failure_message, first_line, ssh_probe_args, wait_with_timeout,
    HostProbeErrorKind,
};
use crate::managed_agents::ssh_config::{resolve_ssh_binary, SshHost};

/// Wall-clock ceiling for an identity read or mint. Both are single commands
/// against a host whose reachability is already known.
const IDENTITY_TIMEOUT: Duration = Duration::from_secs(15);

const IDENTITY_START: &str = "__BUZZ_IDENTITY_START__";
const IDENTITY_END: &str = "__BUZZ_IDENTITY_END__";

/// Longest accepted identity slug. Comfortably longer than any harness agent id
/// while keeping the derived path short enough to read in an error message.
const MAX_SLUG_LEN: usize = 64;

/// The Buzz identity a harness is configured to sign as.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostIdentityResolution {
    pub host: String,
    pub harness_id: String,
    pub ok: bool,
    /// False when Buzz has no recipe for reading this harness's identity. Not a
    /// failure — manual entry is the intended path.
    pub supported: bool,
    /// The configured agent pubkey, lowercase hex, when the harness has one.
    ///
    /// `None` with `ok: true` is the meaningful "no identity yet" answer, and it
    /// is what makes offering generation honest rather than speculative.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<HostProbeErrorKind>,
}

/// A freshly minted host-side identity. Public half only, by construction.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedHostIdentity {
    pub host: String,
    pub pubkey: String,
    pub npub: String,
    /// Where the secret was written on the host, so the user can point the
    /// harness at it. Buzz never reads this file.
    pub secret_key_path: String,
}

/// How to read one harness's configured Buzz identity.
struct IdentityRecipe {
    harness_id: &'static str,
    /// Remote command. A literal with no interpolation, single-quote-free.
    command: &'static str,
}

/// OpenClaw stores the Buzz account's expected agent pubkey in its channel
/// config, and the plugin verifies its loaded signing key against that value at
/// startup. Reading it therefore answers the question the specification actually
/// asks — which identity will this adapter sign as — rather than guessing from a
/// shell environment that has no reason to hold an agent's key.
const IDENTITY_RECIPES: &[IdentityRecipe] = &[IdentityRecipe {
    harness_id: "openclaw",
    command: "openclaw config get channels.buzz.agentPubkey",
}];

fn recipe_for(harness_id: &str) -> Option<&'static IdentityRecipe> {
    IDENTITY_RECIPES
        .iter()
        .find(|recipe| recipe.harness_id == harness_id)
}

/// Accept a slug safe to embed in a remote filename, or explain the refusal.
///
/// Deliberately a whitelist. The value becomes part of a path inside a remote
/// shell command, and the set of characters that are harmless there is much
/// smaller and easier to state than the set that needs escaping. Harness agent
/// ids observed in practice (`main`, `astra`, `cassian`) sit well inside it.
pub fn validate_identity_slug(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("an agent id is required to name the key file".to_string());
    }
    if trimmed.len() > MAX_SLUG_LEN {
        return Err(format!(
            "agent id must be at most {MAX_SLUG_LEN} characters"
        ));
    }
    let mut chars = trimmed.chars();
    let first = chars.next().unwrap_or_default();
    if !first.is_ascii_alphanumeric() {
        return Err("agent id must start with a letter or digit".to_string());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(
            "agent id may contain only letters, digits, dot, dash, and underscore".to_string(),
        );
    }
    // `..` would climb out of the intended directory even though every
    // individual character is allowed.
    if trimmed.contains("..") {
        return Err("agent id must not contain '..'".to_string());
    }
    Ok(trimmed.to_string())
}

/// Where a generated secret goes on the host.
///
/// Under the user's home rather than a harness directory: Buzz does not know a
/// harness's layout well enough to write inside it, and the file has to outlive
/// any harness reinstall.
fn secret_path_for(slug: &str) -> String {
    format!("~/.buzz/agents/{slug}.nsec")
}

fn build_read_command(recipe: &IdentityRecipe) -> String {
    let inner = format!(
        "echo {IDENTITY_START}; {} 2>/dev/null; echo {IDENTITY_END}",
        recipe.command
    );
    format!("exec $SHELL -lc '{inner}'")
}

/// Build the generation command for a validated slug.
///
/// `mkdir -p` first because `buzz keys generate` creates the file, not its
/// parent. `--force` is deliberately absent: an existing key must abort the mint.
fn build_generate_command(slug: &str) -> String {
    let path = secret_path_for(slug);
    let inner = format!(
        "mkdir -p ~/.buzz/agents && chmod 700 ~/.buzz/agents && echo {IDENTITY_START}; \
         buzz keys generate --out {path}; echo {IDENTITY_END}"
    );
    format!("exec $SHELL -lc '{inner}'")
}

fn extract_payload(stdout: &str) -> Option<&str> {
    let start = stdout.find(IDENTITY_START)? + IDENTITY_START.len();
    let rest = &stdout[start..];
    let end = rest.find(IDENTITY_END)?;
    Some(rest[..end].trim())
}

/// Interpret a harness's answer to "what is your configured Buzz identity".
///
/// An unset config path is a normal answer, not an error: OpenClaw reports it as
/// a `Config path not found` line, and treating that as a failure would hide the
/// case where offering generation is exactly right.
pub(crate) fn parse_resolved_pubkey(payload: &str) -> Result<Option<String>, String> {
    let value = payload.trim();
    if value.is_empty() || value.starts_with("Config path not found") {
        return Ok(None);
    }
    let candidate = value.lines().next().unwrap_or_default().trim();
    let normalized = candidate.trim_matches('"').to_ascii_lowercase();
    if normalized.len() == 64 && normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(Some(normalized));
    }
    Err(format!(
        "the harness reported an identity Buzz cannot read as a pubkey: {}",
        first_line(candidate)
    ))
}

/// Parse `buzz keys generate` output into its public half.
pub(crate) fn parse_generated_identity(payload: &str) -> Result<(String, String, String), String> {
    let line = payload
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with('{'))
        .ok_or_else(|| {
            format!(
                "could not find the generated identity in the host's reply: {}",
                first_line(payload)
            )
        })?;
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|error| format!("could not parse the generated identity: {error}"))?;

    // A secret must never travel back. If a future CLI ever prints one by
    // default, refuse the whole result rather than quietly discarding it — the
    // key would already be in this process's memory and in the ssh transcript.
    if value.get("nsec").is_some() {
        return Err(
            "the host returned a secret key, which must never leave that machine. \
             Aborting instead of storing it."
                .to_string(),
        );
    }

    let pubkey = value
        .get("pubkey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "the host's reply had no pubkey".to_string())?
        .to_ascii_lowercase();
    if pubkey.len() != 64 || !pubkey.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("the host returned a malformed pubkey".to_string());
    }
    let npub = value
        .get("npub")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let path = value
        .get("secret_key_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "the host did not report where it wrote the key, so Buzz cannot tell you \
             where to point the harness"
                .to_string()
        })?
        .to_string();
    Ok((pubkey, npub, path))
}

/// Read the Buzz identity `harness_id` is configured to sign as. Read-only.
pub fn resolve_ssh_host_identity(host: &SshHost, harness_id: &str) -> HostIdentityResolution {
    let base = |ok: bool, supported: bool| HostIdentityResolution {
        host: host.host.clone(),
        harness_id: harness_id.to_string(),
        ok,
        supported,
        pubkey: None,
        error: None,
        error_kind: None,
    };

    let Some(recipe) = recipe_for(harness_id) else {
        return HostIdentityResolution {
            error: Some(format!(
                "Buzz cannot read a '{harness_id}' harness's Buzz identity yet. Enter the \
                 agent's public key manually."
            )),
            ..base(false, false)
        };
    };

    let mut command = Command::new(resolve_ssh_binary());
    command
        .args(ssh_probe_args(host))
        .arg(build_read_command(recipe));
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = match wait_with_timeout(command, IDENTITY_TIMEOUT) {
        Ok(Some(output)) => output,
        Ok(None) => {
            let kind = HostProbeErrorKind::TimedOut;
            return HostIdentityResolution {
                error: Some(failure_message(&kind, &host.host, "")),
                error_kind: Some(kind),
                ..base(false, true)
            };
        }
        Err(error) => {
            return HostIdentityResolution {
                error: Some(format!(
                    "could not read the agent identity on '{}': {error}",
                    host.host
                )),
                ..base(false, true)
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let Some(payload) = extract_payload(&stdout) else {
        let kind = if stdout.contains(IDENTITY_START) {
            HostProbeErrorKind::Truncated
        } else {
            classify_ssh_failure(&stderr).unwrap_or(HostProbeErrorKind::Truncated)
        };
        return HostIdentityResolution {
            error: Some(failure_message(&kind, &host.host, &stderr)),
            error_kind: Some(kind),
            ..base(false, true)
        };
    };

    match parse_resolved_pubkey(payload) {
        Ok(pubkey) => HostIdentityResolution {
            pubkey,
            ..base(true, true)
        },
        Err(error) => HostIdentityResolution {
            error: Some(error),
            ..base(false, true)
        },
    }
}

/// Mint a fresh Buzz identity on `host`, keeping the secret there.
///
/// This **writes to the host** and must only run from an explicitly confirmed
/// user action that names the machine being changed.
pub fn generate_ssh_host_identity(
    host: &SshHost,
    slug: &str,
) -> Result<GeneratedHostIdentity, String> {
    let slug = validate_identity_slug(slug)?;

    let mut command = Command::new(resolve_ssh_binary());
    command
        .args(ssh_probe_args(host))
        .arg(build_generate_command(&slug));
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = wait_with_timeout(command, IDENTITY_TIMEOUT)
        .map_err(|error| format!("could not run key generation on '{}': {error}", host.host))?
        .ok_or_else(|| failure_message(&HostProbeErrorKind::TimedOut, &host.host, ""))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let payload = extract_payload(&stdout).ok_or_else(|| {
        let kind = if stdout.contains(IDENTITY_START) {
            HostProbeErrorKind::Truncated
        } else {
            classify_ssh_failure(&stderr).unwrap_or(HostProbeErrorKind::Truncated)
        };
        failure_message(&kind, &host.host, &stderr)
    })?;

    if payload.is_empty() {
        // The most likely cause by far, and the one worth naming: without the
        // CLI there is nothing on the host that can mint a Buzz identity.
        return Err(format!(
            "key generation produced no output on '{}'. Is the buzz CLI installed there? {}",
            host.host,
            first_line(&stderr)
        ));
    }

    let (pubkey, npub, secret_key_path) = parse_generated_identity(payload)
        .map_err(|error| format!("{error} ({})", first_line(&stderr)))?;

    Ok(GeneratedHostIdentity {
        host: host.host.clone(),
        pubkey,
        npub,
        secret_key_path,
    })
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;

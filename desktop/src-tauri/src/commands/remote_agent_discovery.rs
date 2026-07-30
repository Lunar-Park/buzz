//! Tauri commands for host-aware harness discovery.
//!
//! These answer "which machines can I reach, and what agent harnesses are on
//! them?" so an agent that already runs on another host can be found instead of
//! described by hand.
//!
//! All three commands are read-only. Nothing here installs software, writes to
//! a remote host, or collects a credential — the probe runs `command -v` and
//! `--version` and nothing else.

use crate::managed_agents::remote_probe::{
    probe_local_harness_agents, probe_localhost, probe_ssh_harness_agents, probe_ssh_host,
    HarnessRosterResult, HostProbeResult,
};
use crate::managed_agents::ssh_config::{parse_ssh_config, SshHost};

/// Enumerate the user's `~/.ssh/config` host aliases.
///
/// No connection is attempted. An absent config yields an empty list, which
/// means "no remote hosts configured", not a failure.
#[tauri::command]
pub async fn list_ssh_hosts() -> Result<Vec<SshHost>, String> {
    tokio::task::spawn_blocking(parse_ssh_config)
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))
}

/// Probe one host for agent harnesses and the `buzz` CLI.
///
/// `host` must name an alias present in `~/.ssh/config`. Resolving it through
/// the parsed config rather than trusting the argument is what keeps an
/// arbitrary string — including anything shaped like an ssh option — from
/// reaching the `ssh` argv.
///
/// A host-side problem (unreachable, password-only, unknown host key) comes back
/// as `Ok` with `ok: false` and a classified `errorKind`: the UI shows one row
/// per host and needs a renderable status, not an exception.
#[tauri::command]
pub async fn probe_agent_host(host: String) -> Result<HostProbeResult, String> {
    tokio::task::spawn_blocking(move || {
        let hosts = parse_ssh_config();
        let Some(entry) = hosts.into_iter().find(|candidate| candidate.host == host) else {
            return Err(format!(
                "'{host}' is not a Host alias in ~/.ssh/config; only configured hosts can be probed"
            ));
        };
        Ok(probe_ssh_host(&entry))
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

/// Probe the machine Buzz is running on, using the identical probe script so
/// the result is shape-compatible with [`probe_agent_host`].
#[tauri::command]
pub async fn probe_local_agent_host() -> Result<HostProbeResult, String> {
    tokio::task::spawn_blocking(probe_localhost)
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))
}

/// List the durable, named agents one harness holds on a host.
///
/// The step after [`probe_agent_host`]: discovery says a harness is present,
/// this says which agents it contains and which is its primary, so the user can
/// enroll one, several, or none rather than the whole stack.
///
/// `host` is resolved through `~/.ssh/config` for the same reason as the host
/// probe — the alias carries the user's own `User`, `IdentityFile`, and
/// `ProxyJump`, and re-resolving keeps an arbitrary string out of the `ssh`
/// argv. `harness` selects a compiled-in recipe; an unrecognized value comes
/// back as `supported: false`, which is not an error but a prompt to enter the
/// agent's identity manually.
///
/// Read-only. Listing a roster starts nothing and changes no harness state.
#[tauri::command]
pub async fn probe_harness_agents(
    host: String,
    harness: String,
) -> Result<HarnessRosterResult, String> {
    tokio::task::spawn_blocking(move || {
        let hosts = parse_ssh_config();
        let Some(entry) = hosts.into_iter().find(|candidate| candidate.host == host) else {
            return Err(format!(
                "'{host}' is not a Host alias in ~/.ssh/config; only configured hosts can be probed"
            ));
        };
        Ok(probe_ssh_harness_agents(&entry, &harness))
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

/// List the durable agents of a harness on this machine, shape-compatible with
/// [`probe_harness_agents`].
#[tauri::command]
pub async fn probe_local_harness_agent_roster(
    harness: String,
) -> Result<HarnessRosterResult, String> {
    tokio::task::spawn_blocking(move || probe_local_harness_agents(&harness))
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))
}

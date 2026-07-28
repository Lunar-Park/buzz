//! Key custody: where an agent's signing key lives, and the view type for the
//! agents whose key is somewhere else.
//!
//! A separate module rather than more lines in `types.rs`, which sits right at
//! the desktop 1000-line limit with no override. The split is also the honest
//! grouping: custody is one concept with one invariant — Buzz can only act as
//! an agent whose key it holds — and both types here exist to express it.

use serde::{Deserialize, Serialize};

/// Where an agent's signing key lives.
///
/// Buzz has only ever had one answer — Buzz minted the key and it is in this
/// machine's keyring — so the question was never asked anywhere in the code. A
/// self-hosted agent breaks that assumption: it is a real agent with a real
/// pubkey whose secret was minted on, and never leaves, a machine the user
/// owns. That is not a degraded managed agent; it is a different custody
/// model, and every lifecycle affordance Buzz offers (start, stop, deploy,
/// tombstone, profile republish) is predicated on holding the key.
///
/// Deliberately **not** folded into [`BackendKind`], which answers a different
/// question: *where does the process run*. The two are independent — a
/// provider-backed agent runs on another machine while Buzz still holds its
/// key, i.e. a remote process under `Local` custody. Conflating them would
/// make the opposite diagonal unrepresentable the first time it is needed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyCustody {
    /// Buzz minted the key and holds it (OS keyring, or the `0o600` JSON
    /// fallback during a keyring outage). The default, so every pre-existing
    /// record deserializes unchanged.
    #[default]
    Local,
    /// The key lives on `host` and Buzz has never seen it. Buzz can address
    /// this agent and read what it publishes; it cannot sign as it, start it,
    /// or stop it.
    ///
    /// `host` is an `~/.ssh/config` alias, validated at the connect boundary
    /// against the user's own parsed config — it is a probe target, so an
    /// alias `ssh` cannot resolve would produce a record whose reachability
    /// can never be reported.
    Remote { host: String },
}

impl KeyCustody {
    /// `true` for the Buzz-holds-the-key case. Also the
    /// `skip_serializing_if` predicate for the record field, so a local
    /// agent's stored JSON is byte-identical to what earlier builds wrote.
    pub fn is_local(&self) -> bool {
        matches!(self, KeyCustody::Local)
    }

    /// The host an agent's key lives on, or `None` under local custody.
    pub fn remote_host(&self) -> Option<&str> {
        match self {
            KeyCustody::Local => None,
            KeyCustody::Remote { host } => Some(host),
        }
    }
}

/// A self-hosted agent Buzz talks to but does not own.
///
/// Deliberately not a [`ManagedAgentSummary`]. That type carries `status`,
/// `pid`, `log_path`, `needs_restart`, `start_on_app_launch`, and
/// `auto_restart_on_config_change` — every one of them a claim about a process
/// Buzz supervises. Projecting a connected agent onto it would force this
/// surface to invent a lifecycle it has no access to (a self-supervised agent
/// with no local pid is not "stopped"), and the UI would then render controls
/// that cannot work. The narrow shape is what makes "no start/stop button"
/// a property of the type rather than a rule someone has to remember.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedAgentSummary {
    /// The agent's own pubkey, lowercase hex. Buzz holds only the public half.
    pub pubkey: String,
    /// Local label. Not published anywhere — the agent's own kind:10100
    /// profile is the authority on how it presents itself on the relay.
    pub name: String,
    /// `~/.ssh/config` alias of the machine the agent (and its key) lives on.
    pub host: String,
    /// Harness id observed on the host when the agent was connected, e.g.
    /// `"claude"`. A record of what was there, not a spawn instruction —
    /// nothing in Buzz executes it. `None` when the user connected without a
    /// completed probe.
    pub harness: Option<String>,
    /// When the agent was connected. There is deliberately no `relay_url`:
    /// every agent relay lookup resolves the active workspace relay at read
    /// time (see [`crate::relay::effective_agent_relay_url`]), so a per-agent
    /// relay shown here could only ever be a stale value the rest of the app
    /// ignores.
    pub created_at: String,
    pub updated_at: String,
}

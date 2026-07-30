/**
 * Types for the remote-agent surface: enumerating the user's own SSH hosts and
 * probing them for agent harnesses.
 *
 * A separate module rather than more lines in `types.ts`, which is already over
 * the desktop 1000-line limit and carries a documented "queued to be split"
 * override. Import these from here directly — `types.ts` deliberately does not
 * re-export them, because a re-export block would put it back over the limit
 * and defeat the point of the split.
 */

/** One `Host` stanza from the user's `~/.ssh/config`. */
export type SshHost = {
  /** The `Host` alias as written — this is what gets passed to `ssh`. */
  host: string;
  hostname?: string | null;
  user?: string | null;
  port?: string | null;
  identityFile?: string | null;
};

/**
 * Why a host probe failed, when the cause is actionable.
 *
 * `password_required` means the host offered only interactive auth. Buzz never
 * collects or stores an SSH password, so this is a status to render with a
 * remedy, not a prompt to raise.
 *
 * `host_key_problem` covers both an untrusted first-seen key and a changed one.
 * Buzz probes with strict host-key checking and never writes to `known_hosts`,
 * so granting trust is always something the user does outside the app.
 *
 * `truncated` means the probe started but its output stopped early, so the facts
 * are an unknown fraction of the real ones and are withheld rather than shown as
 * a complete answer.
 */
export type HostProbeErrorKind =
  | "password_required"
  | "host_key_problem"
  | "unreachable"
  | "timed_out"
  | "truncated";

/**
 * One agent harness found on a probed host.
 *
 * Deliberately narrower than `AcpRuntime`: that type carries install and auth
 * affordances that only apply to the local machine. Buzz does not install
 * software on, or authenticate CLIs on, another host.
 */
export type RemoteHarness = {
  id: string;
  label: string;
  source: "builtin" | "preset" | "custom";
  acpCommand?: string | null;
  acpCommandPath?: string | null;
  version?: string | null;
  underlyingCliPath?: string | null;
  /**
   * True when the harness is usable on this host: its ACP command resolved and,
   * if it is an adapter, the vendor CLI it wraps resolved too. An adapter
   * without its CLI starts and then fails at first use.
   */
  ready: boolean;
  installHint: string;
  installInstructionsUrl: string;
};

/**
 * Result of probing one host for agent harnesses.
 *
 * A host-side problem comes back with `ok: false` and a classified
 * `errorKind` rather than as a thrown error — the UI shows one row per host and
 * needs a renderable status.
 */
export type HostProbeResult = {
  /** The ssh alias probed, or `__localhost__` for this machine. */
  host: string;
  ok: boolean;
  durationMs: number;
  error?: string | null;
  errorKind?: HostProbeErrorKind | null;
  user?: string | null;
  hostname?: string | null;
  os?: string | null;
  /** Path of the `buzz` CLI on the host; a connected agent needs it. */
  buzzCliPath?: string | null;
  buzzCliVersion?: string | null;
  harnesses: RemoteHarness[];
};

/** Host id the backend uses for the local machine. */
export const LOCALHOST_HOST_ID = "__localhost__";

/**
 * A self-hosted agent Buzz talks to but does not own: it runs on a machine the
 * user owns, supervises itself, and holds its own signing key.
 *
 * Deliberately **not** a `ManagedAgent`. That type carries `status`, `pid`,
 * `logPath`, `needsRestart`, and `startOnAppLaunch` — each one a claim about a
 * process Buzz supervises. A connected agent has none of those, and the narrow
 * shape is what makes "no start/stop button" a property of the type rather
 * than a rule a component has to remember. Connected agents are not part of
 * `listManagedAgents()` at all: they are a separate record type in a separate
 * store, so they cannot reach a surface that renders lifecycle controls.
 */
/**
 * One durable, named agent a harness holds on a host.
 *
 * Harness-neutral on purpose. A resident harness may call these subagents
 * internally, but an enrolled candidate is a normal first-class Buzz agent and
 * nothing here carries the harness's vocabulary beyond `harnessId`.
 *
 * Ephemeral per-turn workers are never candidates: their work stays attributed
 * to the parent agent unless the harness later promotes them to durable, named
 * agents.
 */
export type RemoteAgentCandidate = {
  /** Harness that reported this agent, matching `RemoteHarness.id`. */
  harnessId: string;
  /**
   * The harness's own agent identifier — the routing key. A reply must be
   * produced by this exact agent, never a parent or sibling.
   */
  agentId: string;
  /** Best available label; falls back to `agentId` for an unnamed primary. */
  displayName: string;
  /**
   * True for the harness primary. The selector preselects it and leaves the
   * rest of the roster visible but unselected — the whole stack is never
   * enrolled automatically.
   */
  isPrimary: boolean;
  model?: string;
  workspace?: string;
  /** Existing harness routing bindings. Informational; a bound agent is still enrollable. */
  bindingCount?: number;
};

/** Outcome of listing one harness's durable agents. */
export type HarnessRosterResult = {
  host: string;
  harnessId: string;
  ok: boolean;
  durationMs: number;
  /**
   * False when Buzz has no recipe for this harness — distinct from `ok: false`.
   * Nothing is wrong with the host; offer manual identity entry rather than an
   * error with a retry.
   */
  supported: boolean;
  error?: string;
  errorKind?: HostProbeErrorKind;
  candidates: RemoteAgentCandidate[];
};

/**
 * A NIP-OA owner attestation issued for a connected agent, ready to install on
 * the agent's host.
 *
 * This is what makes a self-hosted agent *addressable* rather than just visible.
 * The relay materializes an owner for the agent when it sees the agent publish
 * while presenting this tag, and two behaviours depend on that: an `owner_only`
 * `channel_add_policy` will only accept an add from the materialized owner, and a
 * membership-required relay admits an agent whose attested owner is a member.
 *
 * Not a secret and not a bearer credential — the signature covers the *agent's*
 * pubkey, so the tag is inert without the agent's private key, which Buzz never
 * holds. Buzz cannot install it either: that would mean writing configuration on
 * a machine it does not administer.
 */
export type ConnectedAgentOwnerEvidence = {
  agentPubkey: string;
  ownerPubkey: string;
  /** The `auth` tag as a JSON array string: `["auth","<owner>","<conditions>","<sig>"]`. */
  authTag: string;
  /**
   * Always empty. The relay's membership and channel-add paths verify only the
   * signature and never evaluate a clause, so a `kind=` or `created_at<` value
   * would read as a restriction while restricting nothing.
   */
  conditions: string;
  issuedAt: string;
  /** True when this replaced an attestation already on the record. */
  replacedPrevious: boolean;
};

export type ConnectedAgent = {
  /** The agent's own pubkey, lowercase hex. Buzz holds only the public half. */
  pubkey: string;
  /** Buzz-local label. The agent's own kind:10100 profile is what the relay sees. */
  name: string;
  /** `~/.ssh/config` alias of the machine the agent and its key live on. */
  host: string;
  /**
   * Harness id observed on the host at connect time (e.g. `"claude"`). A
   * record of what was there — nothing in Buzz executes it.
   */
  harness: string | null;
  createdAt: string;
  updatedAt: string;
  /**
   * Whether Buzz has issued an owner attestation for this agent. A flag, not the
   * tag: the list surface needs to know whether the agent is addressable, and the
   * signature belongs only on the screen that transfers it.
   */
  hasOwnerEvidence: boolean;
  /** Owner that signed the attestation — lets a tag from a replaced identity read as stale. */
  ownerAuthOwnerPubkey?: string;
  ownerAuthIssuedAt?: string;
};

import type {
  HarnessRosterResult,
  HostProbeResult,
  RemoteAgentCandidate,
  RemoteHarness,
} from "@/shared/api/remoteAgentTypes";

/**
 * Draft state for the Connect-an-agent dialog.
 *
 * `probe` is the RC3 host probe result, kept in the draft rather than derived
 * on submit because the harness options and the "is this host even reachable"
 * answer both come from it.
 */
export type ConnectAgentDraft = {
  host: string;
  pubkey: string;
  name: string;
  harness: string;
  probe: HostProbeResult | null;
  isProbing: boolean;
  /**
   * Durable agents the selected harness reports, or `null` before a roster has
   * been asked for. A harness Buzz cannot enumerate comes back with
   * `supported: false`, which is a prompt for manual entry rather than an error.
   */
  roster: HarnessRosterResult | null;
  isLoadingRoster: boolean;
  /** The harness agent the user picked, e.g. `"main"`. Empty when none. */
  harnessAgentId: string;
  /**
   * The last name this dialog suggested from a roster selection.
   *
   * Tracked so a later selection can replace a name the dialog itself filled in
   * while still never overwriting one the user typed.
   */
  nameSuggestion: string;
};

export const emptyConnectAgentDraft: ConnectAgentDraft = {
  host: "",
  pubkey: "",
  name: "",
  harness: "",
  probe: null,
  isProbing: false,
  roster: null,
  isLoadingRoster: false,
  harnessAgentId: "",
  nameSuggestion: "",
};

/**
 * Client-side pubkey shape check.
 *
 * The backend is the authority — it normalizes and stores — but repeating the
 * shape check here lets the dialog disable submit and explain why instead of
 * round-tripping to produce an error. `nsec` gets its own answer because
 * "invalid" would not tell a user who just pasted their agent's secret what
 * they actually did.
 */
export type PubkeyVerdict =
  | { kind: "empty" }
  | { kind: "secret" }
  | { kind: "invalid" }
  | { kind: "ok" };

const HEX64 = /^[0-9a-fA-F]{64}$/;
// npub1 + 58 bech32 data characters. Length is checked rather than the checksum:
// the backend verifies the checksum, and a client-side bech32 implementation
// here would be a second decoder to keep correct.
const NPUB = /^npub1[023456789acdefghjklmnpqrstuvwxyz]{58}$/;

export function verifyPubkeyInput(input: string): PubkeyVerdict {
  const trimmed = input.trim();
  if (!trimmed) return { kind: "empty" };
  if (trimmed.startsWith("nsec")) return { kind: "secret" };
  if (HEX64.test(trimmed) || NPUB.test(trimmed)) return { kind: "ok" };
  return { kind: "invalid" };
}

/** Human-readable reason a pubkey input is not usable yet, or `null`. */
export function pubkeyInputMessage(input: string): string | null {
  switch (verifyPubkeyInput(input).kind) {
    case "empty":
      return null;
    case "secret":
      return "That is a secret key. A self-hosted agent's nsec must never leave its own machine — paste its npub instead.";
    case "invalid":
      return "Expected an npub or 64 hex characters.";
    case "ok":
      return null;
  }
}

export const MAX_CONNECTED_NAME_LENGTH = 64;

/** Human-readable reason a name is not usable yet, or `null`. */
export function nameInputMessage(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  if (trimmed.length > MAX_CONNECTED_NAME_LENGTH) {
    return `Names are limited to ${MAX_CONNECTED_NAME_LENGTH} characters.`;
  }
  return null;
}

/**
 * Harnesses worth offering for a connected agent.
 *
 * Only `ready` ones: an ACP adapter whose vendor CLI is missing starts and then
 * fails at first use, so listing it as the agent's harness would record
 * something known-broken. An empty list is a legitimate answer — the host may
 * run an agent Buzz has no recipe for — which is why the harness field is
 * optional.
 */
export function harnessOptions(probe: HostProbeResult | null): RemoteHarness[] {
  if (!probe?.ok) return [];
  return probe.harnesses.filter((harness) => harness.ready);
}

/**
 * True when the host probe came back but found no `buzz` CLI.
 *
 * Not a blocker: the CLI can be installed after connecting, and a user may be
 * recording an agent they are still setting up. It is the single most useful
 * warning to show, because without it the agent cannot reach the relay at all.
 */
export function missingBuzzCli(probe: HostProbeResult | null): boolean {
  return Boolean(probe?.ok) && !probe?.buzzCliPath;
}

/**
 * Submit gate.
 *
 * Deliberately does NOT require a successful probe. A machine that is asleep,
 * off the VPN, or mid-reboot is still an agent host the user wants recorded —
 * blocking on reachability would make the feature unusable exactly when the
 * user is setting things up. What is required is a host, a well-formed pubkey,
 * and a name; the backend re-validates all three and additionally rejects a
 * host that is not in `~/.ssh/config`.
 */
export function canSubmitConnectAgent(draft: ConnectAgentDraft): boolean {
  if (draft.isProbing) return false;
  if (!draft.host.trim()) return false;
  if (verifyPubkeyInput(draft.pubkey).kind !== "ok") return false;
  const name = draft.name.trim();
  if (!name || name.length > MAX_CONNECTED_NAME_LENGTH) return false;
  return true;
}

/** The payload `connectRemoteAgent` expects, or `null` when not submittable. */
export function connectAgentPayload(draft: ConnectAgentDraft) {
  if (!canSubmitConnectAgent(draft)) return null;
  const harness = draft.harness.trim();
  const harnessAgentId = draft.harnessAgentId.trim();
  return {
    host: draft.host.trim(),
    pubkey: draft.pubkey.trim(),
    name: draft.name.trim(),
    harness: harness ? harness : null,
    // Only meaningful alongside a harness: an agent id without the harness that
    // scopes it does not identify anything.
    harnessAgentId: harness && harnessAgentId ? harnessAgentId : null,
  };
}

/**
 * Candidates worth offering, or `[]`.
 *
 * A failed or unsupported roster yields nothing rather than a partial list: half
 * a roster reads as "these are the agents" and would hide the rest with no
 * visible reason.
 */
export function rosterCandidates(
  roster: HarnessRosterResult | null,
): RemoteAgentCandidate[] {
  if (!roster?.ok) return [];
  return roster.candidates;
}

/**
 * The candidate to preselect: the harness primary.
 *
 * Empty when the harness flags none, which is deliberate — inventing a
 * preselection would enroll an agent Buzz guessed at. The specification's rule is
 * "primary or `main`", and the backend already applies the `main` fallback, so
 * anything still unflagged here genuinely has no primary.
 */
export function preselectedCandidateId(
  roster: HarnessRosterResult | null,
): string {
  const primary = rosterCandidates(roster).find(
    (candidate) => candidate.isPrimary,
  );
  return primary?.agentId ?? "";
}

/** Label for one candidate: its name, plus its id when they differ. */
export function candidateLabel(candidate: RemoteAgentCandidate): string {
  const suffix = candidate.isPrimary ? " · primary" : "";
  return candidate.displayName === candidate.agentId
    ? `${candidate.agentId}${suffix}`
    : `${candidate.displayName} (${candidate.agentId})${suffix}`;
}

/**
 * What to tell the user about a roster attempt, or `null` when the list speaks
 * for itself.
 *
 * `supported: false` is not a failure and must not read as one: the host is fine
 * and manual entry is the intended path, so a retry prompt would send the user
 * after a problem that does not exist.
 */
export function rosterStatusMessage(
  roster: HarnessRosterResult | null,
): string | null {
  if (!roster) return null;
  if (!roster.supported) {
    return "Buzz cannot list this harness's agents yet — enter the agent's identity below.";
  }
  if (!roster.ok) {
    return roster.error ?? "Could not list this harness's agents.";
  }
  if (roster.candidates.length === 0) {
    return "This harness reported no configured agents.";
  }
  return null;
}

/**
 * Which identity affordance applies to the currently selected agent.
 *
 * The subtlety this exists for: a harness reports **one** configured Buzz
 * identity, because today's adapter serves a single Buzz account. That identity
 * belongs to the harness's primary agent. Offering it for any *other* roster
 * candidate would connect that agent under the primary's signing key — two
 * independently visible Buzz agents sharing one key, which the specification
 * forbids outright.
 *
 * So a resolved identity is only offered when it can be attributed to the agent
 * being connected:
 *
 * - `use-resolved` — the harness's identity, and the selection is its primary (or
 *   there is no roster to narrow it, so the harness-level answer is the only
 *   meaningful one).
 * - `generate` — this agent has no identity of its own yet. True for every
 *   non-primary candidate until the adapter supports an account per agent.
 * - `unsupported` — Buzz cannot read this harness's identity; manual entry only.
 */
export type IdentityOffer = "use-resolved" | "generate" | "unsupported";

export function identityOffer({
  hasRosterSelection,
  isPrimarySelected,
  resolvedPubkey,
  supported,
}: {
  hasRosterSelection: boolean;
  isPrimarySelected: boolean;
  resolvedPubkey: string | null;
  supported: boolean;
}): IdentityOffer {
  if (!supported) return "unsupported";
  if (!resolvedPubkey) return "generate";
  // A selection that is not the primary cannot claim the harness's single
  // configured identity.
  if (hasRosterSelection && !isPrimarySelected) return "generate";
  return "use-resolved";
}

/**
 * Name to prefill from a roster selection, or `null` to leave the field alone.
 *
 * Never overwrites something the user typed: the field is theirs once touched,
 * and a selection change silently renaming their agent would be worse than no
 * prefill at all.
 */
export function nameSuggestionForCandidate(
  candidate: RemoteAgentCandidate | undefined,
  currentName: string,
  previousSuggestion: string,
): string | null {
  if (!candidate) return null;
  const untouched = !currentName.trim() || currentName === previousSuggestion;
  return untouched ? candidate.displayName : null;
}

/**
 * Compact label for a failed probe, for the Connected Agents list.
 *
 * Every classified kind gets its own wording because they call for different
 * actions, and "machine unreachable" is actively wrong for all but one of them:
 * the host answered in every case except `unreachable`. Labelling an untrusted
 * host key as unreachable would send someone to check the network when the fix
 * is to review a fingerprint — and since Buzz probes with strict host-key
 * checking and never writes `known_hosts`, this label is the only prompt the
 * user gets.
 */
export function reachabilityLabel(probe: HostProbeResult): string {
  switch (probe.errorKind) {
    case "password_required":
      return "needs an ssh key";
    case "host_key_problem":
      return "host key not trusted";
    case "truncated":
      return "probe incomplete · retry";
    case "timed_out":
      return "probe timed out";
    case "unreachable":
      return "machine unreachable";
    default:
      // Unclassified: the backend could not attribute the failure, so naming a
      // specific cause here would be a guess.
      return "probe failed";
  }
}

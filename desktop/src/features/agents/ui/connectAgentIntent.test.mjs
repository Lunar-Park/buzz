import assert from "node:assert/strict";
import test from "node:test";

import {
  canSubmitConnectAgent,
  candidateLabel,
  connectAgentPayload,
  emptyConnectAgentDraft,
  harnessOptions,
  identityOffer,
  missingBuzzCli,
  nameInputMessage,
  nameSuggestionForCandidate,
  preselectedCandidateId,
  pubkeyInputMessage,
  reachabilityLabel,
  rosterCandidates,
  rosterStatusMessage,
  verifyPubkeyInput,
} from "./connectAgentIntent.ts";

const HEX = "3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d";
const NPUB = "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6";

function draft(overrides = {}) {
  return {
    ...emptyConnectAgentDraft,
    host: "lunar02",
    pubkey: HEX,
    name: "Scout",
    ...overrides,
  };
}

function probe(overrides = {}) {
  return {
    host: "lunar02",
    ok: true,
    durationMs: 900,
    harnesses: [],
    buzzCliPath: "/Users/miles/.local/bin/buzz",
    ...overrides,
  };
}

test("both pubkey forms a user actually has on hand are accepted", () => {
  assert.equal(verifyPubkeyInput(HEX).kind, "ok");
  assert.equal(verifyPubkeyInput(NPUB).kind, "ok");
  assert.equal(verifyPubkeyInput(HEX.toUpperCase()).kind, "ok");
  assert.equal(verifyPubkeyInput(`  ${NPUB}  `).kind, "ok");
});

test("a pasted secret key is called out as such, not as invalid input", () => {
  // A user who pastes an nsec has made a serious mistake. "Invalid pubkey"
  // would not tell them what it was, and they would try again with the same
  // secret.
  assert.equal(verifyPubkeyInput("nsec1abcdef").kind, "secret");
  const message = pubkeyInputMessage("nsec1abcdef");
  assert.match(message, /secret key/i);
  assert.match(message, /never leave/i);
});

test("malformed pubkeys are rejected", () => {
  for (const bad of [
    "not-a-key",
    "npub1short",
    HEX.slice(0, 63),
    `${HEX}f`,
    // Bech32 excludes 1, b, i, and o — a lookalike must not pass.
    `npub1${"b".repeat(58)}`,
  ]) {
    assert.equal(verifyPubkeyInput(bad).kind, "invalid", bad);
  }
});

test("an empty pubkey is silent, not an error", () => {
  // Nothing typed yet is not a mistake; showing red text on an untouched field
  // trains users to ignore it.
  assert.equal(verifyPubkeyInput("").kind, "empty");
  assert.equal(pubkeyInputMessage(""), null);
  assert.equal(pubkeyInputMessage("   "), null);
});

test("names are bounded and the bound is stated", () => {
  assert.equal(nameInputMessage("Scout"), null);
  assert.equal(nameInputMessage(""), null);
  assert.equal(nameInputMessage("n".repeat(64)), null);
  assert.match(nameInputMessage("n".repeat(65)), /64 characters/);
});

test("submit requires a host, a well-formed pubkey, and a name", () => {
  assert.equal(canSubmitConnectAgent(draft()), true);
  assert.equal(canSubmitConnectAgent(draft({ host: "" })), false);
  assert.equal(canSubmitConnectAgent(draft({ host: "   " })), false);
  assert.equal(canSubmitConnectAgent(draft({ pubkey: "nope" })), false);
  assert.equal(canSubmitConnectAgent(draft({ name: "" })), false);
  assert.equal(canSubmitConnectAgent(draft({ name: "n".repeat(65) })), false);
});

test("submit does not require a reachable host", () => {
  // A machine that is asleep, off the VPN, or mid-reboot is still an agent host
  // the user wants recorded. Gating on reachability would break the feature
  // exactly during setup.
  assert.equal(canSubmitConnectAgent(draft({ probe: null })), true);
  assert.equal(
    canSubmitConnectAgent(
      draft({ probe: probe({ ok: false, errorKind: "unreachable" }) }),
    ),
    true,
  );
});

test("submit is blocked while a probe is in flight", () => {
  // The probe fills the harness options; submitting mid-probe would record a
  // null harness the user was about to pick.
  assert.equal(canSubmitConnectAgent(draft({ isProbing: true })), false);
});

test("the payload trims and omits an unset harness", () => {
  assert.deepEqual(
    connectAgentPayload(draft({ host: " lunar02 ", name: "  Scout  " })),
    {
      host: "lunar02",
      pubkey: HEX,
      name: "Scout",
      harness: null,
      harnessAgentId: null,
    },
  );
  assert.deepEqual(
    connectAgentPayload(draft({ harness: "claude" })).harness,
    "claude",
  );
  assert.equal(connectAgentPayload(draft({ harness: "   " })).harness, null);
});

test("an unsubmittable draft yields no payload", () => {
  assert.equal(connectAgentPayload(draft({ pubkey: "" })), null);
});

test("only ready harnesses are offered", () => {
  // An ACP adapter whose vendor CLI is missing starts and then fails at first
  // use. Offering it would record something known-broken as the agent's
  // harness.
  const options = harnessOptions(
    probe({
      harnesses: [
        { id: "claude", label: "Claude Code", ready: true },
        { id: "codex", label: "Codex", ready: false },
      ],
    }),
  );
  assert.deepEqual(
    options.map((harness) => harness.id),
    ["claude"],
  );
});

test("a failed or absent probe offers no harnesses", () => {
  assert.deepEqual(harnessOptions(null), []);
  assert.deepEqual(
    harnessOptions(
      probe({
        ok: false,
        errorKind: "password_required",
        harnesses: [{ id: "claude", label: "Claude Code", ready: true }],
      }),
    ),
    [],
  );
});

test("a missing buzz CLI is flagged only once the probe succeeded", () => {
  // Without the CLI the agent cannot reach the relay at all, so it is the one
  // warning worth surfacing — but an unreachable host has not told us anything
  // about its CLI, and claiming it is missing would be a fabrication.
  assert.equal(missingBuzzCli(probe({ buzzCliPath: null })), true);
  assert.equal(missingBuzzCli(probe()), false);
  assert.equal(missingBuzzCli(null), false);
  assert.equal(missingBuzzCli(probe({ ok: false, buzzCliPath: null })), false);
});

test("a failed probe is labelled by cause, not as unreachable", () => {
  // "machine unreachable" is wrong for every classified kind except one — the
  // host answered in all the others. The host-key case matters most: Buzz probes
  // with strict checking and never writes known_hosts, so this label is the only
  // prompt telling the user to go review a fingerprint.
  const label = (errorKind) =>
    reachabilityLabel({
      host: "lunar02",
      ok: false,
      durationMs: 1,
      errorKind,
      harnesses: [],
    });

  assert.equal(label("host_key_problem"), "host key not trusted");
  assert.equal(label("truncated"), "probe incomplete \u00b7 retry");
  assert.equal(label("password_required"), "needs an ssh key");
  assert.equal(label("timed_out"), "probe timed out");
  assert.equal(label("unreachable"), "machine unreachable");

  // Only `unreachable` may claim the machine could not be reached.
  for (const kind of [
    "host_key_problem",
    "truncated",
    "password_required",
    "timed_out",
  ]) {
    assert.ok(
      !label(kind).includes("unreachable"),
      `${kind} must not be reported as unreachable`,
    );
  }
});

test("an unclassified probe failure does not invent a cause", () => {
  const label = reachabilityLabel({
    host: "lunar02",
    ok: false,
    durationMs: 1,
    errorKind: null,
    harnesses: [],
  });
  assert.equal(label, "probe failed");
});

// ── durable agent roster (RC5) ───────────────────────────────────────────────

function candidate(overrides = {}) {
  return {
    harnessId: "openclaw",
    agentId: "main",
    displayName: "main",
    isPrimary: true,
    ...overrides,
  };
}

function roster(overrides = {}) {
  return {
    host: "lunar01",
    harnessId: "openclaw",
    ok: true,
    durationMs: 12,
    supported: true,
    candidates: [candidate()],
    ...overrides,
  };
}

test("a failed roster offers nothing rather than a partial list", () => {
  // Half a roster reads as "these are the agents" and would hide the rest with
  // no visible reason.
  assert.deepEqual(
    rosterCandidates(roster({ ok: false, candidates: [candidate()] })),
    [],
  );
  assert.deepEqual(rosterCandidates(null), []);
});

test("the primary is preselected", () => {
  const result = roster({
    candidates: [
      candidate({ agentId: "astra", displayName: "Astra", isPrimary: false }),
      candidate({ agentId: "main", isPrimary: true }),
    ],
  });
  assert.equal(preselectedCandidateId(result), "main");
});

test("no preselection is invented when the harness flags no primary", () => {
  // The backend already applies the spec's `main` fallback, so anything still
  // unflagged here genuinely has no primary — guessing would enroll an agent the
  // user never chose.
  const result = roster({
    candidates: [
      candidate({ agentId: "astra", displayName: "Astra", isPrimary: false }),
      candidate({ agentId: "cato", displayName: "Cato", isPrimary: false }),
    ],
  });
  assert.equal(preselectedCandidateId(result), "");
});

test("candidate labels disambiguate name from id", () => {
  assert.equal(candidateLabel(candidate()), "main · primary");
  assert.equal(
    candidateLabel(
      candidate({ agentId: "astra", displayName: "Astra", isPrimary: false }),
    ),
    "Astra (astra)",
  );
});

test("an unsupported harness reads as manual entry, not as a failure", () => {
  // A retry prompt here would send the user after a problem that does not exist.
  const message = rosterStatusMessage(
    roster({ ok: false, supported: false, candidates: [] }),
  );
  assert.match(message, /cannot list/);
  assert.doesNotMatch(message, /retry|again/i);
});

test("a genuine roster failure surfaces the backend's reason", () => {
  const message = rosterStatusMessage(
    roster({ ok: false, error: "ssh said no", candidates: [] }),
  );
  assert.equal(message, "ssh said no");
});

test("an empty roster says so instead of looking like a loading state", () => {
  assert.match(
    rosterStatusMessage(roster({ candidates: [] })),
    /no configured agents/,
  );
});

test("a healthy roster needs no message", () => {
  assert.equal(rosterStatusMessage(roster()), null);
  assert.equal(rosterStatusMessage(null), null);
});

test("the payload carries the selected harness agent id", () => {
  const payload = connectAgentPayload(
    draft({ harness: "openclaw", harnessAgentId: "astra" }),
  );
  assert.equal(payload.harnessAgentId, "astra");
});

test("an agent id without a harness is dropped", () => {
  // An agent id is scoped by its harness; alone it identifies nothing.
  const payload = connectAgentPayload(
    draft({ harness: "", harnessAgentId: "astra" }),
  );
  assert.equal(payload.harnessAgentId, null);
});

test("a connect with no roster selection still submits", () => {
  // A harness may hold exactly one agent, or Buzz may not enumerate it.
  const payload = connectAgentPayload(draft({ harness: "openclaw" }));
  assert.equal(payload.harnessAgentId, null);
  assert.ok(canSubmitConnectAgent(draft({ harness: "openclaw" })));
});

test("a name is suggested only while the field is untouched", () => {
  const astra = candidate({ agentId: "astra", displayName: "Astra" });
  assert.equal(nameSuggestionForCandidate(astra, "", ""), "Astra");
  // Still replaceable when the current value is the previous suggestion.
  assert.equal(nameSuggestionForCandidate(astra, "main", "main"), "Astra");
  // Never overwrites something the user typed themselves.
  assert.equal(nameSuggestionForCandidate(astra, "My Agent", "main"), null);
  assert.equal(nameSuggestionForCandidate(undefined, "", ""), null);
});

// ── identity attribution ─────────────────────────────────────────────────────

test("the harness identity is offered for its primary agent", () => {
  assert.equal(
    identityOffer({
      hasRosterSelection: true,
      isPrimarySelected: true,
      resolvedPubkey: "a".repeat(64),
      supported: true,
    }),
    "use-resolved",
  );
});

test("the harness identity is NOT offered for a non-primary agent", () => {
  // A harness reports one configured Buzz identity and it belongs to the
  // primary. Offering it for a sibling would connect that agent under the
  // primary's signing key — two visible agents sharing one key.
  assert.equal(
    identityOffer({
      hasRosterSelection: true,
      isPrimarySelected: false,
      resolvedPubkey: "a".repeat(64),
      supported: true,
    }),
    "generate",
  );
});

test("with no roster to narrow it, the harness-level identity still applies", () => {
  // A single-agent harness, or one Buzz cannot enumerate: the harness-level
  // answer is the only meaningful one, so withholding it would send the user to
  // fetch a pubkey that was already on screen.
  assert.equal(
    identityOffer({
      hasRosterSelection: false,
      isPrimarySelected: false,
      resolvedPubkey: "a".repeat(64),
      supported: true,
    }),
    "use-resolved",
  );
});

test("an absent identity offers generation", () => {
  assert.equal(
    identityOffer({
      hasRosterSelection: true,
      isPrimarySelected: true,
      resolvedPubkey: null,
      supported: true,
    }),
    "generate",
  );
});

test("an unreadable harness offers neither", () => {
  assert.equal(
    identityOffer({
      hasRosterSelection: false,
      isPrimarySelected: false,
      resolvedPubkey: null,
      supported: false,
    }),
    "unsupported",
  );
});

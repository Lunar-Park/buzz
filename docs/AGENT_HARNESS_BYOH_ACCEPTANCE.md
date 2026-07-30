# Hermes and OpenClaw BYOH acceptance

Status: evidence record for the Buzz-managed ACP lane. Gates 1 and 3 passed on
2026-07-26; Gates 2 and 4 remain open. This document is not product or
resident-plugin authority and does not record or prove the resident OpenClaw
channel-plugin canary. Resident-plugin execution state belongs in
[`RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md`](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md).

This runbook verifies Buzz's generic ACP boundary before any harness-specific
adapter code is added. It applies to the tier-2 `hermes` and `openclaw` presets
in Desktop.

The first gate is intentionally relay-free: `buzz-acp models` spawns the preset
command, negotiates ACP `initialize`, creates a session with `session/new`, and
then shuts the child down. Passing discovery alone is insufficient because
OpenClaw can be on `PATH` while its Gateway is unavailable.

## Baseline

- Clean branch base: upstream `95fdf978800982389b120c66ff5e766d785419c7`
- Preserved fork head: `049568ac5129fd01568d28d60a8ead1f2b01c652`
- Local checkpoint tag: `pre-agent-adapter-realignment-2026-07-26`
- Hermes preset: `hermes-acp` with no arguments
- OpenClaw preset: `openclaw acp`

The checkpoint tag is local. Do not push it as part of an upstream feature PR.

## Gate 1: local ACP protocol

Activate Hermit and run both probes:

```bash
. ./bin/activate-hermit
just agent-harness-acceptance
```

The recipe builds the current `buzz-acp` source before probing so an older local
binary cannot produce a false pass.

Run one harness:

```bash
just agent-harness-acceptance hermes
just agent-harness-acceptance openclaw
```

Override command locations when the runtime is not on the interactive shell's
`PATH`:

```bash
HERMES_ACP_BIN=/absolute/path/to/hermes-acp \
  just agent-harness-acceptance hermes

OPENCLAW_BIN=/opt/homebrew/bin/openclaw \
  just agent-harness-acceptance openclaw
```

Exit codes are part of the contract:

| Exit | Meaning |
|---:|---|
| `0` | Every selected harness completed `initialize` and `session/new` |
| `1` | Buzz build, ACP negotiation, or response validation failed |
| `2` | A selected harness command is not installed or not discoverable |
| `64` | Invalid harness selection |

Do not convert exit `2` into a protocol failure. Record the missing prerequisite
and rerun on the machine that owns the harness.

## Gate 2: Desktop discovery and creation

For each harness:

1. Open Settings > Agents and confirm the preset appears.
2. Confirm an installed command shows as detected.
3. Create a persona using that runtime.
4. Confirm the managed-agent record preserves the preset ID and exact
   command/arguments.
5. Start and stop the agent twice.
6. Confirm no fallback to `buzz-agent` occurs.

Relevant automated coverage:

```bash
. ./bin/activate-hermit
cargo test --manifest-path desktop/src-tauri/Cargo.toml \
  managed_agents::custom_harnesses
cd desktop && pnpm exec playwright test tests/e2e/harness-management.spec.ts
```

## Gate 3: relay interaction

Use a dedicated test identity and channel. Never reuse a production agent's
private key.

For each harness, verify:

1. The agent authenticates and is visible in the intended channel.
2. An owner mention produces exactly one reply.
3. The reply uses the correct `h` channel tag and thread relationship.
4. A non-owner mention is rejected under the default `owner-only` policy.
5. `!cancel` stops an active turn without rotating the channel session.
6. `!rotate` causes the next turn to use a fresh ACP session.
7. Tool execution can invoke `buzz` and reach the relay.
8. Restarting only `buzz-acp` does not duplicate the triggering event.
9. Restarting only the harness yields an explicit, classifiable result.

For OpenClaw, verify Buzz credentials at the Gateway execution locus. Environment
variables injected into the local `openclaw acp` bridge do not automatically
reach Gateway-side tools.

## Gate 4: session continuity

Run two prompts in the same channel with a fact introduced only in the first
prompt. Capture the ACP session ID, then repeat after:

1. a Desktop remount;
2. a `buzz-acp` restart;
3. a harness subprocess restart;
4. an OpenClaw Gateway restart, when applicable.

Classify each result as:

- session restored;
- session intentionally recreated with durable harness memory;
- session recreated and context lost;
- unsupported or failed.

Do not treat persistent harness memory as proof that the same ACP session was
loaded. Durable Buzz channel-to-ACP bindings remain dependent on the generic
session-loading work tracked upstream.

## Acceptance record

Record one row per environment:

| Harness | Version | Host | Gate 1 | Gate 2 | Gate 3 | Gate 4 | Notes |
|---|---|---|---|---|---|---|---|
| Hermes | 0.18.2 | lunar02 | Pass | Not run | Pass | Not run | Full relay path passed; exact-text reply carried one trailing newline |
| OpenClaw | 2026.7.1 | lunar01 | Pass | Not run | Pass | Not run | Full relay path passed through the Gateway execution locus |

The BYOH lane is accepted only when Gates 1-3 pass. Gate 4 may remain an
explicit upstream dependency, but its failure mode must be known before a
production migration.

### 2026-07-26 Gate 1 evidence

The locally tested ARM64 `buzz-acp` binary from upstream baseline `95fdf978`
was copied to an isolated `/tmp` path on each host. Its SHA-256 was
`70269a87af351b371c683ace70f0ef65e124ce29356c088047e9c1c4d416a8ce` on the
local machine and both remote hosts.

- lunar01: `/opt/homebrew/bin/openclaw acp` completed `initialize` and
  `session/new` with exit `0`, reporting `openclaw-acp 2026.7.1`.
- lunar02: `/Users/miles/.hermes/hermes-agent/venv/bin/hermes-acp` completed
  `initialize` and `session/new` with exit `0`, reporting
  `hermes-agent 0.18.2`.

Stdout and stderr were captured separately. The temporary probe binary and
result files were removed after the results were recorded. No running service
was restarted or reconfigured.

### 2026-07-26 Gate 3 evidence

- The lunar01 Buzz relay answered `GET /health` with HTTP `200`.
- Three dedicated identities were created: test owner, OpenClaw canary, and
  Hermes canary. Their private keys were not printed and remain in mode-600
  host-local files.
- All three identities were added as relay members. The owner created private
  channel `84dbe1a5-8e24-4fb5-b79c-d6d1af1728ab` with a 24-hour TTL and added
  both canaries as bot members.
- An owner mention produced exactly one reply from each harness. Both replies
  carried an `e` tag pointing to the correct triggering event.
- Cross-agent mentions produced zero replies, confirming the `owner-only`
  author gate.
- An owner `!cancel` event stopped an active turn in each harness. Both logs
  recorded `mode=Cancel`, and neither cancelled trigger received a reply.
- Owner `!shutdown` stopped both temporary harness processes cleanly. After
  restart, each original trigger still had exactly one reply, proving replay did
  not duplicate the processed events.
- Both harnesses replied to a fresh mention after restart. Hermes' reply content
  was `HERMES_RESTART_OK` plus one trailing newline; OpenClaw matched its
  requested text exactly.

OpenClaw confirmed the execution-locus constraint. The Gateway did not inherit
the bridge process's Buzz credentials, so the test temporarily installed a
`buzz` wrapper that sourced only the OpenClaw canary credentials. That wrapper
was removed immediately after the gate and replaced with a credential-free
symlink to the versioned CLI binary.

Persistent test assets:

- lunar01: `/opt/homebrew/bin/buzz.byoh-95fdf978`,
  `/opt/homebrew/bin/buzz`, and
  `/Users/selene/.local/bin/buzz-acp.byoh-95fdf978`
- lunar02: `/Users/miles/.local/share/buzz-byoh-95fdf978/bin/`
- canary evidence and credentials:
  `/Users/selene/.buzz-canary/gate3-20260726/` and
  `/Users/miles/.buzz-canary/gate3-20260726/`

The temporary admin binary and canary credential wrapper were removed. Both ACP
harnesses were stopped. A post-test invocation of lunar01's bare `buzz` command
failed with the expected auth exit code, confirming it no longer acts as the
canary.

### Gate 4 observation

Hermes advertised `loadSession`, but the pre-restart ACP session
`c5ee27a6-e055-4910-8bdf-aed35e62564e` was replaced after `buzz-acp` restart by
`abf28ed4-3959-4214-99c4-8ffdde55176d`. This is consistent with current
`buzz-acp` using `session/new`; it does not test whether Hermes could have loaded
the prior session. Context continuity remains a separate Gate 4 test.

## Decision boundary

- If both harnesses pass, keep Buzz harness-neutral and proceed to the separate
  resident-agent plugin lane.
- If one fails only on ACP protocol behavior, isolate the smallest generic ACP
  compatibility gap before changing Buzz.
- If the required behavior is lifecycle ownership, durable memory, or
  always-on messaging, do not add it to `buzz-acp`; implement it in the
  harness-native Buzz channel/platform plugin.

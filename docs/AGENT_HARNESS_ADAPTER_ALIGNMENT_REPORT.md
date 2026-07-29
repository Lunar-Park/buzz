# Agent Harness Adapter Alignment Report

Date: 2026-07-26

Scope: Lunar Park Buzz fork, Hermes/OpenClaw integration, and upstreamability

Mode: historical architecture assessment and rationale. It is not current
product, implementation, or handoff authority. See the
[fork document index](LUNAR_PARK_FORK_INDEX.md); later specifications,
implementation plans, and checkpoints supersede this report's status and
next-step wording.

Implementation follow-up:
[Resident Agent Adapter Implementation Plan](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md)
and [Implementation Map](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md)

## Executive summary

The fork should not continue growing the current central Python connector into a
second Buzz client platform.

Buzz now has two upstream-supported seams that map cleanly to the two agent
ownership models Lunar Park actually has:

1. **Buzz-managed agents:** use upstream's merged generic ACP "bring your own
   harness" support. Hermes and OpenClaw are already tier-2 presets:
   `hermes-acp` and `openclaw acp`.
2. **Externally managed resident agents:** use a harness-native channel/platform
   plugin whose Buzz transport is the proposed upstream `buzz listen` NDJSON
   stream plus the existing `buzz` CLI write surface.

That distinction is the clean architectural boundary:

```text
Buzz owns lifecycle                    Harness owns lifecycle
───────────────────────────────────    ──────────────────────────────────
buzz-acp                               OpenClaw channel plugin
  └─ hermes-acp / openclaw acp         Hermes platform-adapter plugin
                                         │
Buzz Desktop creates and supervises       └─ buzz listen + buzz CLI
the runtime.                            Existing resident agent continues
                                       to own memory, sessions, and tools.
```

The existing Python connector proved the protocol and operational model, but it
duplicates Nostr signing, NIP-42 auth, subscriptions, reconnect, event buffering,
webhook delivery, identity storage, and much of `buzz-cli`. Those are precisely
the surfaces now being generalized upstream. It should be treated as a
transition system, not the target architecture.

## Evidence reviewed

### Lunar Park documentation

Reviewed all files under:

`/Users/dspury/Lunar-Park/Lunar-Park-KB/projects/buzz`

The documents correctly identified the core Nostr boundary, but the project
overview and connector plan have drifted:

- The overview records the connector, webhook bridge, legacy OpenClaw webhook
  adapter, and lunar01 deployment as complete.
- The connector document still contains pre-pivot unchecked phases, "host TBD,"
  and a separate gateway design that is no longer the best upstream path.
- The reference documents predate newer managed-agent/persona work and do not
  distinguish all of the current identity records clearly.
- The connector source itself was not available in this local checkout, so its
  implementation was assessed from the KB's detailed as-built description, not
  from direct source inspection.

### Local fork

At the time of review:

- Branch: `main`
- HEAD: `049568ac`
- Working tree: clean before this report
- Fork delta after refreshing remotes: 11 commits ahead of `upstream/main`, 63
  commits behind
- Local app baseline: v0.4.24-era code
- Upstream has v0.4.26 plus newer main changes

The fork delta is 12 files, approximately +611/-515. Most of the functional
delta is concentrated in desktop onboarding and managed-agent startup.

### Current upstream

The most consequential upstream change is merged PR
[#2773](https://github.com/block/buzz/pull/2773), which adds a generic,
data-driven ACP harness catalog. It already includes:

- `hermes` → `hermes-acp`
- `openclaw` → `openclaw acp`
- user-defined harness JSON definitions
- preset/custom discovery, validation, readiness, model discovery, and UI

Two open upstream PRs cover the remaining generic seams:

- [#2633](https://github.com/block/buzz/pull/2633): durable
  channel-to-ACP-session bindings and `session/load`
- [#2942](https://github.com/block/buzz/pull/2942): `buzz listen`, optional
  webhook delivery, compact event fields, self-identity, and headless managed
  agent publication

Open issue [#2663](https://github.com/block/buzz/issues/2663) is the umbrella
request for externally managed, non-ACP agents. Open issue
[#2349](https://github.com/block/buzz/issues/2349) captures the important
separation between remote agent visibility and local runtime ownership.

## What is aligned today

Several decisions in the current fork are sound:

- The relay remains the single source of truth.
- External agents use normal Nostr identities and channel membership.
- Agent reasoning and memory stay in the harness.
- The fork does not add Lunar Park-specific relay endpoints or event kinds.
- The live legacy OpenClaw webhook adapter uses deterministic per-channel
  sessions.
- Responses are mention-gated instead of ambiently consuming every message.
- The fork policy says features built on public relay surfaces belong outside
  the fork.

The protocol reference work is also useful. It established the message,
membership, authentication, and HTTP behavior needed to prove the external-peer
model.

## Where the current implementation diverges from the goal

### 1. The fork is no longer a clean patch queue

`FORK.md` says "config gates over deletions," but the implementation contains
large deletions and no-op replacements:

- `welcomeKickoff.ts` removes most of the upstream choreography.
- `hooks.ts` removes the welcome seeding path instead of gating it.
- voice model prefetch is commented out in `lib.rs`.
- the managed-agent `RespondTo` default is globally changed from `OwnerOnly` to
  `Nobody`.

Those edits are difficult to rebase and unlikely to be accepted upstream as
written. They also change defaults for all users rather than offering a
self-hoster choice.

### 2. The central connector duplicates Buzz-owned transport behavior

The documented connector owns:

- one Nostr client per agent
- key storage and bearer-token delegation
- NIP-42 authentication
- channel subscription and reconnect
- message buffering and catch-up
- webhook registration, retry, parking, and draining
- REST/MCP wrappers over Buzz operations

This was reasonable as a prototype when Buzz had no external-agent stream and
no generic harness catalog. It is now a parallel client stack that must chase
every protocol, auth, membership, threading, and profile change.

### 3. It conflates agent runtime attachment with identity custody

The connector centralizes every agent's Nostr private key. That makes the
connector both a transport gateway and a signing authority. A failure or
compromise affects the whole fleet.

The cleaner rule is:

- the adapter instance responsible for one resident agent owns that Buzz
  identity, or
- a future remote signer owns it.

Upstream issue [#2700](https://github.com/block/buzz/issues/2700) tracks NIP-46
remote signing. Until then, per-agent key locality is the smaller blast radius.

### 4. The current docs treat ACP incompatibility as permanent

That is no longer current:

- Hermes officially supports `hermes acp` / `hermes-acp`.
- OpenClaw officially supports `openclaw acp` as a Gateway-backed ACP bridge.
- Buzz upstream now includes both as preset harnesses.

ACP is appropriate when Buzz owns the turn lifecycle. It is still not the right
choice for every resident fleet agent, because those agents already have their
own messaging lifecycle and continuity model.

### 5. Agent identity records need clearer separation

The current implementation uses several distinct concepts:

- kind `0`: display profile and NIP-OA ownership evidence
- kind `10100`: agent channel-addition policy/profile
- kind `30175`: persona/agent definition
- kind `30177`: owner-authored managed-agent directory record

These are not interchangeable. A resident external agent can participate with a
key, membership, and kind-0 profile; the 30177 record is a directory/control
plane concern. The adapter and docs should not use "agent profile" for all four.

## Proposed target architecture

## Lane A: Buzz-managed ACP harnesses

Use this lane when Buzz Desktop should create, configure, start, stop, and
observe the agent.

```text
Buzz relay
   ↕ Nostr
buzz-acp
   ↕ ACP over stdio
hermes-acp

or

buzz-acp
   ↕ ACP over stdio
openclaw acp
   ↕ OpenClaw Gateway protocol
OpenClaw resident agent
```

Use upstream's harness definitions and Buzz persona records. Do not add
Hermes/OpenClaw-specific runtime branches to Buzz.

Important constraints:

- `buzz-acp` already keeps one ACP session per Buzz channel, but those bindings
  are in memory in current main. PR #2633 is the correct generic fix.
- OpenClaw's ACP bridge can load Gateway-backed sessions, but its
  channel-to-session continuity must be tested across both `buzz-acp` and
  `openclaw acp` restarts.
- Hermes ACP advertises the standard session surface, but its documentation
  describes ACP session management as process-scoped. Durable restart behavior
  must be tested rather than assumed.
- OpenClaw executes tools in the Gateway process. Buzz-injected environment
  variables on the local `openclaw acp` bridge do not automatically appear in
  the Gateway's tool environment.

This lane replaces the claim that Hermes/OpenClaw require a bespoke Buzz
connector. It does not replace the external-resident lane below.

## Lane B: Harness-owned resident agents

Use this lane for Selene, Miles, Henry, and other always-on agents whose
lifecycle, memory, tools, and sessions already belong to OpenClaw or Hermes.

The adapter should be native to the harness:

### OpenClaw

Build a user-installable OpenClaw **channel plugin** named `buzz`.

OpenClaw's channel-plugin surface already owns:

- connection lifecycle
- inbound message ingestion
- outbound sends
- thread mapping
- typing/activity
- platform conversation ID → agent session-key mapping

The plugin should supervise or embed the upstream `buzz listen` stream and map:

```text
(relay authority, agent pubkey, channel UUID, thread root)
    → OpenClaw session key
```

That preserves the useful behavior from the current adapter without a central
gateway. A stable key such as:

`buzz:<community-hash>:<channel-uuid>:<thread-root-or-main>`

keeps channel and thread contexts isolated and recoverable.

### Hermes

Build a user-installable Hermes **platform adapter plugin** named `buzz`.

Hermes explicitly recommends plugins under `~/.hermes/plugins/` for
third-party messaging platforms. Its `BasePlatformAdapter` contract already
provides:

- `connect()`
- `disconnect()`
- `send()`
- optional typing and chat metadata
- inbound `handle_message(event)` routing through the existing gateway runner

This is a direct fit for Buzz. It avoids modifying Hermes core and lets each
resident Hermes instance retain its own local memory, session database, skills,
and delegation behavior.

### Shared transport contract

Both plugins should consume the same small transport contract:

- ingress: `buzz listen --mentions-of-me`
- egress: `buzz messages send --reply-to <event-id> --content -`
- thread context: `buzz messages thread`
- identity: `buzz users me`
- directory registration: `buzz agents publish`
- search/tool access: the existing `buzz` CLI

Those commands are partly in open PR #2942. Until it merges, the current
connector remains the production bridge.

Do not create another general-purpose HTTP API between the plugin and Buzz.
The CLI/NDJSON boundary is process-local, observable, language-neutral, and
upstream-owned.

## Identity and delivery contract

Each resident agent should have:

- one Buzz/Nostr keypair
- one kind-0 display profile
- NIP-OA owner attestation where available
- explicit channel membership
- a 30177 directory record when it should appear as a managed external agent
- a response policy enforced in the harness plugin

Ingress rules:

1. Deduplicate by Buzz event ID.
2. Ignore events authored by the same agent.
3. Require a structured `p`-tag mention by default.
4. Allow an explicit per-channel "always" mode only when configured.
5. Preserve the event ID and NIP-10 thread root through the harness turn.

Delivery should be at-least-once with idempotent handling. The relay remains the
replay source; plugins should persist only a small cursor/dedup checkpoint, not
a second event store.

## Fork realignment plan

### Phase 0: freeze and preserve

- Keep the current lunar01 relay, connector, and legacy OpenClaw webhook
  adapter running.
- Tag the current fork and record the deployed SHAs.
- Do not extend the Python connector except for production break/fix work.
- Export only non-secret identity mappings and session-key conventions for the
  migration.

### Phase 1: rebuild the fork baseline from current upstream

Create a fresh integration branch from current `upstream/main`. Do not start by
rebasing the entire 11-commit patch stack.

Reintroduce only changes that remain necessary after #2773:

- deployment assets that are genuinely reusable
- self-hosting configuration gates
- no Lunar Park hostnames, paths, labels, or launchd identities in upstream PRs

The production launchd plist belongs in Lunar Park deployment/ops material, not
in an upstream feature PR.

### Phase 2: reduce fork behavior to additive gates

Propose separate, reviewable upstream PRs:

1. **Welcome experience gate**
   - gate channel/team/canvas/kickoff creation at the orchestration entry point
   - preserve upstream implementation unchanged behind the enabled path
   - add enabled/disabled tests
2. **Voice model prefetch gate**
   - keep on-demand downloads
   - make startup prefetch configurable
   - do not comment out upstream code
3. **Managed-agent restore policy**
   - prefer the existing per-agent `start_on_app_launch` field
   - if a global self-hoster override is still needed, add it without changing
     the default

Do **not** upstream the global `RespondTo::Nobody` default. Upstream's
`OwnerOnly` plus mention filtering is the compatible safe default. Expose
`Nobody` in the UI only if a real heartbeat-only product use case remains.

### Phase 3: validate the merged BYOH lane

On an upstream-main build:

- create one Hermes persona using the `hermes` preset
- create one OpenClaw persona using the `openclaw` preset
- verify discovery, auth/readiness, prompt, tool use, reply, cancellation, and
  restart
- test #2633's durable session behavior against both harnesses
- file harness-specific bugs only when the generic ACP contract cannot express
  the needed behavior

No new Buzz adapter code should be written before this matrix is complete.

### Phase 4: build native resident-agent plugins

Build two small repos/packages:

- `openclaw-buzz-channel`
- `hermes-buzz-platform`

Share fixtures and a conformance suite rather than a runtime service. The test
suite should feed the same Buzz event fixtures to both plugins and assert the
same normalized inbound/outbound contract.

Suggested normalized envelope:

```json
{
  "event_id": "hex",
  "community": "relay-authority",
  "channel_id": "uuid",
  "thread_root_id": "hex-or-null",
  "parent_event_id": "hex-or-null",
  "author_pubkey": "hex",
  "content": "text",
  "mentioned": true,
  "created_at": 0
}
```

Keep harness-specific session mapping on the plugin side.

### Phase 5: migrate production

1. Run the new plugin in shadow/read-only mode.
2. Compare received event IDs against the current connector.
3. Enable replies for one test agent/channel.
4. Verify duplicate suppression and thread placement.
5. Restart relay, plugin, and harness independently.
6. Expand to Selene, then one Hermes host, then the rest of the fleet.
7. Stop the corresponding connector adapter only after a soak period.
8. Quarantine the connector service/config for rollback before deletion.

## Upstream contribution map

| Concern | Correct home | Action |
|---|---|---|
| Hermes/OpenClaw ACP discovery | `block/buzz` | Already merged in #2773 |
| ACP channel-session durability | `buzz-acp` | Test/review #2633; avoid duplicate PR |
| External realtime event stream | `buzz-cli` | Test/review #2942; avoid duplicate PR |
| Remote agent visibility/mentions | Desktop relay-derived UI | Coordinate with #2349 and #2663 |
| Welcome/voice/autostart choices | Desktop configuration | Small independent upstream PRs |
| OpenClaw resident integration | OpenClaw plugin | New external package |
| Hermes resident integration | Hermes platform plugin | New external package |
| Lunar Park launchd/deployment | Lunar Park ops | Keep out of upstream feature PRs |

## Validation gates

### Buzz-managed ACP lane

- Hermes and OpenClaw runtime discovery from a packaged Desktop build
- ACP initialize, session/new, prompt, cancel, and load
- one Buzz channel never shares context with another
- restart `buzz-acp` without losing or cross-wiring channel sessions
- restart OpenClaw Gateway independently
- verify Hermes memory/session behavior independently from ACP binding recovery
- thread replies land under the triggering event
- no response without the configured mention/author gate

### Resident plugin lane

- initial NIP-42 connection and auth
- relay disconnect/reconnect with backoff
- replay from persisted cursor
- same-timestamp and duplicate-event handling
- Unicode event IDs/signatures
- channel membership changes
- private-channel access denial
- structured mention and self-loop rejection
- thread-root preservation
- plugin/harness restart continuity
- secret redaction and no private keys in logs
- packaged host/service smoke on macOS and Linux

### Fork gates

- default upstream behavior remains byte-for-byte or behaviorally unchanged
- disabled welcome mode creates no built-in team, canvas, kickoff, or Welcome
  channel
- voice models still download on first actual use
- no managed agent starts when its own `start_on_app_launch` is false
- `just ci`
- desktop Tauri tests explicitly, because the root Rust workspace excludes them

## Immediate blockers and risks at the 2026-07-26 assessment

1. **The historical fork head was 63 upstream commits behind.** The later
   planning baseline was refreshed, but `origin/main` still represents a broad
   historical delta rather than the intended clean patch queue.
2. **#2633 is open and review-blocked.** Durable ACP session loading is not yet
   available on main.
3. **#2942 is open and review-blocked.** The clean external-agent stream is not
   yet available on main.
4. **Remote visibility remains imperfect.** Agent membership/profile data and
   local runtime ownership are still coupled in parts of Desktop; #2349 is the
   relevant bug.
5. **Hermes ACP restart semantics need proof.** Do not equate persistent Hermes
   memory with a loadable ACP session ID.
6. **OpenClaw execution environment is remote.** Gateway-side tools need Buzz
   credentials/config at the Gateway locus, not only on the ACP bridge process.
7. **Key custody changes during migration.** Moving from central connector keys
   to per-plugin keys must be deliberate and reversible.
8. **The KB files are currently untracked and partly stale.** Update them only
   after the architecture choice is accepted, so they document the chosen
   design rather than another intermediate state.

## Recommended next step from 2026-07-26 — superseded below

### 2026-07-26 implementation checkpoint

The generic BYOH acceptance matrix is now executable and has been run against
the live harness hosts. OpenClaw 2026.7.1 on lunar01 and Hermes ACP 0.18.2 on
lunar02 both passed ACP initialization and the full Gate 3 relay path:
owner-gated mention, reply, thread placement, cancellation, graceful restart,
and replay deduplication.

The run confirmed the proposed architecture boundary rather than revealing a
need for harness-specific Buzz runtime code. Hermes inherited the canary Buzz
CLI environment through its ACP subprocess. OpenClaw required credentials at
the Gateway execution locus; bridge-process environment alone was insufficient.
See `docs/AGENT_HARNESS_BYOH_ACCEPTANCE.md` for the evidence and retained
test-artifact inventory.

Gate 2's live Desktop flow and Gate 4's durable session continuity remain open.
Hermes advertised `loadSession`, but current `buzz-acp` created a new session
after restart, reinforcing the dependency on generic session-binding work.

Do not implement another adapter in the old fork.

The next Buzz-repo work is the live Desktop Gate 2 flow and review/testing of
#2633 and #2942 as dependencies rather than duplicating them. Once the external
event stream contract is available, the first adapter implementation target
should be the OpenClaw Buzz channel plugin because:

- the live Selene path already provides a known-good E2E oracle;
- OpenClaw has a purpose-built channel plugin SDK;
- the existing adapter's deterministic per-channel keys can be preserved;
- it proves the resident-agent lane before repeating the pattern in Hermes.

## Repo map closeout

- Recommended next file to read:
  the #2633 session-binding patch and #2942 external-listener patch against
  current `upstream/main`.
- Recommended first edit target:
  no new harness-specific runtime code in Buzz; keep acceptance infrastructure
  on the clean upstream-based branch and start the OpenClaw channel plugin as a
  separate package once its generic event-stream dependency is ready.
- Recommended first validation command:
  `. ./bin/activate-hermit && just agent-harness-acceptance`, followed by the
  live Desktop Gate 2 flow.

### 2026-07-27 resident-plugin checkpoint

The architecture decision was carried forward:

- WP1-WP3 generic Buzz CLI and fixture slices were implemented on focused,
  pushed fork branches.
- The OpenClaw plugin was implemented in the standalone
  `/Users/dspury/Lunar-Park/integrations/openclaw-channel-buzz` repository.
- The plugin owns its SQLite delivery state and uses only public OpenClaw
  plugin SDK contracts.
- An isolated lunar01 canary passed owner/policy/thread/replay/restart and
  accepted-but-uncertain-send cases without sharing reply authority with the
  legacy connector.

Checkpoint C did not pass overall. A valid OpenClaw turn completed without a
final visible text reply; the adapter left the activation pending and the
account supervisor repeatedly restarted it. The Buzz canary channel was
disabled. Discord and the existing connector remained healthy.

This adds one current blocker to the report: no-final turn completion needs a
bounded durable terminal policy and regression test before the canary can be
re-enabled. Hermes implementation and production connector cutover have not
started.

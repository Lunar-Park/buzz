# Resident Agent Adapter Implementation Plan

Status: active. The corrected narrow OpenClaw no-final canary passed on
2026-07-29 and was disabled afterward. The full corrected matrix, current
Desktop-to-Selene round trip, Hermes implementation, and connector cutover
remain incomplete. The accepted first-class expansion now also includes
primary-first durable-agent selection and native direct-message routing after
the single-Selene gate.

Updated: 2026-07-29

Architecture authority:
[Resident Agent Adapter Implementation Map](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md)

Product authority:
[Self-Hosted Agent Integration Specification](SELF_HOSTED_AGENT_INTEGRATION_SPEC.md)

## Objective

Deliver clean, independently maintainable Buzz integrations for resident
OpenClaw and Hermes agents while keeping `block/buzz` harness-neutral and
upstreamable.

The finished system has three deliverables:

1. a stable generic external-agent CLI contract in Buzz;
2. a native OpenClaw Buzz channel plugin;
3. a native Hermes Buzz platform plugin.

The current central Python connector remains available for rollback during
migration but receives no new feature work.

## Definition of Done

The work is complete when:

- generic Buzz changes are separated into reviewable upstream PRs;
- neither plugin requires an OpenClaw-, Hermes-, or Lunar Park-specific branch
  in Buzz;
- each resident agent owns its Buzz identity at its harness execution locus;
- owner-only and mention-required policies fail closed;
- each accepted Buzz event produces at most one visible plugin reply during
  ordinary delivery, reconnect, plugin restart, and host restart;
- replies preserve the Buzz channel, immediate parent, and NIP-10 thread root;
- selected durable harness agents retain independent Buzz identities and
  OpenClaw agent/session namespaces;
- Buzz DMs use the stable DM channel UUID and each harness's native direct
  session routing rather than an adapter-owned parallel session system;
- OpenClaw on lunar01 and Hermes on lunar02 both pass the shared conformance
  suite;
- the old connector can be stopped without breaking either native plugin;
- rollback instructions and retained artifacts have been tested;
- documentation names the authoritative implementation, deployment, and
  operational locations.

## Constraints

- Start all Buzz branches from refreshed `upstream/main`.
- Do not implement on the current mixed working tree.
- Do not rebase or delete the preserved pre-alignment history.
- Activate Hermit before Git, Rust, Node, or repo hook commands.
- Do not add new Buzz event kinds or HTTP endpoints for the adapter.
- Do not place resident-agent private keys in the Buzz repository, fixtures,
  plugin manifests, logs, or kind `30177`.
- Do not upgrade OpenClaw, Hermes, or a production service as an incidental
  implementation step.
- Do not give the old connector and a native plugin simultaneous reply
  authority for the same identity.
- Treat PR #2633 as an independent ACP-lane change.
- Coordinate with PR #2942 instead of opening an overlapping competing PR
  without first attempting review/author coordination.

## Recommended Repository Layout

Use separate repositories or top-level projects:

```text
/Users/dspury/Projects/buzz/
  # Buzz fork and upstreamable generic CLI work

/Users/dspury/Lunar-Park/integrations/openclaw-channel-buzz/
  # OpenClaw plugin source

/Users/dspury/Lunar-Park/integrations/hermes-platform-buzz/
  # Hermes plugin source
```

The OpenClaw repository location has been approved and created. The Hermes
location remains the planned next repository after the OpenClaw canary. These
homes keep Lunar Park deployment concerns outside the Buzz fork.

Canonical contract fixtures should originate in Buzz:

```text
crates/buzz-cli/tests/fixtures/external_agent_v1/
  owner_mention.json
  non_owner_mention.json
  nested_reply.json
  self_authored.json
  malformed_tags.json
  duplicate_event.json
  lifecycle_connected.json
  lifecycle_eose.json
  lifecycle_disconnected.json
```

Each plugin vendors the fixtures plus a text file containing the source Buzz
commit:

```text
test/fixtures/external_agent_v1/
UPSTREAM_FIXTURE_COMMIT
```

Fixtures are shared test data, not a shared runtime package or network service.

## Workstream and Dependency Graph

```text
WP0 Baseline and decisions
  │
  ├── WP1 Buzz identity/result prerequisites
  │      │
  │      └── WP2 Buzz listen v1 contract
  │              │
  │              ├── WP3 Shared conformance fixtures
  │              │      │
  │              │      ├── WP4 OpenClaw compatibility spike
  │              │      │      └── WP5 OpenClaw MVP
  │              │      │              └── WP5A OpenClaw roster + DM parity
  │              │      │
  │              │      └── WP6 Hermes compatibility spike
  │              │             └── WP7 Hermes MVP
  │              │
  │              └── WP8 Optional agent directory publication
  │
  └── WP9 Identity and Desktop visibility audit

WP5 + WP5A + WP7 + WP9
  └── WP10 Shadow deployment
          └── WP11 Per-agent cutover
                  └── WP12 Connector quarantine and closeout
```

WP4 and WP6 may proceed in parallel after the CLI schema and fixture shape are
stable. Production deployment remains sequential.

## Current Execution State

This plan has already produced the first upstreamable Buzz slices in clean
worktrees:

| Work package | Branch/worktree | Current result |
|---|---|---|
| WP1 | `/Users/dspury/Projects/buzz-pr-identity` on `buzz-pr-identity` | pushed to `Lunar-Park/buzz`; head `e14a94ec3` |
| WP2 | `/Users/dspury/Projects/buzz-pr-listen` on `buzz-pr-listen` | pushed to `Lunar-Park/buzz`, stacked on WP1; head `1d65784d5` |
| WP3 | `/Users/dspury/Projects/buzz-pr-fixtures` on `buzz-pr-fixtures` | pushed to `Lunar-Park/buzz`, stacked on WP2; head `68513902c` |
| WP4 | lunar01 read-only compatibility spike | OpenClaw `2026.7.1 (5f39975)` is compatible enough to proceed without upgrade |
| WP5 | `/Users/dspury/Lunar-Park/integrations/openclaw-channel-buzz` | local head `3e4e73a`; installed source `aebd8dd`; 22 tests pass; corrected narrow no-final canary passed and was disabled; full corrected matrix remains open |
| WP5A | OpenClaw plugin expansion | specified; deferred until the single-Selene communication gate passes |
| WP6 | lunar02 read-only compatibility spike | Hermes `0.18.2 (2026.7.7.2)` is compatible enough to proceed without upgrade |
| WP7 | planned `/Users/dspury/Lunar-Park/integrations/hermes-platform-buzz` | not started; repository does not exist |

Upstream PR publication is paused at the coordination gate. PR
[#2942](https://github.com/block/buzz/pull/2942) overlaps WP1 and WP2, and PR
[#2933](https://github.com/block/buzz/pull/2933) overlaps the identity slice.
The corrected branch split and validation evidence were posted to PR #2942 in
[this coordination comment](https://github.com/block/buzz/pull/2942#issuecomment-5087842025).
Live GitHub state was refreshed on 2026-07-29: #2633 (`7b76647e7`), #2933
(`837958d5f`), and #2942 (`46987a0fe`) are all still open. Re-evaluate #2942's
listener, lifecycle envelope, replay, compact-event, and executable-fixture
contracts against that current head before publishing or restating the older
review findings. Treat WP1-WP3 as tested fork evidence, not a ready upstream
submission, until coordination and rebase are complete.
Do not open a competing upstream PR until the author responds or the user
explicitly approves a replacement path after coordination fails.

The main checkout at `/Users/dspury/Projects/buzz` remains a mixed planning
worktree and must not be used for implementation commits. Keep coding work in
the focused worktrees or in separately approved plugin repositories.

### Focused branch snapshot — 2026-07-29

The focused WP1-WP3 and RC1-RC4 worktrees are clean, pushed, and match their
`origin/*` refs. They contain `upstream/main` `22be8bb35` in their ancestry.
Refreshed upstream is `3e48f1b23`, 27 commits ahead of that baseline:

| Slice | Branch head | Shape |
|---|---|---|
| WP1 identity | `buzz-pr-identity` @ `e14a94ec3` | two commits over upstream |
| WP2 listen | `buzz-pr-listen` @ `1d65784d5` | WP1 plus two commits |
| WP3 fixtures | `buzz-pr-fixtures` @ `68513902c` | WP2 plus two commits |
| RC1 keys | `buzz-rc-keys` @ `26bb74aa2` | one independent commit |
| RC2 profile | `buzz-rc-profile` @ `ce69cc3fe` | one independent commit |
| RC3 discovery | `buzz-rc-discovery` @ `ad1c5a334` | two independent commits |
| RC4 connected agents | `buzz-rc-custody` @ `098711146` | RC3 plus one feature commit |

The current integrated Desktop test branch is separate:

| Slice | Branch head | Shape |
|---|---|---|
| OpenClaw integration | `lunar-park/openclaw-test-integration-2026-07-29` @ `b19fd1508` | 13 commits over its upstream base; WP1-WP3 + RC1-RC4 + channel/team integration |

It is clean, local-only, and eight commits behind refreshed `upstream/main`.

The old integration worktree `/Users/dspury/Projects/buzz-upstream-merge` remains
clean at `d53ccc486`, but it is now historical evidence only. It is 47 commits
behind and 27 commits ahead of refreshed `upstream/main`, and contains the
discarded RC4 design that put `KeyCustody` on `ManagedAgentRecord`. Do not
validate, publish, or continue implementation from that merge. Rebuild any
future integration candidate from the current test point and focused branch
heads above.

### Remote Agent Connect companion state

The peer
[Remote Agent Connect Implementation Plan](REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md)
now has RC1-RC4 implemented locally:

- RC1 `buzz keys generate`: implemented and tested on `buzz-rc-keys`;
- RC2 `buzz agents profile`: implemented and tested offline on
  `buzz-rc-profile`; live relay side-effect and Desktop rendering checks remain;
- RC3 host-aware discovery: implemented on `buzz-rc-discovery` and validated
  read-only against lunar01 and lunar02;
- RC4 structurally separate connected-agent storage and Connected Agents UI:
  implemented on `buzz-rc-custody`.

These changes improve identity custody, discovery, and Desktop visibility, but
they do not replace WP5/WP7 transport. The remaining RC validation is the first
real connected-agent communication/restart runbook. The integrated Desktop
branch has already proved lunar01 SSH/OpenClaw discovery, public-only
connection, normal agent presentation, channel attachment, mentions, and team
membership for the isolated Selene canary identity. It has not yet produced a
reply through WP5.

RC4 and WP5 deliberately meet only at the public identity and harness label:

- RC4 stores `pubkey`, local name, SSH host alias, and observed harness in
  `connected-agents.json`; it has no nsec, command, pid, or lifecycle controls.
- WP5 runs on the resident host, holds its own Buzz secret through OpenClaw
  configuration, consumes `buzz listen`, and owns durable reply/reaction
  delivery.
- Connecting or disconnecting in Desktop emits no owner-managed kind:30177
  claim or tombstone. The resident publishes its own kind:10100 profile through
  RC2 and remains responsible for its process.
- Buzz channel membership does not currently reconfigure WP5's `channelIds`.
  The communication gate must verify both relay membership and adapter
  subscription before it declares the connected agent ready.
- RC5 and RC6 are accepted but deferred expansions: primary-first selective
  enrollment of durable OpenClaw agents and direct-message delivery through the
  existing OpenClaw session router.

## WP0: Baseline, Branch Hygiene, and Decisions

Purpose: create a safe implementation starting point without disturbing the
current evidence and acceptance work.

### Tasks

1. Record the current working tree, branch, tags, and remote SHAs.
2. Preserve the current uncommitted documentation and acceptance assets in a
   dedicated documentation commit or explicitly approved patch.
3. Fetch `upstream/main` and the current heads of PR #2633 and PR #2942.
4. Inspect whether PR #2942's author accepts review suggestions, commits, or a
   proposed split.
5. Create separate worktrees from `upstream/main`:

   ```text
   buzz-pr-identity
   buzz-pr-listen
   buzz-pr-agent-publish
   ```

6. Choose the permanent plugin repository names and local paths.
7. Pin the starting runtime matrix:

   ```text
   Buzz upstream commit
   buzz CLI contract version
   OpenClaw 2026.7.1 / 5f39975
   Hermes 0.18.2 / 0fa5e41c
   ```

8. Decide initial subscription policy:

   ```text
   selected channels AND explicit p-tag mention
   owner-only author policy
   ```

### Decisions recommended by this plan

- Develop against the installed Hermes `0.18.2` API first; do not combine a
  Hermes upgrade with adapter development.
- Run a time-boxed OpenClaw SDK compatibility spike before deciding whether an
  OpenClaw upgrade is necessary.
- Keep `owner-only` and `require_mention=true` as MVP defaults.
- Limit MVP subscriptions to explicitly configured channel UUIDs.
- Keep kind `30177` optional and outside transport-critical work.

### Exit criteria

- no implementation is occurring in the mixed alignment worktree;
- every branch has one purpose and an identified upstream PR destination;
- plugin repository locations are approved;
- PR #2942 coordination path is recorded;
- runtime versions and the subscription policy are frozen for MVP.

### Approval checkpoint A

User approval is required before:

- creating new repositories;
- publishing branches or PR comments;
- changing another contributor's PR;
- upgrading either harness.

## WP1: Buzz Identity and Result Prerequisites

Purpose: land the smallest generic additions that external processes need
regardless of streaming.

### Buzz change surface

```text
crates/buzz-cli/src/commands/users.rs
crates/buzz-cli/src/commands/messages.rs
crates/buzz-cli/src/client.rs
crates/buzz-cli/src/lib.rs
docs/cli-external-agents.md
```

### Scope

- add `buzz users me`;
- retain `pubkey` and `tags` in compact message output;
- preserve existing JSON output compatibility;
- document stdout, stderr, exit codes, and secret handling;
- add unit and CLI integration coverage.

### Explicit exclusions

- `buzz listen`;
- webhook delivery;
- kind `30177`;
- plugin-specific options;
- resident-agent policy enforcement.

### Validation

```bash
. ./bin/activate-hermit
cargo fmt --all -- --check
cargo clippy -p buzz-cli --all-targets -- -D warnings
cargo test -p buzz-cli users
cargo test -p buzz-cli messages
```

Also run a process-level assertion that `buzz users me` performs no relay
request and never prints the private key.

### Exit criteria

- exact self pubkey and npub are emitted;
- compact output preserves the fields required for self-suppression,
  authorization, mentions, and threading;
- the change is independently mergeable.

### Upstream unit

PR A: identity and compact event prerequisites.

## WP2: Buzz Listen v1 Contract

Purpose: define a stable, replay-aware, language-neutral ingress contract.

### Buzz change surface

```text
crates/buzz-cli/src/commands/listen.rs
crates/buzz-cli/src/commands/mod.rs
crates/buzz-cli/src/lib.rs
crates/buzz-cli/src/client.rs
crates/buzz-ws-client/
docs/cli-external-agents.md
crates/buzz-cli/tests/
```

Only change `buzz-ws-client` if the lifecycle or EOSE contract cannot be
implemented cleanly through its current public surface.

### CLI contract

Recommended plugin invocation:

```bash
buzz listen \
  --channel "$CHANNEL_UUID" \
  --mentions-of-me \
  --since "$REPLAY_SINCE" \
  --envelope v1 \
  --no-reconnect
```

The plugin, not the CLI, owns restart and replay-cursor advancement. This avoids
two nested reconnect supervisors and lets each new process start from the most
recent durable checkpoint.

### Filter semantics

Define flags as:

- `--channel` only: events in any configured channel;
- `--mentions-of-me` only: mentions in any visible channel;
- both: events satisfying channel **and** mention filters.

Do not construct the current two-filter OR union when both flags are present.
Add direct tests for the combined case.

### Versioned envelope

Keep the existing flat event output only as a compatibility mode. Plugins use:

```json
{
  "schema_version": 1,
  "type": "event",
  "event": {
    "id": "<event id>",
    "pubkey": "<author>",
    "kind": 9,
    "content": "hello",
    "created_at": 1785100000,
    "tags": [["h", "<channel>"], ["p", "<agent>"]]
  }
}
```

Lifecycle records use the same envelope:

```json
{
  "schema_version": 1,
  "type": "lifecycle",
  "state": "eose"
}
```

Allowed MVP lifecycle states:

```text
connected
eose
closed
fatal
```

Human-readable diagnostics remain on stderr. Stdout contains NDJSON only.

### Replay contract

- `--since` accepts Unix seconds and maps to the Nostr filter.
- Plugins persist the maximum admitted `created_at`.
- Restart begins from `max(0, last_created_at - 60)`.
- Events sharing a timestamp are disambiguated by event-ID deduplication.
- EOSE separates retained history from live events.
- The event ID, not timestamp, is the idempotency identity.

### Backpressure and shutdown

- flush every stdout record;
- never silently discard a record because the consumer is slow;
- close the Nostr subscription on SIGINT/SIGTERM;
- exit nonzero for fatal auth/config/protocol errors;
- do not emit a lifecycle `closed` record that falsely implies all events were
  durably consumed by the downstream plugin.

### Tests

Unit:

- argument and kind parsing;
- AND filter construction;
- envelope serialization;
- replay overlap calculation;
- lifecycle serialization.

Process integration:

- stdout purity;
- stderr diagnostics;
- EOSE ordering;
- slow stdout consumer;
- SIGINT and SIGTERM;
- relay drop;
- `--no-reconnect`;
- restart with overlap;
- duplicate event replay.

Live:

- NIP-42 authentication;
- canonical host-derived community boundary;
- one owner mention received;
- nonmatching channel excluded;
- self-authored event still delivered by transport when it matches, leaving
  self-suppression to the adapter contract.

### Exit criteria

- v1 schema is documented and fixture-backed;
- reconnect ownership is unambiguous;
- no full-history replay is required after a checkpoint;
- combined channel/mention filtering is proven;
- failures are machine-classifiable without parsing human logs.

### Upstream unit

PR B: stable external-agent listen contract.

If PR #2942 cannot be updated or split cooperatively, carry a temporary local
integration branch based on its head for plugin work. Do not open a competing
upstream PR until coordination has failed and the user approves that path.

## WP3: Shared Conformance Fixtures

Purpose: make the two plugins prove the same behavior without sharing runtime
code.

### Deliverables

- Buzz-side v1 JSON fixtures;
- a small schema description;
- expected normalized facts for each fixture;
- a fixture sync/check script in each plugin repository;
- `UPSTREAM_FIXTURE_COMMIT` pin in each plugin.

### Required fixture cases

1. owner top-level mention;
2. owner direct reply to agent;
3. owner nested thread mention;
4. non-owner mention;
5. allowlisted sender;
6. self-authored message;
7. unsupported kind;
8. missing `h` tag;
9. multiple conflicting `h` tags;
10. malformed `e` marker;
11. duplicate event ID;
12. same-second distinct event IDs;
13. connected/EOSE/closed lifecycle;
14. malformed JSON line;
15. unknown future schema version.

### Expected normalized facts

```text
event_id
author_pubkey
channel_id
immediate_parent_id
thread_root_id
explicitly_mentions_agent
is_self_authored
author_policy_result
activation_result
conversation_lane
```

### Exit criteria

- TypeScript and Python tests consume byte-identical JSON fixtures;
- unsupported schema versions fail closed;
- every ignored event has a stable reason code;
- fixture changes require a Buzz commit-pin update in both plugins.

## WP4: OpenClaw Compatibility Spike

Purpose: prove the exact native extension seam on lunar01 before building the
plugin around an assumed SDK.

### Scope

Create a disposable, non-enabled plugin outside the live OpenClaw plugin
directory and prove imports against `/opt/homebrew/bin/openclaw` version
`2026.7.1`.

Verify:

- import-safe channel declaration;
- account config and SecretRef access;
- channel message adapter;
- `resolveSessionConversation`;
- outbound `MessageReceipt`;
- shared outbound-echo registry;
- durable ingress using installed `createDurableInboundReceiveJournal` with an
  adapter-owned store beneath OpenClaw's resolved state directory;
- plugin status/health surface;
- start/stop hooks without starting the live agent.

### Decision rule

- If all required behavior is expressible through public 2026.7.1 exports,
  pin that version and proceed.
- If durable state requires internal/minified imports or host-only runtime
  state methods, stop. Use an adapter-owned store through public contracts or
  draft a separate OpenClaw upgrade plan and request approval.
- Do not use process-memory-only deduplication as a compatibility fallback.

### Validation

- typecheck against the installed SDK;
- load the plugin in an isolated OpenClaw config root;
- prove no listener starts during metadata/setup imports;
- prove accepted test ingress survives process restart.

### Exit criteria

- a written compatibility decision;
- exact allowed SDK import paths;
- no import from hashed `dist/*.js` internals;
- no live Gateway or service modification.

### Current result

Status: compatible with an external-plugin storage correction. OpenClaw
`2026.7.1` exposes `openKeyedStore`, `openSyncKeyedStore`, and
`openChannelIngressQueue` in the runtime type but rejects those methods for
ordinary third-party plugins. The adapter therefore owns its SQLite delivery
state beneath `runtime.state.resolveStateDir()` and continues to use the public
`createDurableInboundReceiveJournal` facade.

Evidence gathered on lunar01:

- `/opt/homebrew/bin/openclaw --version` reports `OpenClaw 2026.7.1
  (5f39975)`;
- `/opt/homebrew/bin/openclaw` resolves to
  `/opt/homebrew/lib/node_modules/openclaw/openclaw.mjs`;
- Node is `v26.4.0` and npm is `11.17.0`;
- public package exports include the required plugin SDK paths;
- a disposable plugin installed into an isolated `OPENCLAW_STATE_DIR` and
  `OPENCLAW_CONFIG_PATH`;
- runtime inspection loaded the disposable channel plugin with one metadata
  warning only and reported channel capability `buzz`;
- no live OpenClaw Gateway or service configuration was modified.

Allowed public import paths confirmed:

- `openclaw/plugin-sdk/plugin-entry`;
- `openclaw/plugin-sdk/channel-config-helpers`;
- `openclaw/plugin-sdk/channel-outbound`;
- `openclaw/plugin-sdk/channel-message-runtime`;
- `openclaw/plugin-sdk/channel-message`;
- `openclaw/plugin-sdk/secret-ref-runtime`;
- `openclaw/plugin-sdk/conversation-runtime`;
- `openclaw/plugin-sdk/conversation-binding-runtime`;
- `openclaw/plugin-sdk/thread-bindings-runtime`;
- `openclaw/plugin-sdk/dedupe-runtime`;
- `openclaw/plugin-sdk/outbound-runtime`;
- `openclaw/plugin-sdk/reply-runtime`;
- `openclaw/plugin-sdk/plugin-runtime`.

Implementation decisions from the spike:

- use `definePluginEntry` and `api.registerChannel({ plugin })`;
- include `openclaw.extensions` in `package.json`;
- declare `channels`, `channelConfigs`, `configSchema`, and `contracts` in
  `openclaw.plugin.json`; omit `kind` because OpenClaw 2026.7.1 reserves that
  field for `memory` and `context-engine` plugins;
- build channel `config` with public channel config helpers such as
  `createTopLevelChannelConfigAdapter`, rather than hand-rolled partial helper
  objects;
- use durable receive journal helpers from
  `openclaw/plugin-sdk/channel-outbound`; do not import hashed `dist/*.js`
  internals;
- do not call host-managed keyed-store or ingress-queue methods from the
  external Buzz plugin; they require bundled or officially trusted status in
  `2026.7.1`;
- do not depend on a helper named `createChannelIngressMonitor`; it was not
  present in the installed 2026.7.1 public export surface.

WP4 follow-through completed in the permanent local plugin repository:

- strict TypeScript typecheck passes against the pinned OpenClaw 2026.7.1
  development dependency;
- the real plugin and channel manifest load without diagnostics in isolated
  OpenClaw roots locally and against lunar01's patched 2026.7.1 installation;
- import-safety, durable duplicate replay, and restart reconciliation tests
  pass without starting a live lunar01 listener or changing the Gateway.

### Approval checkpoint B

If the spike requires an OpenClaw upgrade, stop for approval. The upgrade must
be a separate reversible operation because lunar01 uses a live, potentially
patched OpenClaw build.

## WP5: OpenClaw Channel Plugin MVP

Purpose: implement one production-quality Buzz text channel in OpenClaw.

### Current first-slice files

```text
package.json
openclaw.plugin.json
src/config.ts
src/conversations.ts
src/errors.ts
src/index.ts
src/ingress.ts
src/runtime.ts
src/state.ts
src/transport.ts
src/types.ts
test/fixtures/external_agent_v1/
test/config.test.ts
test/conversations.test.ts
test/import-safety.test.ts
test/ingress.contract.test.ts
test/runtime.test.ts
test/state.test.ts
test/transport.test.ts
```

### Implementation order

1. Scaffold an import-safe plugin and config schema.
2. Resolve one account's relay URL, key SecretRef, auth tag, owner pubkey, and
   allowed channels.
3. Run `buzz users me` and verify the resolved account pubkey at startup.
4. Supervise `buzz listen --envelope v1 --no-reconnect`.
5. Parse and durably admit each event before reading the next record.
6. Normalize tags and evaluate policy/activation.
7. Map `(account, channel, thread root)` through
   `resolveSessionConversation`.
8. Dispatch through OpenClaw's current inbound channel pipeline.
9. Send final text through `buzz messages send --reply-to`.
10. return/store the Buzz event ID as the outbound receipt.
11. Expose account health, queue depth, cursor, last event, and last error.
12. Implement clean stop: settle admissions, stop listener, await durable
    drain, then close state.

### MVP capabilities

- one or more separately configured Buzz accounts;
- kinds `9` and `40002`;
- text only;
- owner-only or explicit allowlist;
- explicit mention required;
- top-level and threaded replies;
- durable ingress and restart recovery;
- no DMs, media, reactions, edits, typing, or presence.

### Failure behavior

- CLI not found: account unhealthy; no retry storm;
- authentication failure: fatal configuration state;
- malformed NDJSON: quarantine line, mark transport unhealthy, restart with
  bounded backoff;
- unsupported schema: fail closed;
- disk failure: reject admission and restart listener from last durable cursor;
- OpenClaw turn failure: retain failed/pending row with bounded retry policy;
- uncertain Buzz send: do not regenerate the model response automatically;
  reconcile before retrying visible delivery.

### Tests

- shared fixture contract;
- config and secret redaction;
- one serialized lane per thread;
- concurrent different-channel lanes;
- self-echo suppression;
- duplicate replay;
- crash after admission;
- crash after harness adoption;
- crash before/after outbound receipt persistence;
- listener exit and bounded restart;
- plugin stop during active admission;
- import-safe discovery.

### Exit criteria

- isolated local tests pass;
- one lunar01 canary account passes owner mention and nested-thread cases;
- plugin restart does not duplicate the reply;
- existing OpenClaw non-Buzz channels remain unaffected;
- old connector remains available but does not share the canary identity.

### Current result

Status: implemented locally through source commit `aebd8dd`; documentation head
`3e4e73a` records the corrected narrow canary result. The plugin repository has
no remote, so all commits remain local-only. Connector cutover was not
performed.

Implemented:

- one top-level Buzz account with SecretRef resolution and startup identity
  verification;
- supervised `buzz listen --envelope v1 --no-reconnect` transport;
- Buzz-owned v1 fixture normalization and fail-closed local policy;
- adapter-owned SQLite ingress journal, cursor, and prepared/sent outbound
  state beneath OpenClaw's resolved state root;
- stable channel/thread conversation lanes;
- one buffered final-text turn and `buzz messages send --reply-to`;
- one silent-turn outcome using a persisted acknowledgement reaction;
- uncertain-send thread reconciliation before any resend;
- bounded deterministic failure with recorded code, message, and attempt count;
- retryable failures remain pending and never enter the deterministic
  dead-letter path;
- import, fixture, duplicate replay, restart, config, lane, and self-echo tests.

The local package typechecks and all 22 tests pass. The suite
exercises the real OpenClaw no-final dispatcher outcome plus hermetic Buzz CLI
subprocess contracts for message sends and reaction add/get, including exact
arguments and the stdout/stderr boundary. It also loads in an isolated OpenClaw
config/state root with channel ID `buzz` and no diagnostics.
On the laptop, an ordinary locally linked plugin starts under a real OpenClaw
`2026.7.1 (2d2ddc4)` gateway, reopens its database after restart, and shuts down
cleanly without first-party trust.

Live Checkpoint C passed:

- owner top-level and nested-thread replies;
- non-owner, missing-mention, and self-authored rejection;
- completed-event replay suppression;
- recovery after durable admission and Gateway restart;
- accepted-but-uncertain send reconciliation without duplicate delivery;
- continued Discord and legacy connector health.

The original 2026-07-27 matrix did not pass overall. A valid owner event
completed without final text, remained pending, and repeatedly restarted the
account. The corrected candidate now proves:

- the real OpenClaw buffered dispatcher maps completion without a final
  delivery to the explicit silent outcome;
- a silent turn completes the ingress record and posts one reaction;
- replay and adapter restart do not post a second reaction or regenerate the
  model turn;
- an accepted-but-uncertain reaction is reconciled before resend;
- the real Buzz CLI subprocess boundary invokes and parses
  `buzz reactions add/get`;
- a deterministic per-event failure reaches a durable dead letter at the
  configured bound rather than remaining pending forever;
- retryable relay, state, harness, and uncertain-send failures stay pending
  without cursor advance or event loss.

Local validation on the current commits:

```text
npm run typecheck
  -> passed
npm test
  -> 22 passed
npm audit --omit=dev
  -> 0 vulnerabilities
npm pack --dry-run
  -> passed
```

The corrected narrow live rerun passed on 2026-07-29:

- a fresh isolated private channel received an exact `NO_REPLY` stimulus;
- the turn completed as one persisted `reaction` outcome;
- one canary-signed `👀` reaction appeared and no text reply appeared;
- no pending row, duplicate reaction, duplicate model turn, or duplicate
  outbound record appeared after a deliberate Gateway restart;
- Discord, Gateway connectivity, and both legacy health endpoints remained
  healthy;
- the candidate was disabled after the run.

The live state was re-verified read-only during this documentation pass:

- lunar01 OpenClaw remains `2026.7.1 (5f39975)`;
- `channels.buzz.enabled` remains `false`;
- plugin ID `buzz` is loaded without diagnostics from
  `/Users/selene/Lunar-Park/integrations/openclaw-channel-buzz-aebd8dd`;
- no production identity or connector authority changed.

Remaining before WP5 exit:

- configure the exact isolated identity and Desktop test channel without
  overlapping legacy reply authority;
- complete one Desktop-to-Selene text round trip and restart recovery;
- rerun the complete corrected Checkpoint C matrix under its separate review
  point;
- configure a plugin remote before publication;
- implement multi-account config, identity-to-agent routing, and concurrent
  per-lane processing only for RC5 selective enrollment, after the
  single-Selene gate passes.

Checkpoint C evidence and the reusable live/rollback runbook are recorded in
`/Users/dspury/Lunar-Park/integrations/openclaw-channel-buzz/docs/CHECKPOINT_C.md`.

## WP5A: OpenClaw Roster and Direct-Message Parity

Status: **specified and deferred** until WP5 completes one Desktop-to-Selene
text round trip and proves restart recovery without a duplicate.

This package contains two independent expansions that may proceed in either
order after that gate.

### Selective durable-agent routing

- Enumerate OpenClaw's durable configured agent roster; exclude ephemeral
  per-turn workers.
- Mark the primary or `main` agent as the default candidate while returning the
  complete roster to Buzz.
- Support one Buzz account and signing identity per selected OpenClaw agent.
- Map each account to one exact OpenClaw `agentId`; never let a parent or
  sibling sign the selected agent's reply.
- Add concurrent per-conversation lanes without allowing identity, retry,
  cursor, or restart state to cross accounts.

### Direct-message routing

Buzz represents a DM as a stable channel UUID. OpenClaw 2026.7.1 accepts a
`direct` peer and applies `session.dmScope` when it builds the session key.
The plugin therefore extends the existing durable pipeline rather than adding
a second session store:

1. receive a Buzz-owned normalized DM envelope after NIP-17 decryption;
2. use the Buzz DM channel UUID as the stable direct peer ID;
3. preserve an optional Buzz thread root as thread context;
4. call `resolveAgentRoute` with `peer.kind = "direct"`;
5. respect the configured `main`, `per-peer`, `per-channel-peer`, or
   `per-account-channel-peer` scope;
6. prepare, send, and reconcile the reply in the same Buzz DM conversation.

The default OpenClaw `main` DM scope is valid for a single-owner deployment.
Readiness must report an unsafe configuration when multiple permitted DM peers
would silently share that context.

### Exit criteria

- primary-first discovery does not enroll the rest of the roster;
- two selected durable agents use distinct keys, agent IDs, sessions, journals,
  and replies;
- one encrypted owner DM produces one reply authored by the selected identity
  in the same Buzz DM;
- session continuity/isolation matches `session.dmScope`;
- restart and replay produce no duplicate model turn or visible delivery;
- decrypted DM content is absent from logs and relay-searchable events.

## WP6: Hermes Compatibility Spike

Purpose: prove the plugin against lunar02's installed Hermes rather than current
documentation alone.

### Verified starting surface

Hermes `0.18.2` currently contains:

- `BasePlatformAdapter`;
- `MessageEvent.message_id`;
- reply fields;
- threaded `SessionSource`;
- `handle_message()`;
- `ctx.register_platform()`.

### Spike tasks

- load a plugin from an isolated Hermes config/plugin root;
- register a `buzz` platform without core changes;
- construct `MessageEvent` with Buzz message and thread facts;
- verify Hermes' session key for channel and thread;
- verify `send(chat_id, content, reply_to, metadata)` receives the original
  event ID and thread metadata;
- verify clean connect/disconnect;
- verify required env/config entries without printing secret values.

### Decision rule

- Proceed on `0.18.2` if the public plugin surface supports the MVP.
- If not, stop and draft a separate Hermes upgrade plan.
- Do not modify Hermes core as an adapter shortcut.

### Exit criteria

- exact import paths and fields are pinned;
- a no-network fake adapter round-trip passes;
- no live gateway configuration has changed.

### Current result

Status: compatible enough to proceed with the Hermes MVP skeleton on Hermes
`0.18.2 (2026.7.7.2)` without a Hermes upgrade.

The 2026-07-28 read-only host check reconfirmed that version and the
`ai.hermes.gateway` LaunchAgent. No Hermes-Buzz plugin repository or installed
plugin directory exists on the laptop or lunar02.

Evidence gathered read-only on lunar02:

- `/Users/miles/.local/bin/hermes --version` reports `Hermes Agent v0.18.2
  (2026.7.7.2) · upstream 0fa5e41c`;
- the install directory is `/Users/miles/.hermes/hermes-agent`;
- the running gateway is supervised as `ai.hermes.gateway` and executes
  `/Users/miles/.hermes/hermes-agent/venv/bin/python -m hermes_cli.main
  gateway run --replace`;
- `hermes plugins --help` exposes install/update/remove/list/enable/disable;
- `gateway/platforms/ADDING_A_PLATFORM.md` documents the third-party platform
  path: create `plugin.yaml` plus `adapter.py`, subclass
  `BasePlatformAdapter`, and register via `ctx.register_platform()` without
  Hermes core changes;
- `PluginContext.register_platform()` forwards platform registration into
  `gateway.platform_registry.PlatformEntry`;
- plugin platform names are dynamic `Platform` pseudo-members, but only after
  a platform registry entry exists.

Pinned public implementation surface:

- `gateway.platforms.base.BasePlatformAdapter`;
- `gateway.platforms.base.MessageEvent`;
- `gateway.platforms.base.SendResult`;
- `gateway.session.SessionSource`;
- `gateway.session.build_session_key`;
- `gateway.config.Platform`;
- `gateway.platform_registry.PlatformEntry`;
- `hermes_cli.plugins.PluginContext.register_platform`.

Pinned signatures from the installed runtime:

```text
BasePlatformAdapter.send(self, chat_id, content, reply_to=None, metadata=None)
BasePlatformAdapter.handle_message(self, event)
MessageEvent(..., source, message_id, reply_to_message_id, metadata, ...)
SessionSource(platform, chat_id, chat_type, user_id, thread_id, message_id, ...)
SendResult(success, message_id=None, error=None, raw_response=None, ...)
build_session_key(source, group_sessions_per_user=True, thread_sessions_per_user=False, profile=None)
```

A no-network fake `buzz` platform round-trip passed after registering a
temporary in-process `PlatformEntry`. The resulting session key shape was:

```text
agent:main:buzz:channel:<channel-id>:<thread-root-id>
```

Implementation decisions from the spike:

- the Buzz platform plugin must register `buzz` before constructing
  `Platform("buzz")`;
- the adapter constructor must call
  `super().__init__(config, Platform("buzz"))`;
- the adapter must implement at least `connect`, `disconnect`, `send`, and
  `get_chat_info`;
- `handle_message(event)` is the correct gateway ingress call;
- outbound Buzz delivery should return `SendResult(success=True,
  message_id=<buzz-reply-event-id>, raw_response=<buzz-cli-response>)`;
- keep Buzz event/thread facts in `MessageEvent.metadata["buzz"]`.

Remaining WP6 limits:

- no plugin was installed or enabled in the live Hermes plugin directory;
- the Hermes source checkout is dirty, so implementation should pin against
  the installed CLI version plus the inspected signatures and re-run the
  compatibility smoke before live enablement;
- restart-survival of accepted ingress remains a WP7 validation item once the
  real SQLite queue path exists.

## WP7: Hermes Platform Plugin MVP

Purpose: implement the same Buzz contract natively in Hermes.

### Proposed files

```text
plugin.yaml
adapter.py
buzz_transport.py
ingress_store.py
mapping.py
policy.py
errors.py
tests/fixtures/external_agent_v1/
tests/test_config.py
tests/test_mapping.py
tests/test_policy.py
tests/test_ingress_store.py
tests/test_adapter.py
tests/test_restart.py
```

### Implementation order

1. Create `plugin.yaml` and `register(ctx)`.
2. Implement configuration validation and env-driven enablement.
3. Implement SQLite migrations and ingress repository.
4. Start a supervised `buzz listen` child in `connect()`.
5. Durably insert before advancing stdout consumption.
6. Normalize the shared facts and apply fail-closed policy.
7. Build `SessionSource` with Buzz channel and thread root.
8. Build `MessageEvent` with Buzz event ID and reply context.
9. Call `self.handle_message(event)`.
10. Implement `send()` with `buzz messages send --reply-to`.
11. Return/store the Buzz event ID in `SendResult`.
12. Cancel listener tasks and drain accepted work in `disconnect()`.

### SQLite minimum schema

```text
adapter_meta
  schema_version
  last_created_at

ingress_events
  account_id
  event_id
  created_at
  channel_id
  thread_root_id
  state
  ignore_reason
  attempts
  next_attempt_at
  admitted_at
  completed_at
  reply_event_id
```

Primary key: `(account_id, event_id)`.

Use transactions for admission, state transitions, and receipt recording.
Prune completed tombstones by both retention duration and maximum row count.

### Tests

Run the shared fixture contract plus:

- SQLite migration and corruption behavior;
- concurrent duplicate insert;
- restart with pending rows;
- session key mapping;
- active-session serialization;
- two-channel concurrency;
- clean disconnect;
- child-process exit;
- secret redaction;
- uncertain outbound delivery.

### Exit criteria

- all tests pass in the lunar02 Hermes environment;
- one Hermes canary identity passes the same live cases as OpenClaw;
- stopping the old connector does not stop Hermes Buzz delivery;
- Hermes' existing platforms and tools remain unaffected.

## WP8: Optional Managed-Agent Directory Publication

Purpose: separate Desktop directory visibility from transport readiness.

### Buzz change surface

```text
crates/buzz-cli/src/managed_agent_publish.rs
crates/buzz-cli/src/commands/agents.rs
crates/buzz-cli/src/lib.rs
docs/cli-external-agents.md
crates/buzz-test-client/tests/e2e_managed_agent.rs
```

### Scope

- `buzz agents publish`;
- owner-signed kind `30177`;
- `d` tag equals the resident agent pubkey;
- strict public-field validation;
- rejection of known secret/local runtime fields;
- documented distinction between directory record and process ownership.

### Exit criteria

- invalid or secret-bearing content is rejected before signing;
- Desktop renders the external agent record;
- deleting/stopping the plugin does not misrepresent Buzz process ownership;
- transport/plugin MVP does not depend on this PR merging.

### Upstream unit

PR C: headless managed-agent directory publication.

## WP9: Identity and Desktop Visibility Audit

Purpose: make resident agents visible without conflating four identity records.

For each canary:

1. verify the private key resolves to the expected pubkey;
2. verify kind `0` display profile;
3. verify NIP-OA owner evidence when used;
4. verify explicit relay membership;
5. verify channel bot membership;
6. verify kind `10100` channel-add policy;
7. optionally publish kind `30177`;
8. test Desktop `@` mention selection;
9. test direct pasted `nostr:npub` mention;
10. record which record Desktop uses for each visible surface.

If Desktop mention selection excludes valid resident agents, prepare a focused
UI PR based on membership plus kind `0`/directory evidence. Do not weaken
plugin authorization or add a fake local managed process.

### Exit criteria

- identity ownership and directory authorship are documented;
- canaries are mentionable through the supported UI;
- any remaining Desktop gap has a minimal reproducible test and isolated PR
  scope.

## WP10: Shadow Deployment

Purpose: compare native plugin decisions against production traffic without
allowing duplicate replies.

### OpenClaw on lunar01

1. Install the plugin to a versioned staging location.
2. Configure the existing canary identity, not Selene's production identity.
3. Start in `shadow=true`.
4. Record only event IDs and decision reason codes; do not record private keys
   or full sensitive message bodies in diagnostic summaries.
5. Compare eligible/ignored decisions with the current connector for at least
   24 hours.
6. Exercise relay disconnect and plugin restart.

### Hermes on lunar02

Repeat using the Hermes canary identity and local SQLite store.

### Required observations

- event count by channel;
- eligible, ignored, malformed, and duplicate counts;
- queue depth and oldest pending age;
- reconnect count and recovery duration;
- cursor overlap volume;
- decision mismatches with the connector;
- secret-redaction audit.

### Approval checkpoint C — original matrix blocked; corrected narrow rerun passed

Installing or enabling plugins on lunar01/lunar02 changes live host state.
Present:

- exact files and configuration to add;
- version pins;
- service/reload commands;
- secret locations;
- rollback commands;
- completed local test evidence.

The user approved the isolated lunar01 canary. It used a new identity and
channel, so the legacy connector never shared reply authority. The install,
config validation, Gateway restart, policy cases, restart recovery, and
uncertain-send case ran. The canary was then disabled after the no-final-text
retry loop described in WP5.

The user later approved the corrected narrow rerun. On 2026-07-29 the former
no-final path completed as one durable reaction with no text, pending row, model
regeneration, or restart duplicate; the canary was disabled afterward.

Checkpoint C remains incomplete until the full corrected matrix passes. Do not
treat either narrow authorization as permission to keep the plugin enabled,
begin Hermes deployment, or cut over a production identity.

### Exit criteria

- no shadow-mode reply is emitted;
- both plugins survive restart and relay interruption;
- decision mismatches are explained or fixed;
- queues remain bounded;
- rollback has been dry-run with canary state.

## WP11: Per-Agent Cutover

Purpose: transfer reply authority without a dual-writer window.

Cut over one canary, then one production identity at a time.

### Sequence

1. Confirm native plugin healthy in shadow mode.
2. Stop new deliveries for the identity in the connector.
3. Drain or explicitly park connector work for that identity.
4. Record the last connector event ID/timestamp.
5. Enable native plugin reply authority from an overlap cursor.
6. Send an owner mention in the canary channel.
7. Verify exactly one reply, author, `h` tag, immediate parent, and root.
8. Test non-owner rejection.
9. Restart the plugin and send a second owner mention.
10. Restart the harness and send a third owner mention.
11. Reboot the host during a scheduled window and repeat.
12. Observe for the agreed soak period before the next identity.

### Immediate rollback triggers

- more than one visible reply for one ingress event;
- reply signed by the wrong identity;
- wrong channel or thread;
- unauthorized sender triggers a turn;
- durable queue cannot recover;
- plugin affects non-Buzz harness channels;
- secrets appear in logs;
- sustained backlog above the agreed threshold.

### Rollback

1. Disable plugin reply authority.
2. Stop the plugin account and preserve its state directory.
3. Restore connector delivery for the identity.
4. Start connector catch-up from the last known shared checkpoint with overlap.
5. Confirm one owner mention receives one reply.
6. Preserve failure artifacts with secrets redacted.

### Approval checkpoint D

Each production identity cutover requires explicit approval after canary soak
evidence is presented.

## WP12: Connector Quarantine and Closeout

Purpose: retire duplicated transport only after native plugins are proven.

### Tasks

1. Stop the legacy OpenClaw webhook adapter and connector routes for migrated
   identities.
2. Move service definitions and code to a dated quarantine location rather
   than deleting immediately.
3. Remove migrated private keys from the connector only after the rollback
   window closes.
4. Confirm the connector cannot sign as migrated identities.
5. Retain non-secret migration records and final event checkpoints.
6. Update Lunar Park KB operational authority:
   - OpenClaw plugin owns Selene's Buzz transport on lunar01;
   - Hermes plugin owns the Hermes resident identity on lunar02;
   - Buzz relay remains on lunar01;
   - `buzz-acp` remains the temporary Buzz-managed lane.
7. Archive or mark superseded connector documents so they do not remain
   competing active architecture.

### Exit criteria

- native plugins operate independently;
- old connector paths cannot produce duplicate replies;
- rollback window has closed;
- key custody matches the target architecture;
- canonical docs and deployed state agree.

## Test Strategy

### Test layers

| Layer | Purpose | Runs where |
|---|---|---|
| unit | parsers, policy, mapping, state transitions | local/plugin CI |
| fixture contract | identical behavior across Rust/TS/Python | all three repos |
| process integration | CLI pipes, exit codes, signals, replay | Buzz and plugin CI |
| harness integration | native session and reply pipelines | isolated harness roots |
| relay E2E | auth, membership, filters, threading | dedicated local/test relay |
| host canary | installed runtime compatibility | lunar01/lunar02 |
| migration smoke | dual-writer prevention and rollback | production canary channel |

### Buzz quality gates

Run the smallest relevant commands first, then:

```bash
. ./bin/activate-hermit
cargo fmt --all -- --check
cargo clippy -p buzz-cli --all-targets -- -D warnings
cargo test -p buzz-cli
just test-unit
just ci
```

Run relay integration tests when listen/auth/query behavior changes:

```bash
. ./bin/activate-hermit
just test
```

Record infrastructure prerequisites and any tests that could not run.

### OpenClaw gates

Use the plugin repository's pinned package manager and scripts:

```text
format
lint
typecheck
unit
fixture-contract
restart-contract
isolated-plugin-load
lunar01-canary
```

### Hermes gates

Use the pinned lunar02 virtual environment:

```text
format
lint
pytest unit
pytest fixture-contract
pytest restart
isolated-plugin-load
lunar02-canary
```

Do not silently run tests against a different global Python/Hermes install.

## Observability Contract

Both plugins expose:

```text
plugin version
Buzz CLI version and resolved path
relay authority
account pubkey
configured channel count
connection state
last EOSE time
last admitted event ID/time
last completed event ID/time
replay cursor
pending/failed/completed queue counts
oldest pending age
restart count
last classifiable error
shadow/reply-authority mode
```

Never expose:

```text
private key
auth tag contents
full environment
unredacted subprocess arguments containing secrets
raw sensitive message history in health output
```

Use stable error codes:

```text
CONFIG_INVALID
CLI_NOT_FOUND
CLI_VERSION_UNSUPPORTED
AUTH_FAILED
RELAY_UNAVAILABLE
SCHEMA_UNSUPPORTED
EVENT_MALFORMED
STATE_IO_FAILED
HARNESS_UNAVAILABLE
OUTBOUND_REJECTED
OUTBOUND_UNCERTAIN
```

## PR and Commit Strategy

### Buzz

Keep these independently reviewable:

1. PR A: identity and compact event fields;
2. PR B: listen v1, cursor, lifecycle, and tests;
3. PR C: optional managed-agent publication;
4. later PR: Desktop mention eligibility, only if reproduced;
5. later proposal: generic outbound idempotency, only after the MVP exposes a
   concrete crash-recovery need.

Each PR:

- starts from current upstream;
- contains no Lunar Park paths, hostnames, identities, or service files;
- updates `docs/cli-external-agents.md`;
- names exact tests run;
- avoids dependency additions unless justified.

### Plugins

Use narrow commits:

```text
scaffold/config
contract fixtures
transport supervision
durable ingress
policy and mapping
outbound receipts
restart recovery
observability
host deployment docs
```

Do not combine harness upgrades with plugin feature commits.

## Execution Checkpoints

| Checkpoint | Evidence required | Authorization |
|---|---|---|
| A: start branches/repos | clean baseline, repo choices, PR coordination | user |
| B: harness upgrade if needed | compatibility spike and rollback plan | user |
| C: install canary/shadow plugins | original matrix blocked; corrected 2026-07-29 narrow no-final rerun passed with one durable reaction and no restart duplicate; full corrected matrix remains open; canary disabled | user |
| D: production cutover | shadow soak, mismatch report, canary pass | user per identity |
| E: retire connector | soak complete, rollback window decision | user |

## First Implementation Slice

The first coding slice should stop after proving:

```text
Buzz CLI v1 owner-mention fixture
  -> adapter-owned SQLite durable admission
  -> owner/mention policy pass
  -> one OpenClaw conversation lane
  -> final text response OR explicit silent outcome
  -> buzz messages send --reply-to OR acknowledgement reaction
  -> persisted outbound outcome and Buzz event ID when available
```

It must use:

- one canary identity;
- one canary channel;
- text kind `9`;
- no directory publication;
- no media, DMs, general reaction handling, presence, or proactive delivery;
- a reaction is permitted only as the bounded receipt for a completed silent
  turn;
- no production agent key;
- no connector cutover.

This slice is accepted only after duplicate replay and plugin restart tests
prove one visible text reply or silent acknowledgement per accepted event.

## Immediate Next Action

The corrected narrow Checkpoint C case passed, but the native Buzz channel is
disabled and the full corrected matrix remains open. Do not treat Desktop
connection metadata as transport readiness, and do not start production
connector cutover.

Prioritized next work:

1. **Commit the accepted product baseline.** The four product questions in
   `SELF_HOSTED_AGENT_INTEGRATION_SPEC.md` are resolved. Validate the aligned
   plans and handoff, then commit the documentation snapshot before more runtime
   work.
2. **Complete the single-Selene communication gate.** Resolve the exact Desktop
   test channel UUID, verify the isolated canary identity/profile, configure
   only that identity/channel in WP5, enable it under the existing rollback,
   and prove one signed threaded reply plus no duplicate after restart.
3. **Rerun the full corrected Checkpoint C matrix.** Keep legacy and native
   authority disjoint, preserve the narrow-pass evidence, and disable the
   canary after the matrix until cutover is separately approved.
4. **Refresh and review the Buzz integration stack.** The current
   `b19fd1508` branch is clean but eight commits behind refreshed
   `upstream/main` `3e48f1b23`. Rebase only after preserving the current test
   point, then rerun the integrated Desktop/Tauri gates. Coordinate with open
   upstream PRs #2633, #2933, and #2942 before publication.
5. **Implement WP5A only after the single-agent gate.** RC5 preselects the
   primary OpenClaw agent and exposes the full durable roster without enrolling
   the stack; every selected agent gets its own identity/account. RC6 maps the
   stable Buzz DM channel UUID into OpenClaw's native `direct` route and honors
   `session.dmScope`. The two expansions may proceed independently after the
   gate.
6. **Start Hermes only after OpenClaw passes the shared lifecycle contract.**
   Create the standalone repository, vendor the exact Buzz fixture commit, and
   port the durable admission, policy, conversation, reconciliation, and
   terminal-outcome behavior against Hermes `0.18.2`.
7. **Defer migration and retirement.** Run both native plugins in shadow/canary
   mode, cut over one identity at a time after soak evidence, and quarantine the
   legacy connector only after rollback windows close.

The planned Hermes repository remains
`/Users/dspury/Lunar-Park/integrations/hermes-platform-buzz/`. It has not been
created.

# Resident Agent Adapter Implementation Map

Status: accepted architecture reference. It is not current execution
authority; use the linked implementation plan and standalone plugin checkpoint
for live status.

Date: 2026-07-26

Scope: OpenClaw and Hermes resident agents connected to Buzz

Execution follow-up:
[Resident Agent Adapter Implementation Plan](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md)

## Decision

Buzz needs two distinct agent integration lanes:

1. **Buzz-managed temporary agents** continue through `buzz-acp`. The generic
   harness catalog merged in PR #2773 already includes `openclaw acp` and
   `hermes-acp`. PR #2633 is a generic improvement to this lane because it
   persists Buzz-channel-to-ACP-session bindings.
2. **Harness-owned resident agents** connect through native messaging
   extensions: an OpenClaw Buzz channel plugin and a Hermes Buzz platform
   adapter plugin. Their lifecycle, memory, tools, and sessions remain owned by
   their native harness.

Do not add OpenClaw- or Hermes-specific runtime branches to Buzz. Do not evolve
the current central Python connector into a permanent second Buzz API.

```text
                         Buzz relay
                             │
              Nostr auth, events, membership, history
                 ┌───────────┴───────────┐
                 │                       │
       Buzz-managed ACP lane     Resident-agent lane
                 │                       │
             buzz-acp             upstream buzz CLI
                 │               listen / send / query
           ACP subprocess          ┌──────┴──────┐
          ┌──────┴──────┐          │             │
      openclaw acp  hermes-acp  OpenClaw      Hermes
                                channel       platform
                                plugin        plugin
```

This architecture preserves a clean Buzz fork:

- generic protocol and CLI capabilities can be proposed upstream to
  `block/buzz`;
- OpenClaw integration follows OpenClaw's plugin API;
- Hermes integration follows Hermes' plugin API;
- Lunar Park identities, hostnames, service definitions, and deployment
  configuration stay outside upstream Buzz.

## Current Source Map

Review snapshot:

- local alignment branch: `feature/agent-harness-adapter-alignment`;
- local Buzz baseline: `95fdf978` (merged PR #2773);
- refreshed `upstream/main`: `63c62fcf3` at the 2026-07-27 documentation pass;
- WP1 identity branch: `buzz-pr-identity` at `341b37114`;
- WP2 listener branch: `buzz-pr-listen` at `7ce55d055`;
- WP3 fixture branch: `buzz-pr-fixtures` at `c723de94b`;
- lunar01 OpenClaw: `2026.7.1` (`5f39975`);
- lunar02 Hermes: `0.18.2` (`0fa5e41c`).
- standalone OpenClaw plugin: implementation `be24e64`, Checkpoint C record
  `c64e976`.

Do not rebase the current working tree as part of architecture mapping. Rebase
or recreate each implementation branch only when its PR slice begins.

### Already merged in Buzz

| Surface | Location | Role |
|---|---|---|
| Generic ACP bridge | `crates/buzz-acp/` | Buzz-owned process and turn lifecycle |
| Harness presets | Desktop managed-agent modules | Data-driven `openclaw acp` and `hermes-acp` launch definitions |
| Agent-facing commands | `crates/buzz-cli/` | Authenticated Buzz operations for any external process |
| Event builders | `crates/buzz-sdk/` | Correct `h`, `e`, `p`, and NIP-10 tag construction |
| Authenticated WebSocket | `crates/buzz-ws-client/` | NIP-42/NIP-OA relay transport |
| Identity/event kinds | `crates/buzz-core/src/kind.rs` | Canonical kind registry |

### Open upstream work

#### PR #2942: resident-agent transport prerequisites

The proposed CLI surface provides:

- `buzz users me`;
- `buzz listen`;
- `buzz agents publish`;
- `pubkey` and `tags` in compact message results.

`buzz listen` emits one NDJSON record per event:

```json
{
  "id": "<64-hex event id>",
  "pubkey": "<64-hex author pubkey>",
  "kind": 9,
  "content": "message",
  "created_at": 1785100000,
  "tags": [["h", "<channel uuid>"], ["p", "<agent pubkey>"]]
}
```

Its default kinds are `9,40002,40008,45001,45003`. It authenticates through
the same NIP-42/NIP-OA path as the other CLI commands and reconnects with
exponential backoff.

This is the correct upstream direction, but the adapter MVP should not depend
on its current webhook option. Webhook delivery is best-effort, has no durable
acknowledgement, and recreates the central bridge shape that this design is
removing. Consume stdout directly and durably admit each event inside the
harness plugin.

Before treating PR #2942 as a stable adapter contract, resolve these review
items:

1. Add a replay boundary such as `--since <unix-seconds>` and document an
   overlap-plus-dedup recovery rule. The current reconnect subscription has no
   cursor and can replay the entire retained result set.
2. Emit an optional lifecycle envelope for `connected`, `EOSE`, `reconnecting`,
   and `fatal`, or explicitly define stderr JSON as that contract. Plugins need
   to distinguish history replay from live delivery and transport health.
3. Clarify `--channel` plus `--mentions-of-me`. The current implementation
   creates two OR filters, so it receives all traffic in the selected channels,
   not only mentions within those channels.
4. Make the NDJSON schema and compatibility policy explicit. Add a schema
   version before changing or removing fields.
5. Add subprocess-facing tests for stdout purity, slow consumers, SIGTERM, and
   reconnect replay. Diagnostics must remain on stderr.

These are generic external-client capabilities and are appropriate for a
focused Buzz PR.

#### PR #2633: ACP-only continuity

PR #2633 stores:

```text
(normalized agent command, arguments, Buzz channel UUID) -> ACP session ID
```

It loads the mapping under a shared file lock, writes atomically, attempts
`session/load` only when the ACP agent advertises the capability, and
conditionally removes a stale binding before falling back to `session/new`.

That is useful for the Buzz-managed ACP lane only. Resident OpenClaw and Hermes
plugins must use their harness-native conversation/session storage instead of
the `buzz-acp` sidecar.

## Package and Repository Ownership

Use three independently reviewable deliverables.

### 1. Buzz generic external-agent surface

Repository: this Buzz fork, then upstream `block/buzz`.

Owns:

- authenticated listen/query/send primitives;
- stable NDJSON event schema;
- correct NIP-29 channel and NIP-10 thread behavior;
- self-identity lookup;
- optional managed-agent directory publication;
- CLI contract and conformance fixtures.

Must not own:

- OpenClaw session keys or Gateway lifecycle;
- Hermes gateway sessions or memory;
- resident-agent response policy execution;
- Lunar Park fleet routing;
- resident agent private keys as a central service.

### 2. `openclaw-channel-buzz`

Repository/package: a standalone OpenClaw plugin package. Do not place its
runtime in `block/buzz`. A temporary development copy may live in Lunar Park
infrastructure until its package home is chosen.

Current first-slice modules:

```text
openclaw-channel-buzz/
  package.json
  openclaw.plugin.json
  src/
    config.ts                 # account config and SecretRef resolution
    conversations.ts          # OpenClaw dispatch and lane identity
    errors.ts                 # stable adapter error categories
    index.ts                  # import-safe plugin and channel registration
    ingress.ts                # normalization and policy facts
    runtime.ts                # admission, recovery, and account lifecycle
    state.ts                  # adapter-owned SQLite keyed state
    transport.ts              # supervised buzz listen/send processes
    types.ts                  # adapter contracts
  test/
    fixtures/
    config.test.ts
    conversations.test.ts
    import-safety.test.ts
    ingress.contract.test.ts
    runtime.test.ts
    state.test.ts
    transport.test.ts
```

Use OpenClaw's channel SDK rather than legacy dispatch glue:

- the durable ingress monitor/queue surface for admission, serialized
  per-conversation processing, completion tombstones, and restart recovery;
- the channel message adapter for outbound sends and receipts;
- the shared outbound-echo registry for self-message suppression;
- `resolveSessionConversation` for Buzz channel/thread-to-session mapping;
- channel mention-gating helpers for the final activation decision;
- account-scoped runtime lifecycle only after its isolation contract is proven.

The OpenClaw version seam is resolved for this slice. Lunar01 remains pinned to
OpenClaw `2026.7.1`; no harness upgrade is required. Ordinary third-party
plugins cannot call the host-managed keyed-store or ingress-queue methods even
though those methods appear in the runtime type. The plugin therefore uses the
public `createDurableInboundReceiveJournal` facade with an adapter-owned SQLite
store beneath `runtime.state.resolveStateDir()`. It does not import hashed
internals or fall back to process-memory-only deduplication.

The plugin's conversation grammar should be:

```text
account = hash(canonical relay authority + agent pubkey)
chat    = Buzz channel UUID
thread  = NIP-10 root event ID, absent for the channel's base conversation
```

OpenClaw owns the final session key. The plugin should return the structured
base conversation and thread facts rather than constructing an undocumented
OpenClaw key string itself.

### 3. `hermes-platform-buzz`

Repository/package: a standalone Hermes plugin installed under
`~/.hermes/plugins/buzz/` during development.

Suggested modules:

```text
hermes-platform-buzz/
  plugin.yaml
  adapter.py                 # register() and BasePlatformAdapter
  buzz_transport.py          # supervised buzz CLI processes
  ingress_store.py           # SQLite queue, tombstones, cursor
  mapping.py                 # Buzz event -> MessageEvent
  policy.py                  # owner/allowlist/mention activation
  tests/
    fixtures/
    test_adapter.py
    test_ingress_store.py
    test_restart.py
```

The plugin should:

- extend `BasePlatformAdapter`;
- start `buzz listen` in `connect()`;
- stop and await the listener in `disconnect()`;
- turn accepted Buzz events into `MessageEvent` values and call
  `self.handle_message(event)`;
- implement `send(chat_id, content, reply_to, metadata)` with
  `buzz messages send`;
- return the Buzz event ID as `SendResult.message_id`;
- register through `ctx.register_platform()` with no Hermes core edits.

Hermes does not currently expose the same durable ingress helper described by
OpenClaw. Implement the minimum equivalent as a plugin-local SQLite queue:

```text
primary key: (account_id, Buzz event ID)
states: pending -> adopted -> completed
conversation lane: (relay authority, channel UUID, thread root)
completed retention: 30 days, capped for expected volume
```

Do not create a shared network service for this. The TypeScript and Python
plugins should share fixtures and behavioral requirements, not a runtime
daemon.

## Common Adapter Contract

### Configuration

Each plugin account needs:

| Setting | Required | Notes |
|---|---:|---|
| relay URL | yes | Canonical authority; `lunar01:3000` host scoping must be preserved |
| private key | yes | Per-agent, resolved from the harness secret system |
| auth tag | deployment-dependent | NIP-OA JSON, kept with the agent account |
| allowed channel UUIDs | yes for MVP | Fail closed |
| owner pubkey | yes for `owner-only` | Explicit until safely derivable |
| response policy | yes | `owner-only` default; `allowlist` or `anyone` opt-in |
| require mention | yes | `true` by default |
| Buzz CLI path | optional | Resolve once and log exact version/path |

OpenClaw credentials must be available in the Gateway/plugin execution locus.
The live Gate 3 test proved that variables injected into a separate
`openclaw acp` bridge do not automatically reach Gateway-executed tools.
Hermes credentials stay local to the Hermes host and plugin process.

Never store private keys in kind `30177`, plugin manifests, logs, fixtures, or
the shared conformance package.

### Identity records

Keep the four records distinct:

| Record | Author | Purpose | Resident-agent requirement |
|---|---|---|---|
| kind `0` | agent | Display identity and owner evidence | required |
| kind `10100` | agent | Channel-addition policy | configure when needed |
| kind `30175` | owner | Reusable persona definition | optional |
| kind `30177` | owner, `d` = agent pubkey | Managed-agent directory/control record | optional for directory visibility |

A resident agent can participate with a key, kind-0 profile, and channel
membership. Publishing `30177` must not imply that Buzz owns its process.
`respond_to` in `30177` is metadata; the resident plugin must enforce the
actual activation policy.

### Ingress admission order

Both plugins must apply the same ordered gates:

1. Validate the schema and extract exactly one valid `h` channel tag.
2. Reject unsupported kinds.
3. Reject channels outside the configured set.
4. Reject self-authored events.
5. Durably insert by Buzz event ID.
6. Extract sender, explicit `p` mention, immediate reply ID, and NIP-10 root.
7. Evaluate `owner-only`, `allowlist`, or `anyone`.
8. Evaluate mention/reply/thread-participation activation.
9. Serialize delivery within the conversation lane.
10. Hand the event to the harness and mark it adopted/completed according to
    that harness's ingress contract.

Malformed or unauthorized events should be completed as ignored with a reason,
not retried forever. Transport, disk, and harness-unavailable failures remain
pending with bounded backoff.

### Delivery and restart semantics

Target behavior is **at least once at ingress, idempotent at visible reply**:

- Buzz event ID is the immutable ingress identity.
- A completed tombstone prevents normal redelivery.
- A pending event survives plugin restart.
- The relay is the replay source; the plugin is not a second message archive.
- Reconnect uses a timestamp overlap and event-ID dedup until Buzz supports a
  stronger cursor.
- An outbound receipt is the returned Buzz event ID.
- Store `(ingress event ID, reply event ID)` before treating the turn as
  visibly complete.
- On uncertain publish failure, query the stored receipt or an idempotency
  marker before generating another reply.

The last point is a remaining generic gap: `buzz messages send` does not accept
an idempotency key. The first plugin versions can suppress duplicate model runs,
but there is still a crash window after relay acceptance and before the plugin
persists the returned event ID. A later generic Buzz proposal should add a
client-supplied idempotency tag and lookup contract rather than solving this
with harness-specific relay behavior.

### Threading

For inbound events:

- `h` identifies the Buzz chat/channel;
- the `e` tag marked `reply` identifies the immediate parent;
- the `e` tag marked `root` identifies a nested thread root;
- for a direct reply with only a `reply` tag, the parent is also the root;
- for a top-level message, the triggering event becomes the thread root when
  the agent replies.

For outbound events, always call:

```text
buzz messages send --channel <uuid> --content - --reply-to <trigger-event-id>
```

The CLI already queries the parent and constructs the correct NIP-10 root and
reply tags. The plugins should not duplicate that logic.

Direct messages, media, edits, reactions, presence, and typing are not part of
the first production cut. Add them one capability at a time after text channel
delivery and restart recovery pass.

## Implementation Phases

### Phase 0: contract stabilization — fork branches complete

WP1-WP3 are implemented on focused, pushed fork branches. Upstream publication
remains paused at the PR-coordination gate; this is not the same as having
landed the contract in `block/buzz`.

Buzz PR work:

1. Rebase or split PR #2942 onto current `upstream/main`.
2. Land `users me` and compact `pubkey`/`tags` as one small prerequisite if
   needed.
3. Land `listen` with the replay/lifecycle/schema clarifications above.
4. Land `agents publish` separately; it is directory registration, not
   transport.
5. Add language-neutral JSON fixtures under a durable CLI contract test
   location.

Exit criteria:

- stdout contains only versioned event NDJSON;
- reconnect behavior and cursor limitations are documented;
- owner mention, non-owner rejection, thread reply, slow consumer, restart,
  and duplicate delivery are covered by automated tests.

### Phase 1: OpenClaw plugin MVP — implemented, canary blocked

Implement first on lunar01 because the relay, OpenClaw Gateway, and existing
canary evidence are colocated.

MVP:

- a pinned OpenClaw SDK version and an explicit decision to upgrade or compose
  the durable monitor from the installed journal/queue primitives;
- one configured Buzz account;
- text kinds `9` and `40002`;
- configured channel allowlist;
- owner-only plus explicit mention activation;
- durable inbound journal backed by adapter-owned SQLite;
- threaded text reply;
- start/stop/restart recovery;
- status output with CLI version, relay, account pubkey, queue depth, and last
  transport error.

The first slice is implemented in the standalone plugin repository. An isolated
active canary was used because it had a new identity/channel with no connector
reply authority. Owner/policy/thread/replay/restart/uncertain-send cases passed.
The canary failed overall when OpenClaw completed a turn without final text:
the activation remained pending and the account supervisor repeatedly
restarted it. The Buzz channel is disabled until that outcome is bounded and
durably terminal.

### Phase 2: Hermes plugin MVP

Implement on lunar02 using the same fixtures and acceptance cases:

- plugin-local SQLite ingress queue;
- `BasePlatformAdapter` lifecycle;
- `MessageEvent.message_id = Buzz event ID`;
- Buzz channel as Hermes `chat_id`;
- threaded reply metadata where Hermes exposes it;
- owner-only plus mention activation;
- clean disconnect and restart recovery.

Do not route Hermes through OpenClaw or the lunar01 connector. The Hermes
plugin must continue working when the old connector is stopped.

### Phase 3: identity and directory

For each resident agent:

1. verify `buzz users me`;
2. verify kind-0 display profile and NIP-OA owner evidence;
3. verify channel membership;
4. publish kind `10100` policy when required;
5. optionally publish kind `30177` from the owner identity;
6. confirm Desktop mention eligibility separately from transport.

Any Desktop mention-selector gap should be a focused upstream UI PR. Do not
work around it by weakening adapter activation policy.

### Phase 4: production migration

Use a per-agent cutover:

1. Freeze feature development on the Python connector.
2. Export non-secret identity, channel, and session-routing mappings.
3. Run the new plugin in shadow mode beside the connector.
4. Compare ingress decisions for at least 24 hours.
5. Stop outbound delivery for that agent in the connector.
6. Enable plugin replies with a canary channel first.
7. Verify restart, relay disconnect, host reboot, duplicate suppression, and
   thread continuity.
8. Move the old adapter service and configuration to a dated quarantine
   location for rollback.
9. Remove its key only after the rollback window closes.

Do not run both implementations with reply authority for the same agent
identity.

### Phase 5: first-class expansion and optional capabilities

Add only after both text MVPs pass:

- durable-agent roster discovery and identity-to-agent routing;
- DMs and encryption through each harness's native direct-session router;
- media receive/send;
- reactions;
- typing and presence;
- broadcast/always-listen channels;
- cron/proactive delivery;
- remote signing when the Buzz NIP-46 path is ready.

## Upstream PR Map

| PR | Repository | Contents | Explicit exclusions |
|---|---|---|---|
| A | `block/buzz` | `users me`, compact author/tags | plugin code |
| B | `block/buzz` | stable `listen` NDJSON, cursor/lifecycle, tests | webhooks as resident architecture |
| C | `block/buzz` | `agents publish` validation and docs | private keys, runtime ownership |
| D | OpenClaw plugin repo | Buzz channel plugin MVP | Buzz relay/runtime branches |
| E | Hermes plugin repo | Buzz platform plugin MVP | Hermes core edits |
| F | `block/buzz` | mention-selector eligibility if still missing | Lunar Park-specific policy |
| G | `block/buzz` | optional generic outbound idempotency contract | harness-specific semantics |

PR #2633 remains independent and should be reviewed only as a generic ACP
continuity improvement.

## Acceptance Matrix

Each plugin must pass the same black-box cases:

| Case | Expected result |
|---|---|
| owner explicitly mentions agent | exactly one threaded reply |
| non-owner mentions under `owner-only` | no harness dispatch and no reply |
| agent receives its own reply | ignored |
| same event delivered twice | one harness adoption and one visible reply |
| two events in one thread | serialized in order |
| two channels receive events | may run concurrently within configured limit |
| plugin exits after durable admission | pending event recovers after restart |
| relay drops | reconnects with overlap and no duplicate reply |
| CLI exits or emits malformed stdout | unhealthy state, supervised restart, no data loss |
| outbound publish is rejected | pending/failed state is visible; no false completion |
| host reboots | identity, queue, and session routing recover |
| connector is stopped | native plugin continues operating |

Reuse the existing Gate 3 owner/non-owner/cancel/restart canary setup where
applicable, but keep the ACP acceptance suite separate. A resident plugin
passing does not prove ACP compatibility, and an ACP preset passing does not
prove resident lifecycle integration.

## Known Blockers and Decisions

### Must resolve before the next canary

1. Resolve the exact Buzz Desktop test channel UUID and isolated canary
   identity.
2. Verify that OpenClaw's configured account key, `agentPubkey`, and
   subscribed `channelIds` match that test pair without overlapping legacy
   reply authority.
3. Stabilize or locally carry the generic Buzz CLI contract while upstream
   PR #2942 coordination remains open.
4. Preserve the corrected narrow no-final evidence and use the documented
   enable/test/disable rollback sequence.

The bounded silent-turn policy and its restart regression test are complete;
the corrected narrow live rerun passed on 2026-07-29.

### Can remain deferred

- remote signing;
- Desktop lifecycle controls for resident agents;
- automatic directory registration;
- media;
- shared binary SDK;
- replacement of the CLI subprocess with an in-process Nostr client.
- creation of the Hermes plugin repository until the OpenClaw lifecycle gate
  passes.

DM work remains deferred only until the single-agent channel/thread gate
passes. It is then required for first-class parity: use the stable Buzz DM
channel UUID as the native direct peer address rather than creating an
adapter-owned parallel session model.

## Recommended Next Step

Do not re-enable the OpenClaw canary without the exact isolated identity,
channel, and rollback boundary. The next implementation slice is narrow:

```text
Buzz Desktop owner mention
  -> exact isolated OpenClaw account and subscribed channel
  -> one signed threaded Selene reply
  -> Gateway restart
  -> no duplicate model turn or visible delivery
```

The corrected narrow no-final case has already passed. After the Desktop
round-trip and restart gate passes, rerun the full corrected Checkpoint C
matrix. Selective roster enrollment, DM parity, Hermes implementation, and
connector cutover remain later checkpoints.

## References

- Buzz external-agent issue: https://github.com/block/buzz/issues/2663
- Buzz generic ACP presets: https://github.com/block/buzz/pull/2773
- Buzz ACP persistence: https://github.com/block/buzz/pull/2633
- Buzz external-agent CLI: https://github.com/block/buzz/pull/2942
- OpenClaw channel plugins:
  https://docs.openclaw.ai/plugins/sdk-channel-plugins
- OpenClaw ACP agents: https://docs.openclaw.ai/tools/acp-agents
- Hermes platform adapters:
  https://hermes-agent.nousresearch.com/docs/developer-guide/adding-platform-adapters

# Remote Agent Connect Implementation Plan

Status: active. RC1-RC4 are implemented on focused branches; the integrated
Desktop flow is manually validated through discovery, connection, normal agent
presentation, channel attachment, and team membership. A live Selene reply,
automatic identity onboarding, Buzz-managed archive/deletion, and RC5 selective
resident-agent enrollment remain open. RC6 direct-message parity is specified
but not started. Opened 2026-07-27; updated 2026-07-29.

This plan covers connecting **already-running, self-hosted agents** on remote
machines to a Buzz workspace. It is a peer of
[`RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md`](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md),
not a replacement: that plan owns the harness-side adapters (OpenClaw, Hermes);
this plan owns the Buzz-side identity, directory, and discovery surface those
adapters need.

Canonical product behavior:
[Self-Hosted Agent Integration Specification](SELF_HOSTED_AGENT_INTEGRATION_SPEC.md).

## Objective

Let a Buzz Desktop user **connect** an agent that already exists on another
machine, rather than **create** an agent that Buzz then owns and runs.

The distinction is load-bearing:

| | Create (upstream today) | Connect (this plan) |
|---|---|---|
| Identity custody | Buzz mints the nsec, holds it in the OS keyring | The agent's own host holds its nsec; Buzz never sees it |
| Process lifecycle | Buzz spawns and supervises `buzz-acp` | The host supervises itself; Buzz has no start/stop |
| Harness location | Local, or deployed by a `buzz-backend-*` provider | Already installed and running on the remote host |
| Buzz's role | Control plane + communication plane | Communication plane only |

Target user shape: specialist agents on separate machines, each with its own
harness, tools, resources, and skills, reachable over SSH from a laptop that is
a **viewer** and runs no agents itself.

## Decisions taken

Recorded 2026-07-27 with the user.

1. **Host-supervises-itself**, not Buzz-deploys-to-host. The `buzz-backend-*`
   provider protocol (`managed_agents/backend.rs`) is deliberately **not** used:
   it exists to *deploy*, and this plan does not deploy. No `buzz-backend-ssh`
   binary is in scope.
2. **Host-owned identities.** A connected agent may reuse an existing stable
   host-owned Buzz identity or generate a fresh keypair on its own host. The
   user must confirm reuse, generation must be explicit, and reconnecting must
   never rotate a key silently. No key is migrated out of the legacy Python
   connector's custody, and no production identity is touched during canary
   work.
3. **Isolated first identity.** The current Desktop integration uses the
   isolated OpenClaw canary identity on lunar01, not Selene's production
   identity. lunar02 remains the Hermes/development validation host. Neither
   path authorizes production connector cutover.
4. **No new event kinds and no new HTTP endpoints.** The generic Nostr surface
   is sufficient; see "Why no new kinds" below.
5. **Explicit host-side identity generation.** The preferred onboarding path
   retrieves an existing public identity and, only when absent, offers a
   confirmed one-click generation action on the selected host. Manual command
   instructions and npub/hex entry remain fallbacks.
6. **Two-stage Buzz-owned removal.** Buzz-created agents are first archived
   with their identity retained and hidden from normal surfaces. Permanent
   deletion is a separately gated action from the archived state. Connected
   agents remain disconnect-only because Buzz does not own their keys.
7. **Primary-first selective discovery.** A resident harness's primary agent is
   selected by default while the full durable roster remains available in the
   selector. The complete stack is never enrolled automatically.
8. **Native conversation routing.** Buzz channel UUIDs, thread roots, and DM
   channel UUIDs feed the harness's existing session router. DMs are not a
   separate Buzz-side session architecture.

## Why this is mostly already possible

Three findings from the 2026-07-27 code read of `upstream/main` at
`070fb6a16`. These are why the delta is small.

1. **Buzz already renders agents it does not own.**
   `commands/agent_discovery.rs:1293` (`list_relay_agents`) issues an
   unfiltered `kind:10100` query, and
   `desktop/src/features/agents/knownAgentPubkeys.ts` merges *managed ∪ relay*
   agents. An agent that self-publishes its own `kind:10100` profile becomes
   mentionable, avatar-bearing, and loop-guarded with no Desktop involvement.
2. **The agent store already tolerates key-less records.**
   `managed_agents/storage.rs` splits the unified store on
   `pubkey.is_empty()` — non-empty is an *instance*, empty is a *definition*.
   A connected agent is a third shape: real pubkey, no local key.
3. **Transport is already built.** `buzz listen` / `buzz messages send` /
   `buzz users me` exist on the `buzz-pr-identity` → `buzz-pr-listen` →
   `buzz-pr-fixtures` branch stack. This plan does not depend on those branches
   merging upstream.

## The one missing concept: non-owned agent state

`managed_agents/storage.rs:158` states the gap outright:

> Returns `Some(error)` when `private_key_nsec` is empty — after `hydrate_keys`
> an empty key means a keyring outage or a genuinely absent secret, **NOT a
> deliberately keyless agent**.

Buzz had no state for *"real agent, real pubkey, key lives on another machine,
never spawn this locally."* The final implementation adds that as a separate
record type and store rather than adding a custody value to the managed-agent
record.

## Work packages

Four implemented, independently reviewable units plus one deferred expansion.
They are ordered by dependency and deliberately smallest-and-most-defensible
first, so early work lands even if later work stalls at a coordination gate.

### RC1: `buzz keys generate` — host-side identity minting

Status: **implemented and pushed to the fork**, `buzz-rc-keys` at
`26bb74aa2`. No upstream PR is open.

**Problem.** There is currently no way for an agent to create its own Buzz
identity. Every path mints the key inside Buzz Desktop, which is exactly
backwards for remote custody: the key would be created on the laptop and then
have to be transported to the host that should have generated it.

**Scope.**

```text
crates/buzz-cli/src/commands/keys.rs   (new)
crates/buzz-cli/src/commands/mod.rs
crates/buzz-cli/src/lib.rs
crates/buzz-cli/README.md
crates/buzz-cli/tests/keys_generate.rs (new)
```

- `buzz keys generate` mints a fresh secp256k1 keypair.
- Local-only: dispatched **before** the `BUZZ_PRIVATE_KEY` requirement in
  `run()`, following the existing `Cmd::Pack` precedent (`lib.rs:1737`).
- Default output prints the **public** half only (`pubkey`, `npub`) plus the
  path written; the nsec goes to a file created with mode `0600`.
- `--out <path>` selects the destination. `--stdout` is an explicit opt-in for
  printing the nsec, for callers that pipe into their own secret store.
- Refuses to overwrite an existing file without `--force`, so re-running the
  connect wizard cannot silently destroy a live agent's identity.

**Exit criteria.**

- runs with no relay, no network, and no `BUZZ_PRIVATE_KEY` set;
- the nsec never appears on stdout unless `--stdout` was passed;
- the written file is mode `0600`;
- an existing destination is refused without `--force`;
- the generated secret round-trips to the reported pubkey.

Note on the round-trip check: `buzz users me` is the natural way to verify this,
but it lives on the `buzz-pr-identity` branch and is **not** on `upstream/main`.
RC1 must not depend on that branch, so the criterion is met two other ways —
a `Keys::parse` round trip in-process, plus a live check that a generated key is
accepted past the CLI's key-parse stage (a relay command with the generated key
against an unreachable relay exits 2/network, while a malformed key exits
3/auth). Re-verify with `users me` once that branch is present.

**Upstream unit.** PR: headless identity generation for self-hosted agents.

### RC2: `buzz agents profile` — full agent-profile publication

Status: **implemented offline and pushed to the fork**, `buzz-rc-profile` at
`ce69cc3fe`. Live profile side-effect and Desktop hydration validation remain
outstanding — see "Validation status" below.

Two deviations from the scope as first written, both deliberate:

- the logic lives in a new `crates/buzz-cli/src/commands/agent_profile.rs`
  rather than inside `agents.rs`, so the merge semantics are unit-testable
  without a relay and `channels.rs` can share the exact same write path;
- `docs/cli-external-agents.md` was not touched: it does not exist on
  `upstream/main` (it arrives with the `buzz-pr-listen` branch), and RC2 must
  not depend on that stack.

The clobber is worse than first described. The relay's `channel_add_policy`
side effect is invoked from `ingest.rs`, where a side-effect failure is
**warn-logged, not rejected** — the event is still stored and still becomes the
author's profile. So a kind:10100 published without `channel_add_policy`
replaces the visible directory record *and* leaves the relay's stored policy at
its previous value. The event log and the database diverge with nothing but a
relay-side warning. This is why the merged document is *required* to carry a
valid policy rather than defaulting to one.

**Problem, and a live bug.** `kind:10100` is a *replaceable* event.
`commands/channels.rs:1004` (`cmd_set_add_policy`) publishes a `kind:10100`
whose content is **only** `{"channel_add_policy": ...}`. Any agent that has a
name, type, capabilities, or status in its profile loses all of them the next
time that command runs, because the replaceable event is overwritten wholesale.
Two writers, one replaceable event, no merge. This is a defect on `upstream/main`
independent of this plan.

**Scope.**

```text
crates/buzz-cli/src/commands/agents.rs
crates/buzz-cli/src/lib.rs
crates/buzz-cli/README.md
docs/cli-external-agents.md
crates/buzz-cli/tests/agent_profile.rs (new)
```

- `buzz agents profile get` reads the signing identity's current `kind:10100`.
- `buzz agents profile set` writes a **complete** profile: `display_name`,
  `agent_type`, `capabilities`, `status`, `channel_add_policy`.
- `set` is read-modify-write: unspecified fields are preserved from the current
  profile rather than dropped.
- `cmd_set_add_policy` is refactored onto the same read-modify-write helper, so
  the existing command stops clobbering sibling fields. Its CLI surface and the
  `BUZZ_ACP_ALLOWED_CHANNEL_ADD_POLICIES` gate are unchanged.
- Strict validation before signing: reject unknown fields, reject any
  secret-shaped key, reject values the Desktop's `agents_from_events`
  (`nostr_convert.rs:445`) would coerce or drop.

**Exit criteria.**

- a profile written by `profile set` survives a subsequent
  `channels set-add-policy` with all fields intact (regression test for the
  clobber);
- `channel_add_policy` written by `profile set` reaches the relay's
  `set_channel_add_policy` side effect (`handlers/side_effects.rs:1078`);
- the published profile renders in Desktop's relay-agents list;
- no private key material is ever placed in profile content.

**Validation status.** The first criterion is met offline: the merge is a pure
function with unit coverage, including the policy-only-update-preserves-all-
fields regression. The second and third criteria are **not yet met** — both need
a live relay, and no Buzz relay is currently running that this plan may write
to. lunar02 is reachable and has the `buzz` CLI and a repo checkout, but no
relay process, no Redis, and no Docker, so `buzz-relay` cannot start there as-is.
lunar01 is production and excluded by this plan. Resolve the relay question
before claiming RC2 complete.

**Upstream unit.** PR: agent-authored profile publication + replaceable-event
clobber fix.

### RC3: Host-aware harness discovery

Status: **implemented, pushed, and validated against real hosts**,
`buzz-rc-discovery` at reviewed head `ad1c5a334` (feature `fde50bf7b`,
size/format and review fixes `ad1c5a334`).

Deviation from the scope as first written: the probe returns a dedicated
`RemoteHarness` / `HostProbeResult` shape rather than the existing
`AcpRuntimeCatalogEntry`. That type carries `can_auto_install`,
`node_required`, and `auth_status`, which describe actions Buzz performs on the
*local* machine. Reusing it would require fabricating those fields for a host
Buzz cannot install on or authenticate against, and the UI would then render
buttons that cannot work. Slate's own remote shape (`DetectedAgent`) is
similarly narrow; its "one shape for local and remote" lesson is preserved by
having `probe_localhost` return the same `HostProbeResult`.

Also excluded: data-directory probing (Slate's `hasDataDir`). Slate carried a
hand-maintained `KNOWN_DIRS` list; deriving `.{id}` across Buzz's twelve
harnesses would be guesswork, and a second hand-maintained list is the drift
this work package otherwise avoids. Binary presence, version, and adapter
readiness are the load-bearing signals.

Custom (tier-3) harnesses are excluded from remote probing: their definitions
describe commands on the *local* machine, so projecting them onto another host
would assert a layout nothing has verified.

**Problem.** `discover_acp_runtimes` probes localhost only. Buzz cannot answer
"which harnesses exist on lunar02?" Upstream already documents the underlying
tension — the bundled `openclaw` preset warns that
`openclaw acp` executes inside a Gateway daemon whose environment the Desktop
does not control (`managed_agents/discovery.rs:1580`). Host-aware discovery is
the generalization of a problem upstream has already written down.

**Source.** Port the proven implementation from Slate
(`/Users/dspury/Lunar-Park/slate`), which has shipped this:

| Slate file | What to port |
|---|---|
| `electron/ssh-config.ts` | `~/.ssh/config` parser, mtime-cached, `Host`/`HostName`/`User`/`Port`/`IdentityFile`, drops `*` |
| `electron/agent-discovery.ts` | single constant probe script run via `exec $SHELL -lic` so login PATH resolves; `command -v` per harness binary **and** per ACP adapter binary separately; `--version` capture; data-dir presence; `USER`/`HOST`/`OS` |
| `electron/agent-discovery.ts` | `isPasswordAuthFailure()` — classify `BatchMode=yes` denials into an actionable `password-required` status instead of raw ssh stderr |
| `electron/agent-discovery.ts` | `probeLocalhost()` reuses the identical script, so local and remote return one shape with no downstream special-casing |

Slate lessons carried over, plus two corrections found by running the probe
against lunar01 and lunar02:

- **Carried:** the probe script contains **no interpolated user input** — it is
  a constant, single-quoted into the ssh argv, and host/port reach `ssh` as
  separate argv entries.
- **Carried:** `BatchMode=yes`, with a password wall reported as a status. A
  bare `(publickey)` denial is deliberately *not* classified as a password wall
  — that is a missing or rejected key, and telling the user to install a key
  they already have sends them after the wrong fix.
- **Corrected — `-lic` hangs.** Slate runs the probe under
  `exec $SHELL -lic`. On lunar02 (`/bin/zsh`) that hung indefinitely and had to
  be killed: `-i` sources `.zshrc`, where prompt frameworks, completion init,
  and autosuggestion plugins live, and several of those block without a TTY.
  `exec $SHELL -lc` — login but not interactive — returned the complete binary
  set including a Python venv prefix. Buzz uses `-lc`.
- **Corrected — an unbounded `--version` truncates the whole probe.**
  `claude --version` never returned on lunar02, so every harness after it in the
  loop went unreported and the trailing sentinel never printed; the result read
  as a half-provisioned machine rather than a stuck command. Slate's
  whole-process timeout masks this as a generic failure. Buzz bounds each
  `--version` call individually, so a broken binary costs one `unknown` version
  instead of the entire result. The bound is hand-rolled because `timeout(1)` is
  absent from a stock macOS — exactly where the hang was found. Closing stdin on
  the version call is a second, independent guard: an ACP harness with no
  `--version` may instead start its JSON-RPC server and block reading stdin.
- **Simplified:** the shell `for` list is a flat set of binary names, with
  harness identity reattached in Rust. Slate encoded `kind=binary` pairs in the
  loop to dodge `|` being a parse error inside a `for … in` list; keeping the
  shell dumb avoids the question entirely.

**Scope addition beyond the Slate port.** The probe also reports whether the
`buzz` CLI is present on the host, and its version — a connected agent needs it
to reach the relay, so its absence is the single most useful thing to surface.

Resolving the agent's pubkey in the same round trip was considered and
deferred to RC4: it needs `buzz users me`, which is not on `upstream/main`
(it arrives with `buzz-pr-identity`), and RC3 must not depend on that stack.

**Exit criteria.**

- a host with no reachable SSH returns a classified, actionable status, never
  raw stderr;
- a password-only host is reported as `password-required` and no password is
  ever collected or stored;
- localhost and remote results are the same shape;
- probe timeouts are bounded and a hung host cannot wedge the UI;
- no harness-specific branching in the probe — the recipe table drives it.

**Upstream unit.** PR: host-aware ACP runtime discovery.

### RC4: Separate connected-agent storage and Desktop surface

**Implemented scope.**

```text
desktop/src-tauri/src/managed_agents/connected_agents.rs
desktop/src-tauri/src/commands/remote_agent_connect.rs
desktop/src/features/agents/ui/ConnectAgentDialog.tsx
desktop/src/features/agents/ui/ConnectedAgentsSection.tsx
desktop/src/shared/api/remoteAgentApi.ts
desktop/src/shared/api/remoteAgentTypes.ts
```

- `ConnectedAgentRecord` is a distinct type in `connected-agents.json`, beside
  but never mixed into `managed-agents.json`.
- The record contains only public identity and local discovery metadata:
  pubkey, Buzz-local name, SSH host alias, observed harness, and timestamps.
- `connect_remote_agent`, `disconnect_remote_agent`, and
  `list_connected_agents` own the local pointer lifecycle.
- Connected agents render in their own Agents-panel section with host, harness,
  reachability, and pubkey. The type has no status, pid, or restart fields, so
  the UI has no lifecycle controls to render.

**Exit criteria.**

- managed-agent storage remains byte-compatible and requires no migration;
- a connected agent cannot reach spawn, deploy, auto-start, profile-republish,
  or managed-agent tombstone paths because those paths accept a different type;
- a connected save cannot rewrite or erase `managed-agents.json`;
- no connected record can deserialize from an owned-agent row, and vice versa;
- connect rejects nsecs, the owner's pubkey, and cross-store pubkey/name
  collisions without echoing secret input;
- disconnect is local-only and publishes no kind:30177 tombstone or archive;
- the Agents panel exposes reachability but no process lifecycle controls.

**Focused status: implemented and clean (2026-07-29).**

```text
base upstream/main       22be8bb35
RC3 feature              fde50bf7b
RC3 prerequisite head    ad1c5a334
RC4 feature / head       098711146
```

The final implementation intentionally replaces the earlier
`KeyCustody { Local, Remote }` design. Putting custody on
`ManagedAgentRecord` made every construction site and lifecycle reader reason
about a concept that only applies to agents Buzz does not manage, required a
shared-store save path to preserve connected rows, and crossed eight file-size
ratchets. The separate type and file remove all three failure modes:

1. **Never-spawned is structural.** Lifecycle paths accept
   `ManagedAgentRecord`; `ConnectedAgentRecord` has no key, command, pid,
   timeout, or auto-start state and cannot be passed to them.
2. **Storage ownership is disjoint.** Connected saves touch only
   `connected-agents.json`; unrelated managed-agent saves cannot erase connected
   rows and connected records cannot enter key-restricted storage.
3. **Uniqueness crosses both stores.** Connect checks managed instances,
   key-less definitions, and connected records for pubkey and case-insensitive
   name collisions.
4. **Directory authority stays with the resident.** Connect emits no
   owner-signed kind:30177 claim. Disconnect removes only the local pointer and
   emits no tombstone or NIP-IA archive. The resident publishes its own
   kind:10100 profile.
5. **Channel attachment remains generic.** The existing owner-signed
   `build_add_member` path already accepts any pubkey, so RC4 does not add a
   second membership mechanism.

The connect boundary accepts npub or hex and normalizes to lowercase hex. It
requires a real `~/.ssh/config` alias because RC3 probes that same alias, but it
does not require the host to be reachable at connect time. A sleeping or
off-VPN resident host is still valid stored configuration.

**Agent pubkey resolution** remains manual in the current dialog: it accepts a
pasted npub/hex identity and points to `buzz users me`. The integrated branch
contains that CLI command, so the remaining gap is product wiring rather than a
missing primitive. The accepted target flow in
`SELF_HOSTED_AGENT_INTEGRATION_SPEC.md` retrieves and confirms the public key
when present, offers a confirmed `Generate identity on host` action when
absent, and retains command instructions and manual entry as fallbacks without
ever returning the private key to Desktop.

**Integration extensions.** The clean local integration worktree
`/Users/dspury/Projects/buzz-openclaw-integration` combines WP1-WP3 and RC1-RC4
on `lunar-park/openclaw-test-integration-2026-07-29`:

```text
e003dd88f  connect self-hosted agents Buzz does not own
be9bcc101  add connected agents to channels
b19fd1508  unify self-hosted agents and teams
```

Manual testing on 2026-07-29 proved:

- lunar01 SSH discovery and OpenClaw harness selection;
- creation of a public-only Selene connected record;
- presentation as a normal agent card;
- adding Selene to a channel;
- replacing the Welcome Team's persona members with Selene.

It did not prove a message round trip through the OpenClaw plugin. The native
channel is still disabled and Buzz channel membership does not automatically
update the plugin's subscribed channel list.

**Validation evidence recorded by the implementation commit.**

- 1910 Desktop backend tests passed;
- 3782 Desktop JavaScript tests passed;
- Rust fmt, clippy, TypeScript, Biome, and all three `pnpm check` guards passed
  against `upstream/main`;
- reconciliation reran `pnpm check` successfully and fixed the one trailing
  blank line that made `git diff --check upstream/main...HEAD` fail;
- a later focused Rust rerun was not possible offline because this worktree did
  not yet have the pinned `mesh-llm` git checkout. The recorded full run remains
  the Rust evidence for this handoff.

**Upstream unit.** PR: connected self-hosted agent storage and visibility,
stacked on the RC3 discovery PR.

### Adjacent Desktop lifecycle: Buzz-managed archive and deletion

Status: **specified, not implemented**. This is a native Buzz lifecycle
correction exposed by the all-self-hosted test, not part of connected-agent
custody.

- `Remove from My Agents` stops any required managed instances, reports linked
  team references, hides the agent from normal surfaces, and retains its
  identity in `Archived agents`.
- `Permanently delete identity` is available only from the archived state and
  irreversibly removes Buzz-held key material, managed instances,
  definition/configuration, and local team references.
- Published signed events and relay/audit history are not erasable by this
  local action.
- Starter templates remain restorable; connected agents continue to expose
  `Disconnect`, not these Buzz-owned lifecycle actions.

### RC5: Selective resident-agent enrollment

Status: **specified and deferred** until one Selene/OpenClaw identity completes
the Validation steps 6–8 below, including a correct reply after restart.

**Problem.** A resident harness can contain multiple configured agents. Inside
OpenClaw these may be described as subagents, but a user-selected one should
behave as a normal, first-class agent in Buzz. Connecting the host must not
automatically expose the entire harness stack.

**Required behavior.**

- Host discovery may enumerate durable, named resident-agent candidates in
  addition to the harness itself.
- The primary or `main` agent is selected by default, and the same selector
  exposes the full durable roster for that harness instance.
- Enrollment is opt-in per candidate. The user can connect one, several, or
  none, change the default selection, rescan later, and add another without
  rewriting existing connections.
- Every selected candidate has its own stable Buzz keypair, public profile, and
  connected-agent record. The private key remains on the resident host.
- Buzz treats each selected identity as a normal agent for profiles, mentions,
  channel membership, teams, permissions, and per-channel/thread
  conversations.
- The resident adapter maps each Buzz identity to exactly one harness agent ID;
  replies are signed by that selected agent's key, never by the parent
  identity.
- Ephemeral workers spawned for one turn are not enrollment candidates and
  remain internal to their parent agent unless the harness later promotes them
  to durable, named agents.
- Disconnecting one selected agent removes only that Buzz connection. It does
  not stop the harness, delete the harness agent, rotate another agent's key, or
  disturb sibling connections.

**Ownership boundary.**

- The OpenClaw plugin owns enumeration of OpenClaw agent IDs, identity-to-agent
  routing, and the multi-account/concurrent-lane support required to serve more
  than one selected identity.
- Buzz consumes a harness-neutral candidate shape and owns selection,
  connected-agent storage, and native Desktop presentation. No
  OpenClaw-specific runtime branch belongs in an upstream Buzz PR.
- This expansion adds no Nostr event kind or relay endpoint. Each enrolled
  agent continues to use its own existing `kind:10100` profile and normal
  channel membership.

**Exit criteria.**

- discovery preselects the primary candidate and presents the full durable
  roster without preselecting the full stack;
- selecting two candidates produces two distinct Buzz pubkeys and two normal
  agent cards;
- mentioning either agent reaches only its mapped harness agent and the reply
  is authored by the expected Buzz identity;
- independent threads and restart recovery do not cross agent IDs, keys, or
  durable state;
- unselected and ephemeral harness agents never appear in Buzz;
- adding or disconnecting one agent leaves every sibling connection intact.

### RC6: Direct-message conversation parity

Status: **specified and deferred** until the single-Selene communication gate
passes. RC6 and RC5 may proceed independently after that gate.

Buzz already gives each direct-message conversation a stable channel UUID.
OpenClaw already accepts `direct`, `group`, and `channel` peers and applies
`session.dmScope` when it builds session keys. The missing work is therefore a
transport and address-mapping extension:

- extend the Buzz CLI/listen contract to receive the resident identity's
  NIP-17 gift wraps without weakening `p`-gating;
- decrypt and normalize the DM into the same durable inbound envelope used for
  channel messages;
- pass the Buzz DM channel UUID to the resident adapter as the stable direct
  conversation ID;
- preserve an optional Buzz thread root as a thread suffix when the DM surface
  exposes one;
- call OpenClaw routing with `peer.kind = "direct"` and let the configured
  `session.dmScope` decide main-session continuity versus peer isolation;
- send and reconcile the signed reply back into the same Buzz DM conversation.

The Desktop does not create a parallel DM-to-session table. Adapter readiness
must warn when multiple permitted DM peers would share OpenClaw's `main`
session.

**Exit criteria.**

- one encrypted owner DM reaches only the intended connected identity;
- the normalized event carries the stable Buzz DM channel UUID;
- one OpenClaw turn produces one correctly signed reply in that DM;
- session reuse/isolation matches the configured `session.dmScope`;
- replay and restart do not duplicate the model turn or visible reply;
- no decrypted DM content enters logs, fixtures, or relay-searchable events.

## Why no new kinds or endpoints

Everything connect needs already has a generic representation:

| Need | Existing mechanism |
|---|---|
| Agent directory entry | `kind:10100`, agent-authored, replaceable |
| Channel membership | `kind:9000` add-member, `bot` role |
| Online/away/offline | `buzz users set-presence` |
| Inbound delivery | `buzz listen --mentions-of-me --envelope v1` |
| Outbound replies | `buzz messages send --reply-to` |
| Mention gating / loop guards | already relay- and Desktop-side |

Adding a kind here would duplicate `kind:10100` and split the directory.

## Explicit exclusions

- No `buzz-backend-ssh` provider binary (decision 1).
- No key migration out of the legacy Python connector (decision 2).
- No changes to the legacy connector or the legacy OpenClaw webhook adapter
  except production break/fix.
- No SSH password collection or storage, ever. Key-based auth only; a
  password-only host is a reported status, not a prompt.
- No agent nsec transported over SSH, and no nsec in a remote process argv —
  argv is world-readable via `ps`.
- No unapproved production-identity or connector-authority changes on lunar01.
  The corrected narrow no-final canary passed, but the native Buzz channel
  remains disabled. Re-enabling it for the Desktop-to-Selene communication
  gate requires the exact isolated identity/channel configuration and rollback
  described by the resident-adapter plan.
- No Lunar Park hostnames, service definitions, identities, or secrets in any
  upstream PR.

## Sequencing

```text
RC1 keys generate ──┐
                    ├──► manual lunar02 bring-up (no UI) ──► RC3 discovery ──► RC4 custody + panel
RC2 agents profile ─┘

one connected Selene round trip + restart recovery
                    ├──► RC5 selective resident-agent enrollment
                    └──► RC6 direct-message conversation parity
```

RC1 and RC2 are independent of each other and of the WP1–WP3 branch stack; both
start from refreshed `upstream/main`. After both land locally, the connect path
is provable **by runbook with no UI at all**: generate a key on lunar02,
publish a profile, add the agent to a channel as a bot, and confirm it appears
in the Desktop agent list. RC3 and RC4 then turn that runbook into two clicks.

All four focused units are implemented against `upstream/main` `22be8bb35`.
RC4 is stacked on RC3 because it consumes `probe_agent_host`; RC1 and RC2
remain independent. Refreshed upstream is `3e48f1b23`, 27 commits beyond that
baseline, so the focused branches require refresh before publication.

The integrated branch has now exercised runbook steps 2–6 against the live
Lunar Park relay with the isolated OpenClaw canary identity. What remains is
the communication/restart portion: configure the native plugin for the exact
Desktop test channel, enable only that isolated account, produce one correctly
signed Selene reply, restart, and prove no duplicate. Automatic public-key
resolution can then replace the dialog's manual copy step.

## Validation

Bring-up order for the current first connected agent, using the isolated
OpenClaw identity and a dedicated canary channel:

1. Confirm the intended host-owned identity; generate one on the host only if
   no suitable isolated identity exists.
2. Confirm `buzz users me` reports the expected public key.
3. `buzz agents profile set` with display name, type, capabilities,
   `channel_add_policy`.
4. Confirm the agent appears in Desktop's relay-agents list.
5. Owner adds the agent to the canary channel as `bot`.
6. Confirm the agent is mentionable from Desktop.
7. Run the resident loop (`buzz listen` → harness → `buzz messages send`) and
   confirm one owner mention produces exactly one reply, correct author,
   correct `h` tag, correct parent and root.
8. Restart the agent process and repeat step 7.

Steps 2, 5, and 6 have been observed in the current Desktop integration;
profile hydration and automatic identity resolution remain incomplete. Step 7
depends on the harness adapter owned by the resident-adapter plan and is the
current blocking gate.

Buzz quality gates apply unchanged: `just ci` before every PR, `git commit -s`
for DCO, no new `unwrap()`/`expect()` in production paths, doc comments on new
public API.

## Coordination

The blocked upstream PRs (#2633, #2933, #2942) overlap the WP1–WP3 identity and
listen slices. RC1–RC4 were chosen to **not** touch that surface, so they are
publishable without waiting on that coordination gate. Re-check for overlap
before opening each PR. As refreshed on 2026-07-29, all three remain open:
#2633 at `7b76647e7`, #2933 at `837958d5f`, and #2942 at `46987a0fe`.
Refreshed `upstream/main` is `3e48f1b23`; every focused RC branch is 27 commits
behind and must be rebased and revalidated before publication.

## Working locations

The main checkout at `/Users/dspury/Projects/buzz` is a mixed planning worktree
and must not carry implementation commits. Each work package gets a focused
worktree branched from refreshed `upstream/main`.

| Work package | Worktree | Branch |
|---|---|---|
| RC1 | `/Users/dspury/Projects/buzz-rc-keys` | `buzz-rc-keys` (created) |
| RC2 | `/Users/dspury/Projects/buzz-rc-profile` | `buzz-rc-profile` (created) |
| RC3 | `/Users/dspury/Projects/buzz-rc-discovery` | `buzz-rc-discovery` (created) |
| RC4 | `/Users/dspury/Projects/buzz-rc-custody` | `buzz-rc-custody` (created, stacked on RC3) |
| Integrated test | `/Users/dspury/Projects/buzz-openclaw-integration` | `lunar-park/openclaw-test-integration-2026-07-29` |
| RC5 | not created | deferred until the single-Selene communication gate passes |
| RC6 | not created | deferred until the single-Selene communication gate passes |

## Execution log

- 2026-07-27: plan opened. Decisions 1–4 recorded. Code read completed against
  `upstream/main` at `070fb6a16`.
- 2026-07-27: RC1 implemented on `buzz-rc-keys` (worktree
  `/Users/dspury/Projects/buzz-rc-keys`, refreshed to `upstream/main`
  `22be8bb35`), commit `26bb74aa2`. `buzz keys generate` with `--out` /
  `--stdout` / `--force`; secret written at mode `0600` via `OpenOptions::mode`
  at creation; overwrite refused through atomic `create_new`; secret withheld
  from stdout and stderr unless `--stdout`. 6 unit tests + 6 subprocess tests;
  full `buzz-cli` suite 262 passed; `cargo fmt` clean; `cargo clippy
  --all-targets -D warnings` clean. Not pushed; no upstream PR opened.
- 2026-07-27: RC2 implemented on `buzz-rc-profile` (refreshed to
  `upstream/main` `22be8bb35`), commit `ce69cc3fe`. New shared
  `agent_profile` read-modify-write module; `buzz agents profile get` / `set`;
  `channels set-add-policy` refactored onto the same path with its CLI surface
  and deployment gate unchanged. 11 unit tests; full `buzz-cli` suite 261
  passed; fmt and clippy clean.
  - Found during implementation: the relay's side-effect failure path is
    warn-only, which makes a policy-less kind:10100 diverge the event log from
    the stored policy column. Recorded above; it is the reason the merged
    document requires a policy instead of defaulting to one.
  - Found by running the built binary rather than trusting the unit tests:
    validation originally ran *after* the relay fetch, so a bad `--policy`
    reported a network error (exit 2) instead of an input error (exit 1).
    Fixed by validating the caller's values before any network call, with a
    regression test.
- 2026-07-27: RC3 implemented on `buzz-rc-discovery` (refreshed to
  `upstream/main` `22be8bb35`), feature commit `a78843c23`. 1,500 lines:
  `ssh_config` parser, `remote_probe` (script build, parse, failure
  classification, ssh + localhost probes), three read-only Tauri commands
  (`list_ssh_hosts`, `probe_agent_host`, `probe_local_agent_host`), TS types and
  wrappers. Probe targets projected from `KNOWN_ACP_RUNTIMES` +
  `PRESET_HARNESSES` so adding a preset extends remote discovery for free.
  28 new tests; full Tauri suite 1816 passed; fmt, clippy, `tsc`, and biome
  clean.
  - **Validated against real hosts.** lunar01: `buzz`, `claude 2.1.205`,
    `codex-cli 0.144.1`, `openclaw 2026.7.1 (5f39975)`. lunar02: `buzz`,
    `claude` (version `unknown` — see below), `codex-cli 0.143.0`,
    `hermes-acp 0.18.2`. The OpenClaw and Hermes versions independently match
    the ones recorded in the resident-adapter plan's WP4/WP6 spikes.
  - Two real bugs found only by running it, both now regression-tested: the
    inherited `-lic` login shell hangs on a real zsh host, and an unbounded
    `--version` silently truncates the whole probe. See the RC3 section.
- 2026-07-27: **blocked on a relay for live validation of RC2 only.** lunar02 is reachable
  (`Darwin`, `buzz` CLI at `/Users/miles/.local/bin/buzz`, checkout at
  `/Users/miles/Projects/buzz`, Postgres present) but runs **no relay**, and has
  no Redis and no Docker, so `buzz-relay` cannot be started there without setup.
  Local Docker is not running either. Steps 4–8 of the Validation runbook, and
  the RC2 exit criteria that need a relay, are outstanding. Next: decide the
  relay target, then RC3.
- 2026-07-28: **RC3 had two latent CI failures that RC4 surfaced**, fixed on
  `buzz-rc-discovery` at `e1805f726` before final RC4 implementation. RC3's gate
  run used `just desktop-tauri-{check,test,clippy}` plus `tsc` and biome, which
  skips two things: `pnpm check` also runs file-size, px-text, and
  pubkey-truncation guards, and `cargo fmt` for the Tauri crate is unreachable
  from the root workspace manifest (desktop is excluded from it). RC3 was
  therefore 61 lines over `discovery.rs`'s size ratchet, 73 over `types.ts`'s,
  and had `mod remote_agent_discovery;` outside rustfmt's module ordering.
  Fixed by splitting, as the checker instructs, not by bumping: probe targets
  moved to the child module `discovery/probe_targets.rs` (a child still sees
  `discovery`'s private tables, so nothing had to be made more visible — a
  sibling module would have required widening `PresetHarness` and all six of its
  fields), and the SSH/probe types moved to `shared/api/remoteAgentTypes.ts`.
  **Gate lesson for every remaining work package: run `pnpm check` and
  `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check`, not just
  the `just desktop-tauri-*` recipes.**
- 2026-07-28: the initial RC4 custody-field design was retired before handoff.
  It passed behavior tests but put a remote-only concept on
  `ManagedAgentRecord`, required shared-store preservation logic, and crossed
  eight file-size ratchets. The final reviewed RC4 feature commit `098711146`
  uses a distinct `ConnectedAgentRecord` in `connected-agents.json`, with
  connect / disconnect / list commands and the Connected Agents Desktop
  section. It records 1918
  backend tests and 3782 Desktop JavaScript tests passing, plus fmt, clippy,
  TypeScript, Biome, all three `pnpm check` guards, and `git diff --check`. The
  earlier trailing-blank-line reconciliation commit was squashed into it, so RC4
  is a single feature commit.
- 2026-07-29: code review found four correctness defects across both lanes; all
  four are fixed with falsified regression tests (each new test was confirmed to
  fail against the pre-fix code, not merely to pass after it).
  - RC3 `probe_ssh_host` used `StrictHostKeyChecking=accept-new`, contradicting
    its own comment: clicking Probe wrote a first-seen host key into the user's
    `known_hosts`. Now `=yes`, with unknown and changed keys reported as
    distinct statuses and the option list asserted in a test, since nothing else
    in the module touches trust state.
  - RC3 `parse_ssh_config_str` reconstructed stanza boundaries by comparing
    field values, so `Host alpha` / `Host beta` / `Port 2222` applied the port to
    both — and `probe_ssh_host` forwards it as `-p`, probing alpha on beta's
    port. A `Host *` block could likewise overwrite a real host's values. The
    parser now tracks the exact entry range each `Host` line creates.
  - RC3 `run_probe` required only `PROBE_START`, so a session dropped mid-probe
    reported `ok: true` with partial facts — indistinguishable from a host where
    the missing harnesses genuinely are not installed. Both markers are now
    required, with a new `truncated` error kind.
  - The three fixes are folded into RC3's own commits (`fde50bf7b`,
    `ad1c5a334`), so RC3 passes every gate independently rather than depending
    on a later fix commit. `remote_probe.rs` test module moved to
    `remote_probe_tests.rs` to stay inside the size ratchet, matching the
    `#[path = "..._tests.rs"]` convention already used in that module.
  - Lane B `processPendingRecord` dead-lettered any non-fatal failure after
    `MAX_EVENT_ATTEMPTS` without consulting `retryable`, then advanced the
    cursor — so a relay, disk, harness, or uncertain-send outage spanning five
    attempts permanently destroyed the queued events. Both the constant's own
    docstring and `CompletionMetadata` already specified deterministic-only.
    Dead-lettering is now gated on `!retryable`; retryable failures route to the
    existing bounded-backoff reconnect path in `run()`, which replays them.
- 2026-07-29: the local integration branch
  `lunar-park/openclaw-test-integration-2026-07-29` was rebuilt on
  `upstream/main` `047533c56` and combined WP1-WP3 plus RC1-RC4. Commits
  `be9bcc101` and `b19fd1508` added native channel attachment, normal agent-grid
  presentation, mentions, and connected-agent team membership. Desktop
  JavaScript reported 3795 passing tests; the Tauri suite reported 1932
  passing, 14 ignored, and three diagnostics tests passing; `pnpm check` and
  the production Vite build passed.
- 2026-07-29: manual UI testing discovered lunar01/OpenClaw, connected the
  isolated canary pubkey as Selene, added her to a channel, and replaced the
  Welcome Team membership with the connected identity. The application remains
  runnable from that clean integration worktree. No Selene message round trip
  has passed yet.
- 2026-07-29: testing feedback added three explicit product items to
  `SELF_HOSTED_AGENT_INTEGRATION_SPEC.md`: automatic host-side public-identity
  resolution/generation, reversible removal of Buzz starter agents, and RC5
  selective enrollment of durable OpenClaw subagents as normal Buzz agents.
- 2026-07-29: product review accepted explicit one-click host-side generation,
  two-stage archive/permanent-delete semantics for every Buzz-created agent,
  primary-first OpenClaw roster selection, and native DM routing using the
  stable Buzz DM channel UUID plus OpenClaw `session.dmScope`. RC5 and RC6
  remain deferred until the single-Selene communication gate passes.

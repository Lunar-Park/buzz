# Session Handoff — 2026-07-29

Status: current handoff for the OpenClaw/Buzz Desktop integration test.
Supersedes the
[pre-integration review handoff](history/SESSION_HANDOFF_2026-07-29_PRE_INTEGRATION.md).

Read first:

1. [Lunar Park Fork Document Index](LUNAR_PARK_FORK_INDEX.md)
2. [Self-Hosted Agent Integration Specification](SELF_HOSTED_AGENT_INTEGRATION_SPEC.md)
3. [Remote Agent Connect Implementation Plan](REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md)
4. [Resident Agent Adapter Implementation Plan](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md)

## 1. Binding stop state

- The native OpenClaw Buzz channel on lunar01 is loaded, **wired to the Gate C
  identity/channel pair, and still disabled**. See
  [§3A Gate C wiring](#3a-gate-c-wiring-and-blockers).
- The legacy Python connector and webhook adapter remain the production and
  rollback path.
- No native/legacy reply-authority cutover has occurred.
- The corrected narrow no-final canary passed; the full corrected Checkpoint C
  matrix has not been rerun.
- The current Desktop build has connected Selene as metadata but has not
  completed a message round trip through OpenClaw.
- **One blocker must clear before enablement:** the legacy production Selene
  identity is still a member of the Gate C channel and the legacy connector
  holds a global mention subscription. Remove it first (§3A).
- Do not implement selective OpenClaw-agent enrollment until the single-Selene
  communication and restart gate passes.
- Do not implement direct-message parity until that same gate passes. RC5
  roster enrollment and RC6 DM delivery may proceed independently afterward.
- Do not merge or publish implementation branches during the documentation
  review.

## 2. Current repository state

State refreshed 2026-07-29 after fetching `upstream/main` and the focused fork
branches.

### Planning checkout

```text
worktree         /Users/dspury/Projects/buzz
branch           feature/agent-harness-adapter-alignment
snapshot parent  95fdf9788
upstream         3e48f1b23
```

This is a mixed, intentionally dirty planning checkout. It contains the
fork-planning documents plus pre-existing modified acceptance/configuration
files. Do not use it for implementation or bundle its contents into an
upstream PR.

The 12-file documentation-control set is committed locally as one signed,
docs-only snapshot containing this handoff. It has not been pushed.
Pre-existing `.github/workflows/ci.yml`, `ARCHITECTURE.md`, `Justfile`, and two
acceptance-script changes remain unstaged and outside the snapshot.

### Integrated Buzz test branch

```text
worktree  /Users/dspury/Projects/buzz-openclaw-integration
branch    lunar-park/openclaw-test-integration-2026-07-29
head      b19fd1508
state     clean, local-only
delta     13 commits ahead / 8 commits behind refreshed upstream/main
```

The 13-commit stack contains:

```text
WP1-WP3  external-agent identity, listen contract, and fixtures
RC1-RC2  host-side key generation and safe profile publication
RC3      SSH host/harness discovery
RC4      separate public-only connected-agent storage
be9bcc101  add connected agents to channels
b19fd1508  unify connected agents with normal agents and teams
```

Do not rebase this branch before preserving the current manual test point and
reviewing the eight new upstream commits.

### RC5 harness-roster branch

```text
worktree  /Users/dspury/Projects/buzz-rc5-roster
branch    lunar-park/rc5-harness-roster
head      14103c14d
base      b19fd1508 (the preserved integration test point)
state     clean, local-only
```

Branched from the integration test point rather than committed onto it, so the
documented manual test point stays byte-identical. Contains harness-neutral
durable agent-roster detection: `probe_harness_agents` /
`probe_local_harness_agent_roster`, a `ROSTER_RECIPES` table whose only current
row runs `openclaw agents list --json`, and the neutral `RemoteAgentCandidate`
shape. Primary selection implements the spec's "harness primary or `main`".

Validation: 1958 Tauri tests (26 new), fmt, clippy `-D warnings`, `tsc`, biome,
px-text, and pubkey-truncation all clean. The assembled remote command was run
against lunar01 and returned both markers with all eight durable agents and
`main` flagged default.

**Gate finding:** `pnpm check`'s desktop file-size ratchet already fails on
`b19fd1508` itself, byte-identically, with 17 entries. Its base is
`76aeae7036` while the branch carries 13 commits of growth. RC4's recorded
"all three `pnpm check` guards passed" was measured on the focused RC4 branch
against `upstream/main`, where the base matched; it does not hold for the
integration stack. Any PR cut from this lineage needs a rebase or a split
before that guard can pass, and the RC5 commit adds no new entry.

### P6 owner-attestation branch

```text
worktree  /Users/dspury/Projects/buzz-rc5-roster
branch    lunar-park/rc-p6-owner-attestation
head      e14d90e9b
base      lunar-park/rc5-harness-roster @ 14103c14d (stacked)
state     clean, local-only
```

Adds `mint_connected_agent_owner_evidence` and
`get_connected_agent_owner_evidence`: Buzz signs a NIP-OA attestation for a
connected agent locally and returns it for the user to install on the host as
`BUZZ_AUTH_TAG` / `channels.buzz.authTag`. Buzz installs nothing itself and
publishes nothing — the attestation takes effect through the agent's own later
events.

The delivery chain was verified link by link in code, not assumed:

```text
Desktop mints tag (buzz_sdk::nip_oa::compute_auth_tag)
  -> host installs it as the adapter's auth tag
  -> buzz-ws-client build_auth_event(challenge, relay, keys, auth_tag)
       puts it in the kind:22242 AUTH event
  -> relay handlers/auth.rs extract_auth_tag_json(&event)
  -> materialize_nip_oa_owner  (open relays: auth.rs:244 backfill;
                                closed relays: delegated membership)
  -> users.agent_owner_pubkey set
  -> owner_only channel adds accept the owner, and a closed relay admits the
     agent because its attested owner is a member
```

`conditions` is deliberately empty: the membership and channel-add paths verify
only the signature and never evaluate a clause, so a `kind=` or `created_at<`
value would restrict nothing while reading as a restriction. A test pins the
empty value so the decision must be revisited deliberately if the relay ever
enforces clauses there.

Validation: 1970 Tauri tests (+12), fmt, clippy `-D warnings`, `tsc`, biome, and
both remaining guards clean; no new file-size ratchet entry. Storage fields are
optional with serde defaults and a test loads a pre-field store.

### Connect-flow UI

```text
head   878ca7896  (same worktree/branch, third commit)
```

Wires both slices into the connect flow:

- choosing a harness lists its durable agents with the primary preselected and
  the rest visible but unselected; selecting one prefills the agent name only
  while the field is untouched;
- `harness_agent_id` is persisted, so roster picking is a stored
  Buzz-pubkey-to-harness-agent mapping rather than a cosmetic choice;
- each connected agent has an owner-attestation dialog that reads existing
  evidence rather than re-minting, copies the tag, and names where it goes
  (`BUZZ_AUTH_TAG`, or `channels.buzz.authTag`).

Validation: 1970 Tauri tests, 3807 Desktop JavaScript tests, fmt, clippy, tsc,
biome, both remaining guards, no new ratchet entry, and a passing production Vite
build. The roster rules live in the pure intent module and are unit-tested; the
rendered UI has **not** been visually verified — launch the app from
`/Users/dspury/Projects/buzz-rc5-roster` to see it.

### P1 host-side identity onboarding

```text
heads  8a8f06d80 (backend + API), b90423ed0 (connect-flow UI)
```

- `resolve_host_agent_identity` reads the identity a harness is configured to
  sign as. For OpenClaw that is `channels.buzz.agentPubkey`, which the plugin
  already verifies its loaded key against at startup, so it answers §3.2 item 6
  rather than guessing from a shell environment. Verified live against lunar01:
  it returns the configured canary pubkey exactly. An unset config path is
  reported as "no identity yet", not a failure.
- `generate_host_agent_identity` runs `buzz keys generate --out` on the host.
  The secret is written there at mode `0600`; only the public half and the path
  return. `--stdout` and `--force` are never passed, so an agent with an existing
  key fails loudly instead of losing its identity, and a reply that contained a
  secret would abort rather than be quietly discarded.
- Generation is the **only host write** in this surface and the only place
  user-influenced text reaches a remote shell. `validate_identity_slug`
  whitelists the agent id and refuses everything else rather than escaping;
  quoting, injection, traversal, and `..` are covered, and the assembled command
  is parse-checked with `sh -n`.
- The connect dialog offers the resolved identity for one-click acceptance, or a
  two-step confirmation naming the host before generating, and keeps manual entry
  as the fallback.

The generate path was **not** run live: it writes a key file to a host and needs
its own approval. Everything else was.

Validation across the P1 commits: 1992 Tauri tests (+22), 3807 Desktop JavaScript
tests, fmt, clippy `-D warnings`, tsc, biome, both guards, no new ratchet entry,
production Vite build.

### Still open after this session

- Connected-agent records remain global rather than community-scoped
  (specification §4.2 item 1) — the reason Selene appears in both communities.
- The rendered UI for the roster picker, attestation dialog, and identity field
  is unit-tested but has not been visually verified.
- The OpenClaw adapter still serves one Buzz account, so multi-agent
  *communication* remains WP5A regardless of what Buzz can now enroll. Buzz-side
  detection, selection, identity minting, and attestation are in place.
- Gate C itself is unchanged and still needs an operator: remove the legacy
  Selene from `dff91016…`, enable, send one mention, verify, restart, disable.

### Focused Buzz branches

All focused worktrees are clean, pushed, and match their `origin/*` refs:

| Slice | Head | Relationship |
|---|---|---|
| WP1 identity | `e14a94ec3` | two commits over `22be8bb35` |
| WP2 listen | `1d65784d5` | WP1 plus two |
| WP3 fixtures | `68513902c` | WP2 plus two |
| RC1 keys | `26bb74aa2` | independent |
| RC2 profile | `ce69cc3fe` | independent |
| RC3 discovery | `ad1c5a334` | two commits |
| RC4 connected agents | `098711146` | RC3 plus one |

Refreshed upstream is 27 commits beyond their `22be8bb35` baseline. Rebase and
full validation are required before publication.

### OpenClaw plugin

```text
repo        /Users/dspury/Lunar-Park/integrations/openclaw-channel-buzz
local head  3e4e73a
source head aebd8dd
remote      none configured
tests       22 passed; typecheck/audit/pack previously passed
```

The local working tree contains intentionally untracked `AGENTS.md` and
`CLAUDE.md`.

Read-only lunar01 verification during this documentation pass:

```text
OpenClaw       2026.7.1 (5f39975)
channels.buzz  false
plugin ID      buzz
plugin status  loaded, no diagnostics
loaded source  /Users/selene/Lunar-Park/integrations/openclaw-channel-buzz-aebd8dd
```

The corrected narrow no-final rerun produced one durable canary-signed `👀`
reaction, no text, no pending row, and no duplicate after Gateway restart. The
candidate was disabled afterward. Full evidence is in the plugin repository's
`docs/CHECKPOINT_C.md`.

## 3. Desktop build and manual test state

The integration worktree's standalone app was built and launched with:

```sh
cd /Users/dspury/Projects/buzz-openclaw-integration
. ./bin/activate-hermit
just desktop-standalone
```

The app was relaunched later the same day from the same worktree, used for the
Gate C setup, and then **stopped**. No Buzz Desktop process is running at
handoff; port 37299 is free. Relaunch with:

```sh
cd /Users/dspury/Projects/buzz-openclaw-integration
. ./bin/activate-hermit
just desktop-standalone
```

```text
Buzz Dev (openclaw-test-integration-2026-07-29)
Vite port 37299
bundle ID xyz.block.buzz.app.dev.lunar-park-openclaw-test-integration-2026-07-29
```

Every piece of Gate C setup survived the shutdown, so relaunching needs no
re-onboarding and no repeated wiring:

```text
connected-agents.json  Selene / 4687f50d… / lunar01 / openclaw
teams.json             Welcome Team → [4687f50d…], no persona ids
communities            lunarpark (hosted) and lunar01 (ws://lunar01:3000,
                       id f0e51b11-847f-4b87-88c0-3c052e66d0e6, added 23:10:42Z)
active community       lunar01 — the app opens straight into the Gate C community
```

Recorded validation for `b19fd1508`:

```text
Desktop JavaScript  3795 passed
Tauri Rust          1932 passed, 14 ignored
Tauri diagnostics   3 passed
pnpm check          passed; two pre-existing Biome infos only
Vite production build passed
```

Manual UI observations:

1. SSH host discovery found `lunar01`.
2. The probe found OpenClaw `2026.7.1 (5f39975)`.
3. The dialog required manual public-key entry; it did not retrieve the
   identity from the host.
4. Selene connected using isolated canary pubkey
   `4687f50de3a9e235e28eb58d68b0746062d7be6401bbf78a766bbd6f96ffe3c9`.
5. Selene appeared as a normal agent card.
6. Adding Selene to a Buzz channel worked.
7. Connected-agent team membership worked; the local Welcome Team now contains
   Selene and no persona IDs.
8. Fizz, Honey, and Bumble could not be removed cleanly. Each built-in
   definition still has a linked Buzz-managed instance, while the menu labels
   the built-in deactivation path as `Delete`.
9. No message has successfully traveled from the current Buzz build through
   the native OpenClaw plugin and back as Selene. §3A explains why: the build
   and the adapter were on two different communities, and a connected agent
   publishing `owner_only` could not be added to a channel at all.
10. A second community, `ws://lunar01:3000`, has been added to this build for
    the Gate C run. The `lunarpark` hosted community remains configured and
    untouched, including its `selene-openclaw-test` channel
    (`70c8cefb-f62e-41c1-aa8c-53055ca7d09f`), which is now unused for Gate C.

Persisted public-only local state:

```text
~/Library/Application Support/
  xyz.block.buzz.app.dev.lunar-park-openclaw-test-integration-2026-07-29/
    agents/connected-agents.json
    agents/teams.json
```

No resident private key is stored there.

## 3A. Gate C wiring and blockers

Recorded at the end of the 2026-07-29 evening session. Everything in this
section was verified against live state, not inferred from a document.

### Identity and channel mapping

```text
Desktop build   Buzz Dev (openclaw-test-integration-2026-07-29)
app data        xyz.block.buzz.app.dev.lunar-park-openclaw-test-integration-2026-07-29
webview state   ~/Library/WebKit/buzz-desktop
owner identity  6ff3b9d49d59b3e6656dc3a938f800bc3b0d0b675969593fd85ea018bc34c297
connected agent 4687f50de3a9e235e28eb58d68b0746062d7be6401bbf78a766bbd6f96ffe3c9
Gate C channel  dff91016-e5d1-4929-b4d8-5d78b3379f05  "selene-gate-c"  (private)
Gate C relay    ws://lunar01:3000   community fa7e3353-bcfe-4742-bfe6-64ca1e2357c7
```

### The decisive finding: two communities, not one

Buzz derives the community boundary from the relay host, so the Desktop build
and the adapter were never on the same community:

- the integration build's only community was `lunarpark` at
  `wss://lunarpark.communities.buzz.xyz` — a Cloudflare-fronted hosted relay
  with `auth_required` and `restricted_writes`;
- the plugin, pinned CLI, canary identity, and legacy connector all live on the
  local relay at `ws://lunar01:3000`;
- each community has its own `general` (`32473c76…` versus `4d6d3cc4…`).

A bounded read-only `buzz listen` proved the consequence directly. Against the
hosted community the canary is refused at NIP-42:

```text
{"state":"fatal","message":"auth error: … Authentication failed: restricted: not a relay member"}
```

Against the local community the same identity connects, replays, and reaches
EOSE. Desktop channel membership is not relay membership; the hosted relay
would need an owner-signed kind:9030 or a NIP-OA auth tag for the canary.

Earlier plan text claiming the integration branch exercised runbook steps 2-6
"against the live Lunar Park relay" was therefore imprecise: it exercised them
against the hosted community while the adapter listened to the local one. No
transport defect was involved — the round trip was never reachable.

The user chose to run Gate C on the local `ws://lunar01:3000` community, where
the canary already authenticates and the owner identity is already established.

### Second finding: owner_only blocks connected-agent channel adds

Adding the connected agent to a channel failed with:

```text
4687f50d…e3c9: relay returned 400 Bad Request:
invalid: policy:owner_only — agent has no owner set
```

`handlers/side_effects.rs` enforces `channel_add_policy` on the *target* of a
third-party add. `owner_only` requires a materialized `agent_owner_pubkey`,
which the relay only backfills from a valid NIP-OA auth tag. Buzz never
establishes that relationship for a connected agent, because it does not own
the key and mints no attestation — so an owner cannot add its own connected
agent to any channel once that agent publishes `owner_only`. It succeeded in
the hosted community only because the canary had never published a profile
there. This is a concrete instance of P4 and is tracked as P6 in the
specification.

Worked around agent-side: the canary republished its own policy as `anyone`
(`44b1642026fac3a789f1e7fec211b344967ede805537ff702a1330a57a47447c`), after
which the owner-signed add succeeded. Upstream `channels set-add-policy`
publishes a kind:10100 containing only the policy field, so it clobbered the
canary's other profile fields — the exact replaceable-event defect RC2 fixes.
The pre-change profile event was `c9760112…`, content:

```json
{"agent_type":"openclaw","capabilities":["conversation"],
 "channel_add_policy":"owner_only","display_name":"Selene","status":"offline"}
```

Restoring the full field set requires RC2's `buzz agents profile set`; the
pinned canary binary can only restore the policy value. A side benefit: the
canary no longer publishes `display_name: "Selene"`, which removes a real
footgun — two different pubkeys both presented as "Selene" in that community
and the legacy production identity was selected by mistake once.

### Live configuration now on lunar01

```text
OpenClaw          2026.7.1 (5f39975)
plugin buzz       loaded, no diagnostics,
                  /Users/selene/Lunar-Park/integrations/openclaw-channel-buzz-aebd8dd
enabled           false
agentPubkey       4687f50de3a9e235e28eb58d68b0746062d7be6401bbf78a766bbd6f96ffe3c9
ownerPubkey       6ff3b9d49d59b3e6656dc3a938f800bc3b0d0b675969593fd85ea018bc34c297
channelIds        ["dff91016-e5d1-4929-b4d8-5d78b3379f05"]
relayUrl          ws://lunar01:3000
requireMention    true
allowlistedPubkeys []
buzzCliPath       /Users/selene/.local/bin/buzz-openclaw-canary-68513902c
silentAckEmoji    👀
config validate   valid; one pre-existing duplicate-plugin-id warning
```

`buzz users me` under the configured signing key returns
`4687f50d…e3c9` / `npub1g6rl2r0r483rtc5wkkxk3vr5vp3d00nyqxal0znkdw7kl9hlu0ys3mpklf`,
so the loaded key matches `agentPubkey`.

Adapter delivery state is unchanged by this session:

```text
cursor 1 | inbound.completed 8 | outbound 6 | pending 0
cursor lastCreatedAt 1785352970  lastEventId 3868c3c4…
```

The Gateway PID moved `20325 → 10374` during the session without a host reboot
(uptime 11 days) — a LaunchAgent respawn. Buzz was disabled throughout, so no
listener ran and no delivery state changed. Discord probe `ok`; both legacy
health endpoints returned HTTP 200 before and after.

### Remaining blocker before enablement

`selene-gate-c` currently has three members:

```text
6ff3b9d4…c297  owner   Desktop owner
4687f50d…e3c9  member  connected canary            ← intended
2311ce81…efa6  member  legacy production Selene    ← must be removed
```

`connector/router.py` subscribes each legacy agent to its configured channels
**and** to a global mention filter:

```python
await self.client.subscribe("agent_mentions",
    {"kinds": [9, 11], "#p": [self.identity.public_key], "limit": 0})
```

So the production Selene will act on any `p`-tagged mention of itself in any
channel it can read. The per-identity invariant is not violated — the pair
`(4687f50d, dff91016)` has exactly one writer, and `dff91016` is absent from
the connector's configured channel list (`b5169461`, `4d6d3cc4`) — but a
production identity sitting in the canary channel is an unnecessary
cross-authority hazard and a mention aimed at the wrong "Selene" would pollute
the evidence. Removal needs an owner-signed kind:9001 from Desktop.

### Exactly what remains for Gate C

1. Remove `2311ce81…efa6` from `dff91016…`.
2. Enable: `channels.buzz.enabled true --strict-json`, `config validate`,
   `gateway restart`, `gateway status`, Discord probe.
3. Send **one** owner mention from the Desktop app, selecting the connected
   agent from the `@` autocomplete so a `p` tag is emitted. A plain-text
   `@Selene` does not qualify: the message already in that channel
   (`e2eefce8…`, `@Selene hey`) carries only an `h` tag and will be replayed
   from the cursor and durably classified missing-mention with no reply — a
   free negative case worth capturing.
4. Verify one reply: author `4687f50d…`, `h` = `dff91016…`, correct immediate
   parent, correct NIP-10 root; `inbound.completed` with `activation=accepted`,
   one `outbound status=sent`, zero pending.
5. `gateway restart`, then re-verify: still one reply, one outbound row, no
   second model turn.
6. Disable and preserve evidence.

Reply verification needs no owner private key — the relay's own store is
authoritative:

```sql
SELECT encode(id,'hex'), encode(pubkey,'hex'), kind, tags
  FROM events
 WHERE community_id='fa7e3353-bcfe-4742-bfe6-64ca1e2357c7'
   AND channel_id='dff91016-e5d1-4929-b4d8-5d78b3379f05'
   AND deleted_at IS NULL
 ORDER BY created_at;
```

Rollback restores `ownerPubkey=626ad40d…6609` and
`channelIds=["d18b14c3-7e55-48b7-b980-f9de29ba5cb8"]` through the same batch
form, then `config validate`, `gateway restart`, and both legacy health checks.
Do not touch `plugins.load.paths`, the legacy LaunchAgents, or the canary
secret.

## 4. Product issues now tracked in the specification

### P1 — automatic identity onboarding

The connect flow should retrieve and confirm an existing host-owned public
identity. If none exists, the accepted primary path is a confirmed
`Generate identity on host` action; the exact command and manual npub/hex entry
remain fallbacks. No private key may return to Desktop.

### P2 — two-stage Buzz-managed agent removal

Every Buzz-created agent first uses reversible `Remove from My Agents`: stop as
needed, report linked managed instances and team references, hide it from
normal surfaces, and retain its key and definition under `Archived agents`.
Only the archived state offers separately confirmed `Permanently delete
identity`, which wipes Buzz-held key/configuration/instance state but cannot
erase previously published events or relay/audit history. Starter templates
remain restorable. Connected agents retain disconnect because their identity
is host-owned.

### P3 — selective OpenClaw-agent enrollment

After the single-Selene gate passes, discovery may list durable named OpenClaw
agents. The primary or `main` agent is selected by default and the full durable
roster appears in the same selector; the stack is never enrolled
automatically. Every selected agent gets a distinct host-held Buzz key and
behaves as a normal Buzz agent. Ephemeral workers are not exposed.

### P4 — readiness must cover both sides

Buzz membership and connected metadata do not update the OpenClaw plugin's
`channelIds` or enable its account. The UI must not imply communication
readiness until relay membership, adapter identity, and adapter channel
subscription all agree.

### P5 — direct messages

DMs are required for first-class parity but do not block the current
single-Selene channel gate. Buzz already assigns each DM a stable channel UUID.
RC6 should decrypt/normalize the NIP-17 event, pass that UUID to OpenClaw as a
`direct` peer, and let OpenClaw's existing `session.dmScope` control native
main-session continuity or peer isolation. This is an ingress and mapping
extension, not a second DM session architecture.

### P6 — connected-agent addressability

Specified in `SELF_HOSTED_AGENT_INTEGRATION_SPEC.md` §4.2. Connecting an agent
must establish owner evidence for its pubkey, record which community the
connection belongs to, and ensure relay membership on relays that require it.
Until then, an owner cannot add its own connected agent to a channel when that
agent publishes `owner_only`, and Desktop can present a connection as ready in a
community the adapter cannot reach. Weakening the agent's policy to `anyone` is
the current workaround and is not the target design.

## 5. Next-session resumption sequence

1. Use the accepted decisions in
   `SELF_HOSTED_AGENT_INTEGRATION_SPEC.md` as the product baseline.
2. Steps 2-4 of the previous list are complete; the results are in §3A. The
   identity/channel pair is resolved, compared, and already written to the live
   configuration with the channel still disabled.
3. Execute the six remaining Gate C steps in
   [§3A](#exactly-what-remains-for-gate-c). It needs an operator at the Desktop
   app for the mention, and the legacy-Selene removal must happen first.
4. Disable the native channel after the test unless a separately approved soak
   begins.
5. Only after that gate passes, discuss the full corrected matrix and rerun it
   under its own review point.
6. Buzz-side specification work does not wait on the gate. The accepted
   sequence for it is P1 host-side identity onboarding, P6 connected-agent
   channel-add readiness, P4 two-sided readiness, P3/RC5 OpenClaw durable-agent
   roster detection, and P2 two-stage Buzz-managed removal. Hermes stays
   unstarted until OpenClaw is proven stable, and upstream PR reconciliation
   follows that.

## 6. Accepted product decisions

Recorded with the user on 2026-07-29:

1. Identity generation is an explicit one-click host-side action with command
   instructions as fallback.
2. Every Buzz-created agent uses reversible archive first and separately gated
   permanent deletion second.
3. OpenClaw discovery selects the primary agent by default and exposes the full
   durable roster without enrolling it automatically.
4. DMs use the stable Buzz DM channel UUID and OpenClaw's native direct/session
   routing. Channel/thread validation may come first, but DMs are required for
   first-class parity.

## 7. Upstream coordination

Live GitHub state refreshed 2026-07-29:

| PR | Head | State |
|---|---|---|
| `block/buzz#2633` | `7b76647e7` | open |
| `block/buzz#2933` | `837958d5f` | open |
| `block/buzz#2942` | `46987a0fe` | open |

Do not rely on the older review of #2942 without re-reading its current head.
Do not open a competing PR until overlap and contract differences are
reassessed.

## 8. Safety invariants

- No resident nsec, private key, auth tag, or secret-bearing config enters this
  repository, a fixture, a log, or an owner-authored directory event.
- Buzz and a legacy/native adapter never share reply authority for one
  identity/channel pair.
- A connected record never enters managed-agent lifecycle or storage paths.
- SSH probing never accepts or persists a host key.
- No production connector cutover occurs without canary, soak, rollback, and
  per-identity approval.
- `/Users/dspury/Projects/buzz-upstream-merge` remains historical only; do not
  resume the retired `KeyCustody` design from it.

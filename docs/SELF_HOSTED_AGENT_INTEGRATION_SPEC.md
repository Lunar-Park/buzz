# Self-Hosted Agent Integration Specification

Status: accepted product baseline; implementation gaps remain. Updated
2026-07-29.

This is the canonical product specification for connecting host-owned resident
agents to Buzz. It covers the user-visible contract shared by Buzz Desktop and
resident adapters. Implementation sequencing belongs in the two linked plans:

- [Remote Agent Connect Implementation Plan](REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md)
- [Resident Agent Adapter Implementation Plan](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md)

## 1. Product outcome

A user can build a Buzz workspace using only agents they operate on their own
machines. Those agents retain their existing harness, memory, sessions, tools,
and lifecycle while behaving as normal first-class Buzz agents.

Buzz is the communication and collaboration surface. It is not the resident
agent's process supervisor or secret custodian.

## 2. Core model

### 2.1 Create versus connect

| | Create agent | Connect agent |
|---|---|---|
| Runtime owner | Buzz | Resident harness/host |
| Private-key custody | Buzz keyring | Resident host |
| Process lifecycle | Buzz starts and stops it | Host starts and stops itself |
| Buzz representation | Managed agent | Connected agent |
| Native Buzz behavior | Yes | Yes |

A connected agent is not a reduced-capability card. Once enrolled, it should
participate in profiles, mentions, channels, teams, permissions, and
conversations like any other Buzz agent. Only lifecycle controls differ.

### 2.2 Identity

- Every enrolled resident agent has one stable Buzz keypair and public profile.
- The private key remains on the resident host and is never transported to
  Buzz Desktop, stored in `connected-agents.json`, or placed in a relay event.
- Buzz stores the public key, display metadata, host reference, harness label,
  and timestamps required to present the connection.
- An existing host-owned identity may be reused when the user intentionally
  wants continuity. Otherwise, onboarding should offer to generate a fresh
  identity on the resident host.
- Host-side generation is an explicit, one-click action in the preferred flow.
  Buzz must show what host will be changed and require confirmation before
  invoking it. Instructions remain available as a fallback.
- Reconnecting must not silently rotate or overwrite an existing identity.
- Two independently visible Buzz agents must never share a signing key.

## 3. Onboarding flow

### 3.1 Host and harness discovery

1. Buzz lists configured SSH hosts without changing SSH trust state.
2. The user selects a host and requests a bounded, read-only probe.
3. Buzz reports reachable harnesses, versions, adapter readiness, and the
   presence/version of the Buzz CLI.
4. Unknown or changed SSH host keys fail with actionable messaging. Buzz never
   accepts a host key on the user's behalf.

Discovery is informative. It does not start a harness, install software, or
change resident configuration.

### 3.2 Agent identity resolution

For the selected harness agent, Buzz should:

1. attempt to retrieve the existing public identity from the host;
2. show the resolved npub/hex identity for confirmation;
3. if no identity exists, offer a confirmed `Generate identity on host` action;
4. if host-side automation is unavailable, provide the exact host command;
5. retain manual npub/hex entry as a fallback;
6. verify that the public identity matches the resident adapter's signing key
   before communication is enabled.

Automatic resolution or generation must never print, return, or persist the
private key on the laptop.

### 3.3 Selective enrollment

When a harness contains multiple durable named agents:

- the harness primary or `main` agent is selected by default;
- the selector exposes the full eligible roster from that harness instance;
- the user may keep the primary selection, choose another candidate, or add
  additional candidates;
- no flow automatically enrolls the full harness stack;
- the user may select one, several, or none;
- the user may rescan and enroll another agent later;
- every selected candidate receives or reuses its own stable Buzz identity;
- unselected candidates do not appear in Buzz.

Inside OpenClaw, a candidate may be called a subagent. In Buzz, an enrolled
candidate behaves as a normal agent and appears as a peer of Selene or any
Buzz-managed agent.

Ephemeral workers spawned for a single turn are not enrollment candidates.
Their work remains attributed to the parent agent unless the harness later
promotes them to durable named agents.

## 4. Native Buzz behavior

An enrolled connected agent must support:

- a normal agent card and profile;
- avatar and display-name hydration from its agent-authored profile;
- mentions and mention suggestions;
- channel membership through the existing bot-member path;
- inclusion in agent teams;
- adding an individual agent or team to a channel;
- independent per-channel/thread conversation state;
- normal Buzz permissions and visibility rules;
- local disconnect without stopping or deleting the resident agent.

Buzz must not render start, stop, restart, deploy, provider, PID, or local
key-management controls for a connected agent.

### 4.1 Buzz-managed agent removal

Any agent created and owned by Buzz uses a two-stage lifecycle:

1. `Remove from My Agents` is reversible. The confirmation lists linked
   managed instances, active/running state, and team references. After any
   required stop completes, Buzz hides the agent from normal cards, teams, and
   pickers but retains its identity, private key, definition, and local
   history in an `Archived agents` surface.
2. `Permanently delete identity` is available only from the archived state.
   It uses a second destructive confirmation and removes Buzz-held private-key
   material, managed instances, definition/configuration, and local team
   references. It is irreversible.

Permanent deletion cannot erase already-published signed events, relay audit
records, or copies retained by other clients. The UI must state that boundary
instead of promising deletion of historical network data.

Buzz starter agents use the same reversible removal path. Their bundled
templates remain available through `Restore Buzz starter agents`; restoring a
template after its former identity was permanently deleted creates a new
identity rather than resurrecting the erased key.

Connected agents are different: they retain `Disconnect` semantics because
Buzz does not own their key, definition, or runtime. Any true deletion of a
resident identity remains a host-side operation.

## 4.2 Connected agents must be addressable, not merely visible

A connected agent is only first-class when every gate between an owner's
keystroke and the agent's process agrees. Three separate facts were found to
diverge in practice, and all three are product requirements, not deployment
details:

1. **Community identity.** Buzz derives a community from the relay host, so a
   connected agent is meaningful only within the community whose relay the
   resident adapter actually listens to. Onboarding must record which community
   a connection belongs to and must refuse to present an agent as ready in a
   community its adapter cannot reach.
2. **Relay membership.** On a relay that requires membership, an agent's key is
   refused at authentication regardless of channel membership. Enrolling a
   connected agent must therefore either establish relay membership for its
   pubkey or supply owner attestation that satisfies the relay's delegation
   path. Adding an agent to a channel is not sufficient and must not be
   presented as if it were.
3. **Channel-add authority.** The relay enforces the target agent's
   `channel_add_policy` on any third-party add. An agent that publishes
   `owner_only` can be added only by a pubkey the relay has materialized as its
   owner. Buzz mints no owner attestation for a key it does not hold, so an
   owner currently cannot add its own connected agent to a channel once that
   agent declares `owner_only`.

The product requirement is that connecting an agent establishes owner evidence
for it — so `owner_only` remains the safe default and still permits the owner to
add the agent — and that readiness is reported per community. Weakening the
agent's policy to `anyone` is a workaround, not the design: it lets any
community member attach that agent to any channel.

## 5. Conversation routing

For every enrolled resident identity:

```text
Buzz pubkey
  -> resident adapter account
  -> exact harness agent ID
  -> OpenClaw conversation route
       channel: Buzz channel UUID + optional thread root
       direct:  Buzz DM channel UUID + optional thread root
```

- A mention reaches only the mapped harness agent.
- A reply is signed by that agent's Buzz key, never by a parent or sibling.
- Thread, retry, replay, and restart state cannot cross identities.
- Different Buzz channels resolve to different OpenClaw sessions.
- Different thread roots within a channel resolve to different thread
  sessions.
- A Buzz DM uses its stable DM channel UUID as the OpenClaw `direct` peer ID.
  The adapter passes that address to OpenClaw's normal router rather than
  maintaining a separate DM session system.
- OpenClaw's configured `session.dmScope` remains authoritative. Its native
  `main` behavior preserves one-owner continuity; multi-user deployments may
  choose `per-peer`, `per-channel-peer`, or
  `per-account-channel-peer` isolation. Readiness must warn when multiple
  permitted DM peers would share a `main` session.
- One accepted event produces at most one visible text reply or the specified
  durable silent acknowledgement.
- Buzz channel membership and adapter channel subscription are separate facts;
  onboarding must ensure both are configured before declaring an agent ready.

The current channel plugin already uses OpenClaw's agent and session router for
channel/thread lanes. DM parity requires Buzz NIP-17 ingress/decryption and a
`direct` peer mapping, but it does not require a second conversation
architecture.

## 6. Removal and lifecycle

Disconnecting a connected agent:

- removes only Buzz's local connection record;
- does not delete or rotate the resident key;
- does not stop the harness or delete the harness agent;
- does not disconnect sibling agents on the same host;
- does not publish a managed-agent tombstone;
- does not claim that the resident runtime was removed.

Changing or retiring the adapter's reply authority is an explicit harness-side
operation governed by the resident-adapter cutover plan.

## 7. Acceptance gates

### Gate A — documentation and identity safety

- product and implementation authority are unambiguous;
- no resident secret is present in Buzz files, logs, or relay metadata;
- SSH probing is bounded and never changes trust state.

### Gate B — single-agent onboarding

- the host and OpenClaw harness are discovered;
- the primary OpenClaw agent is selected by default and the full durable roster
  is available in the selector;
- Selene's public identity is retrieved automatically, explicitly generated on
  the host, or entered manually;
- Selene appears as a normal connected agent;
- Selene can be added individually and through a team to a channel.

### Gate C — single-agent communication

- the Desktop build and the resident adapter address the **same** community, and
  the agent's key authenticates against that relay;
- the owner can add the connected agent to the test channel without weakening
  the agent's own `channel_add_policy`;
- no second identity with reply authority for the same display name is a member
  of the test channel;
- the native adapter is enabled only for the intended identity and channel;
- one owner mention reaches Selene, carrying a real `p` tag rather than
  plain-text `@name`;
- one correctly signed reply appears in the expected thread;
- missing-mention, non-owner, and self-authored events do not trigger a reply;
- restart recovery produces no duplicate reply or model turn.

Gate C is the current blocking gate. UI connection alone does not satisfy it.

### Gate D — direct-message parity

- Buzz NIP-17 messages are decrypted and normalized before adapter policy;
- the stable Buzz DM channel UUID reaches OpenClaw as a `direct` peer;
- one DM produces one correctly signed reply in the same Buzz conversation;
- repeated DMs reuse or isolate the OpenClaw session according to the
  configured `session.dmScope`;
- a DM restart/replay produces no duplicate turn or reply;
- multi-user configurations cannot silently share private DM context.

### Gate E — selective multi-agent enrollment

- discovery preselects the primary OpenClaw agent and exposes the full durable
  roster without preselecting the full stack;
- selecting two agents creates two distinct Buzz identities and normal cards;
- each mention reaches only its mapped OpenClaw agent;
- independent threads and restart recovery do not cross identity or state;
- adding or disconnecting one agent leaves sibling connections intact.

Gates D and E may be implemented independently, but neither begins until Gate
C passes. Both are required before the resident integration is called
first-class.

### Gate F — cutover

- the corrected full Checkpoint C matrix passes;
- legacy and native adapters never share reply authority for one
  identity/channel pair;
- shadow/soak evidence is reviewed;
- rollback is demonstrated;
- production cutover is approved per identity.

## 8. Current implementation status

As of 2026-07-29:

- SSH host/harness discovery is implemented and manually worked against
  `lunar01`.
- A Selene connected-agent record can be created with a manually supplied
  public key.
- Connected agents are unified into the normal agent grid.
- Connected agents can be added to channels and agent teams.
- The current native OpenClaw adapter supports one top-level Buzz account.
- The corrected no-final-turn narrow canary passed and the channel was disabled
  afterward.
- No message from the current Buzz Desktop integration build has yet completed
  a round trip through Selene.
- Automatic public-key retrieval/generation, managed-agent archive/deletion,
  selective multi-agent enrollment, and direct messages remain open.
- Connected-agent addressability (§4.2) was the actual reason the first
  round-trip attempt could not run: the Desktop build and the adapter addressed
  two different communities, the agent's key was not a member of the Desktop
  community's relay, and its own `owner_only` policy blocked the owner from
  adding it to a channel.
- §4.2 item 3 (channel-add authority) and item 2 (relay membership) now have an
  implementation: Buzz can issue a NIP-OA owner attestation for a connected
  agent, which the host installs as its adapter auth tag. The relay materializes
  ownership from the AUTH event, which satisfies `owner_only` adds and admits the
  agent on a membership-required relay through owner delegation. Surfacing the
  attestation in the connect flow, and §4.2 item 1 per-community scoping, remain
  open.
- Connected records are community-scoped (§4.2 item 1), so an agent appears only
  in the community it was connected in; records predating the field stay visible
  everywhere rather than being silently migrated.
- Two-stage Buzz-managed removal (§4.1) has its storage and commands: archive is
  reversible and publishes nothing, permanent deletion refuses any agent that is
  not already archived. Its UI, and starter-template restoration, are open.
- Durable harness-agent roster detection is implemented behind a
  harness-neutral candidate shape, with the primary preselected and the full
  roster returned. Selection UI and per-agent identity minting remain open, and
  the current OpenClaw adapter still serves one account, so multi-agent
  *communication* remains WP5A.

Exact commits, installed paths, tests, and runtime state belong in the current
[session handoff](SESSION_HANDOFF_2026-07-29.md).

## 9. Deferred scope

The following are not required for the current single-Selene Gate C:

- media;
- typing and presence;
- proactive delivery;
- general reaction handling;
- edits;
- automatic production identity migration;
- Hermes plugin implementation;
- connector retirement.

Direct messages are a required Gate D parity item, not deferred product scope.
Channel/thread parity may be validated first, but the integration is not
complete or first-class until DM routing passes.

## 10. Accepted product decisions

Recorded with the user on 2026-07-29:

1. Identity generation is an explicit host-side one-click action, with manual
   instructions as fallback.
2. Every Buzz-created agent uses reversible removal/archive first and a
   separately gated permanent key/configuration deletion second.
3. OpenClaw discovery selects the primary agent by default and exposes the
   complete durable roster without enrolling it automatically.
4. DMs use Buzz's stable DM conversation identity and OpenClaw's native direct
   routing/session behavior; they do not need a separate session architecture.

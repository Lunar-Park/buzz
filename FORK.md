# Lunar Park Buzz Fork

This repository is the Lunar Park fork of
[block/buzz](https://github.com/block/buzz). Buzz is our self-hosted
communications hub: humans and resident agents participate as separate,
auditable Nostr identities in the same channels.

This document is the authority for why the fork exists and what may live in
it. Start with the
[fork document index](docs/LUNAR_PARK_FORK_INDEX.md) for the complete authority
order. User-visible connected-agent behavior belongs in the
[self-hosted agent integration specification](docs/SELF_HOSTED_AGENT_INTEGRATION_SPEC.md);
execution state belongs in the two implementation plans and the current
handoff.

## Product Scope

Buzz is a Discord replacement for Lunar Park. It is not the fleet-management
control plane. Fleet dashboards, NATS bridging, and using the Buzz event log as
the knowledge base are out of scope.

The fork should remain a patch queue over upstream, not a permanently divergent
application.

## Agent Integration Model

There are two distinct lanes:

1. Buzz-managed temporary agents use `buzz-acp`. Buzz starts the harness
   subprocess and owns that process/session lifecycle. OpenClaw and Hermes ACP
   presets have passed the separate BYOH Gate 3 relay path.
2. Harness-owned resident agents use native harness plugins. OpenClaw owns its
   Gateway, memory, sessions, and tools; Hermes owns its gateway, memory,
   sessions, and tools. Buzz supplies generic identity, listen, query, and send
   contracts through `buzz-cli`.

```text
                         Buzz relay
                             |
              Nostr auth, events, membership, history
                 +-----------+-----------+
                 |                       |
       Buzz-managed ACP lane     Resident-agent lane
                 |                       |
             buzz-acp             generic buzz CLI
                 |               listen / send / query
       openclaw acp, hermes-acp      |             |
                                  OpenClaw       Hermes
                                  plugin         plugin
```

OpenClaw- or Hermes-specific runtime branches do not belong in Buzz. Resident
private keys do not belong in the Buzz repository or a central replacement API.

## Legacy Connector

The Python `buzz-connector` and its OpenClaw webhook adapter proved the live
message, identity, and deployment path. They remain operational on lunar01 as
the production/rollback path.

They are no longer the target architecture:

- do not add new product features to the connector;
- do not centralize additional resident-agent signing keys there;
- do not give the connector and a native plugin reply authority for the same
  identity/channel pair;
- quarantine the old path only after native plugins pass canary, restart,
  cutover, soak, and rollback gates.

## Fork Policy

- Keep the delta small and observable.
- Prefer explicit configuration gates over deleting upstream behavior.
- Upstream generic capabilities in narrowly scoped PRs.
- Keep native OpenClaw and Hermes plugins in separate repositories.
- Keep Lunar Park hostnames, service definitions, identities, and secrets out
  of upstream PRs.
- Do not introduce new event kinds or HTTP endpoints solely for resident
  adapters when the generic CLI/Nostr surface is sufficient.
- Coordinate with overlapping upstream work before publishing competing PRs.

## Current State — 2026-07-29

The current `origin/main` fork is not yet the clean patch queue described
above. It contains a broad historical delta from upstream. Preserve it as
evidence and rollback material while rebuilding generalizable changes from a
refreshed `upstream/main` in focused worktrees.

Current adapter work:

- WP1-WP3 identity, listener, and fixture branches are clean, pushed, and
  remain narrow stacked units. Upstream has moved beyond their baseline, so
  publication requires a new rebase and coordination review.
- RC1 host-side key generation and RC2 safe profile publication are clean
  independent branches from the same earlier upstream baseline.
- RC3 remote harness discovery and RC4 the structurally separate Connected
  Agents Desktop surface are clean, pushed stacked branches. RC4 stores public
  metadata only in `connected-agents.json` and does not share managed-agent
  storage or lifecycle paths.
- The test-integration worktree
  `/Users/dspury/Projects/buzz-openclaw-integration` is clean. It combines
  WP1-WP3 and RC1-RC4 with two Desktop extensions: add connected agents to
  channels, and present connected agents as normal agents that can join teams.
  It is local-only and requires refresh before publication.
- OpenClaw compatibility spike: complete against lunar01 OpenClaw `2026.7.1`.
- Native OpenClaw plugin: implemented in the standalone
  `openclaw-channel-buzz` repository.
- OpenClaw Checkpoint C: the original matrix was blocked by a completed turn
  with no final text. The corrected narrow rerun passed on 2026-07-29 with one
  durable reaction and no restart duplicate. The full corrected matrix has not
  been rerun, so the native channel remains disabled and is not approved for
  connector cutover.
- Manual Desktop testing from the integration worktree has proved SSH harness
  discovery, manual public-identity connection, normal agent presentation,
  channel attachment, and connected-agent team membership for Selene. A live
  Buzz message has not yet completed a round trip through Selene.
- Explicit host-side identity resolution/generation, two-stage
  archive/permanent-delete behavior for Buzz-created agents, primary-first
  selective enrollment of durable OpenClaw agents, and native DM conversation
  parity are specified but not implemented.
- Hermes compatibility spike: complete against lunar02 Hermes `0.18.2`.
- Native Hermes plugin: not started.
- Production connector cutover: not started.

Do not describe the native OpenClaw plugin as production-ready or live. The
legacy webhook adapter is live; the native channel plugin is installed but
disabled.

## Fork Product Delta

The fork's existing product changes include:

| Change | Current state | Target disposition |
|---|---|---|
| Managed-agent autostart disabled by default | Implemented; `BUZZ_AGENT_AUTOSTART=1` re-enables | Generalize as an upstreamable gate |
| Built-in persona pack disabled by default | Implemented; `BUZZ_BUILTIN_PERSONAS=1` re-enables | Generalize as an upstreamable gate |
| Welcome team/channel/canvas/kickoff stripping | Implemented historically | Replace no-ops/deletions with explicit gates |
| TTS/STT automatic downloads disabled | Implemented historically | Replace stripping with an explicit gate |
| Builderlab authentication | Retained | Keep unless product scope changes |
| `buzz-acp` and managed agents | Retained | Keep as the separate ACP lane |

The implementation must be inspected before claiming any historical strip has
been converted into a clean gate.

## Deployment Snapshot

- lunar01: production Buzz relay, legacy Python connector, legacy OpenClaw
  webhook adapter, and OpenClaw Gateway.
- lunar02: Buzz development stack and Hermes host.
- Laptop: primary fork development and desktop build environment.

Operational details and mutable host state belong in the Lunar Park KB and
adapter checkpoint docs, not in upstream PR descriptions.

## Documentation Authority

The canonical order and update rules are defined in
[`docs/LUNAR_PARK_FORK_INDEX.md`](docs/LUNAR_PARK_FORK_INDEX.md). This file
owns policy only. It must not become a second execution log.

## Upstream Sync

- `origin` points to `Lunar-Park/buzz`.
- `upstream` points to `block/buzz`.
- Start each upstreamable change from refreshed `upstream/main`.
- Keep identity, listener, fixtures, and optional directory publication as
  independently reviewable units.
- Run the full Buzz quality gate before any PR and sign commits for DCO.

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

## Current State — 2026-07-31

The fork now uses a clean-base plus integration-branch model:

- `main` is an exact mirror of current `upstream/main`;
- `lunar/integration` is the complete intentional Lunar Park fork delta replayed
  on that clean base;
- `pr/<topic>` branches are the only branches that should become draft upstream
  PRs after review.

The pre-cleanup state is preserved under `archive/*` refs. Use
`git diff upstream/main...lunar/integration` to inspect the full fork delta.
Do not use `main` as the fork patch queue.

Gate layer (every strip is now an explicit, tested gate — no deletions):

| Gate | Default | Re-enable |
|---|---|---|
| Managed-agent autostart | off | `BUZZ_AGENT_AUTOSTART=1` |
| Built-in persona pack merge | off | `BUZZ_BUILTIN_PERSONAS=1` |
| Welcome experience (channel/team/canvas/kickoff) | off | `VITE_BUZZ_WELCOME_EXPERIENCE=1` (build-time) |
| Voice-model fetch at boot | off | `BUZZ_VOICE_MODEL_AUTODOWNLOAD=1` |
| Agent respond-to default | `nobody` (silent) | per-agent setting |

`Restore Buzz starter agents` (new-agent menu) re-adds the bundled pack on
demand regardless of the persona gate.

Product stack on `lunar/integration`: WP1–WP3 external-agent CLI contract, RC1
keys, RC2 profile-safe writes, RC3 SSH discovery, RC4 connected agents,
channel/team unification, RC5 roster detection, P6 owner attestation, P1
host-side identity onboarding, community scoping, P2 two-stage removal (fully
wired, including the card menu and starter-template restore), RC6 Buzz-side DM
ingress (`buzz listen --dms`), and the current channel/DM agent behavior
controls proven in live Selene testing.

Current status:

- Gate C canary retired 2026-07-30: OpenClaw-in-Buzz was proven live (roster
  picker end-to-end, connection, channel add, team membership). Subsequent live
  testing confirmed Selene channel replies, inline reply mode, DM no-mention
  wakeup, and DM inline replies.
- Upstream PR preparation is in progress. The legacy connector remains the
  production/rollback path until the native path has enough practice soak and
  restart/replay evidence.
- Native Hermes plugin: not started. Production connector cutover: not started.
- Upstream PR candidates are prepared, not opened: `pr/keys-generate`,
  `pr/profile-clobber-fix`. WP1/WP2 are superseded by open upstream PRs
  #2933/#2942 — coordinate, do not compete.

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
- Keep `origin/main` equal to `upstream/main`.
- Keep the full Lunar Park delta on `lunar/integration`.
- Start each upstreamable change from refreshed `origin/main` / `upstream/main`.
- Keep identity, listener, fixtures, and optional directory publication as
  independently reviewable units.
- Run the full Buzz quality gate before any PR and sign commits for DCO.

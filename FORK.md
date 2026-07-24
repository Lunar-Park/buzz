# Lunar Park Fork — Spec

This is `Lunar-Park/buzz`, a fork of [block/buzz](https://github.com/block/buzz).
This document is the fork's contract: why it exists, what it changes, what it
deliberately doesn't, and how it stays close to upstream. If a change doesn't
fit this document, it probably belongs in the connector repo, not here.

## Purpose

Buzz is our self-hosted **Discord replacement**: a comms hub where humans and
an existing fleet of agents — running on different machines, under different
harnesses (Hermes, OpenClaw, occasionally pi) — participate as equals.

It is **not** a fleet-management platform. Fleet monitoring, dashboards, NATS
bridging, and KB search were considered and shelved (2026-07-24).

## Why fork at all

Upstream's agent model (`buzz-acp`) spawns ACP harnesses as **local
subprocesses** and owns their lifecycle — spawn at GUI startup, prompt queue,
respawn on death. That's "bring your own agent binary," not "let my existing
agents join." Our agents live out-of-process on other machines with their own
lifecycles. They connect as **network peers**: Nostr keypairs speaking NIP-01/
NIP-42 through a central connector (separate repo, `buzz-connector`).

The fork exists only to make vanilla Buzz livable for that deployment:
no auto-firing embedded agents, no seeded welcome team, no forced model
downloads. Everything else is upstream, unmodified.

## Fork policy — a patch queue, not a divergence

- **Thin.** The delta versus upstream stays as small as possible. Every patch
  is a liability against upstream's churn (onboarding and agent lifecycle are
  their fastest-moving files).
- **Config gates over deletions.** Behavior we don't want gets a setting, not
  a deletion. Gated code rebases cleanly; deleted code conflicts forever.
- **Additive over invasive.** New behavior goes in new files/routes where
  possible.
- **Upstream what generalizes.** Most of our gates are generic self-hoster
  features ("bring your own agents" mode, disable welcome seeding,
  `RespondTo::Nobody` default). Every patch accepted upstream is a patch we
  stop maintaining. Issue first, then PR.
- **Features live in the connector.** If it can be built against the relay's
  public surface (WebSocket + HTTP), it goes in `buzz-connector`, not here.
- **Escape hatch check:** if we ever need to change relay protocol semantics
  or core desktop UX, stop and reconsider — that's hard-fork territory and a
  deliberate decision, not a drive-by patch.

## Current delta vs upstream

| Patch | Kind | Status |
|---|---|---|
| `RespondTo::Nobody` variant, new default (agents don't auto-fire) | Rust | landed |
| `seedWelcomeExperience()` no-op (no Fizz/Honey/Bumble) | TS | landed |
| TTS/STT model auto-downloads disabled at startup | Rust | landed |
| `ensureWelcomeChannel()` disabled | TS | landed |
| `useWelcomeKickoff()` gutted | TS | working tree |
| Gate `agent-event-reconcile` boot provisioning | Rust | **TODO** — last embedded-agent leak: backend provisions agents on boot regardless of frontend |
| Convert the no-op strips above into config gates | both | TODO — precondition for upstreaming |

**Kept from upstream, deliberately:**

- **Builderlab auth** — cross-device login + key backup. Accepted dependency
  on Block's service (comms and data stay ours if it dies; keep local key
  backups).
- **buzz-acp / native Claude Code + Codex agents** — bonus lane for agents
  that are fine living on the host machine under Buzz's lifecycle.
- Relay, protocol, desktop UX, mobile — untouched.

## Deployment model

- One always-on host (TBD — addressed by tailnet name, never hard-coded) runs
  the relay stack (relay, Postgres, Redis, MinIO) and the connector.
- Humans use the desktop app (built from this fork) as a pure client from any
  machine; Builderlab login gives Discord-style cross-device sign-in.
- Agents use the connector's HTTP API over the tailnet. Keypairs exist only on
  the connector host. Resident agents (Hermes ×4 devices, OpenClaw) receive
  **webhook push** with retry/park and catch-up-on-reconnect; session agents
  (pi, Claude Code) pull via an MCP wrapper. Polling is not a delivery
  mechanism.

## Related

- `buzz-connector` — the agent gateway (separate repo; not part of this fork)
- Lunar Park KB: `projects/buzz/` — plan, connector spec, relay reference docs
  (event kinds, message flow, HTTP surface, NIP-42, channel membership)

## Upstream sync

- `origin` → `Lunar-Park/buzz`, `upstream` → `block/buzz`
- Sync `main` from upstream regularly (weekly-ish while development is hot);
  rebase the patch queue; this table above is the checklist of what must
  survive the rebase.

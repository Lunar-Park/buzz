# Lunar Park Buzz Fork Document Index

Status: canonical document-control index. Updated 2026-07-29.

This index defines where Lunar Park fork decisions, specifications, execution
state, evidence, and session handoffs belong. It exists to prevent a dated
handoff, historical report, or operational note from silently becoming product
or implementation authority.

These files are private fork-planning material. Do not include them in an
upstream `block/buzz` pull request.

## Authority order

When documents disagree, use this order:

1. [`FORK.md`](../FORK.md) — fork purpose, scope, and upstreamability policy.
2. [`SELF_HOSTED_AGENT_INTEGRATION_SPEC.md`](SELF_HOSTED_AGENT_INTEGRATION_SPEC.md)
   — canonical product behavior and acceptance gates for connected resident
   agents.
3. [`REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md`](REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md)
   — Buzz-side identity, SSH discovery, storage, and Desktop implementation.
4. [`RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md`](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md)
   — harness-side OpenClaw/Hermes transport, durability, canary, and cutover
   execution.
5. [`RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md`](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md)
   — stable repository and package ownership boundaries.
6. The standalone adapter repository's `README.md`, `ARCHITECTURE.md`, and
   `docs/CHECKPOINT_C.md` — plugin implementation and live-canary evidence.
7. [`SESSION_HANDOFF_2026-07-29.md`](SESSION_HANDOFF_2026-07-29.md) — current
   working snapshot and exact resumption point.
8. Lunar Park KB `projects/buzz/` — operational index and historical connector
   record, not implementation authority.

Code and live state override every document. When a current-status claim is
cheap to verify, verify it before acting.

## Document classes

| Document | Class | Maintained content | Must not contain |
|---|---|---|---|
| `FORK.md` | policy | why the fork exists, permanent boundaries, high-level delta | detailed test logs or live secrets |
| `SELF_HOSTED_AGENT_INTEGRATION_SPEC.md` | product specification | user-visible behavior, identity semantics, acceptance gates, deferred scope | branch hashes or command transcripts |
| `REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md` | Buzz execution plan | RC work packages, Buzz files, branch state, validation | OpenClaw implementation internals |
| `RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md` | adapter execution plan | WP work packages, plugin state, canary/cutover gates | duplicated product requirements |
| `RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md` | architecture reference | stable package/repository ownership | volatile status or next-session instructions |
| `AGENT_HARNESS_BYOH_ACCEPTANCE.md` | evidence record | Buzz-managed ACP lane evidence | resident-plugin readiness claims |
| `AGENT_HARNESS_ADAPTER_ALIGNMENT_REPORT.md` | historical assessment | the 2026-07-26 architecture review | current execution authority |
| `SESSION_HANDOFF_2026-07-29.md` | session snapshot | verified heads, runtime state, test observations, next action | new architecture decisions |

## Update rules

1. Record a product decision once in the product specification. Implementation
   plans may link to it but should not restate it in competing language.
2. Record Buzz implementation state only in the Remote Agent Connect plan.
3. Record OpenClaw/Hermes implementation and canary state only in the Resident
   Agent Adapter plan and the adapter repository's checkpoint evidence.
4. Keep exact branch heads, test counts, installed paths, and enabled/disabled
   runtime state in the dated handoff. Plans may carry a dated summary, but the
   handoff is the resumption checklist.
5. Keep `AGENTS.md` procedural. It points here and must not duplicate a dated
   stop state.
6. Never copy a resident private key, nsec, auth tag, or secret-bearing config
   into any Buzz document.
7. A session that changes code, live configuration, branch topology, or an
   acceptance gate must update the handoff before stopping.
8. A new handoff supersedes the prior handoff. Preserve older handoffs only as
   history; do not keep two files claiming to be current.

## Terminology

| Term | Meaning |
|---|---|
| Buzz-managed agent | Buzz owns the local key and starts/supervises the ACP process |
| Connected agent | Buzz stores only a public pointer to an already-running, host-owned agent |
| Resident agent | A durable agent whose harness, memory, sessions, tools, and lifecycle remain on its host |
| Harness agent | A durable named agent configured inside OpenClaw, Hermes, or another resident harness |
| Ephemeral worker | A temporary child spawned for a task; not independently enrolled in Buzz |
| Enrollment | Selectively exposing one durable harness agent as a normal connected Buzz agent |

An OpenClaw subagent can therefore be a normal first-class Buzz agent after
explicit enrollment. The OpenClaw relationship does not appear in Buzz's
user-facing lifecycle model.

## Current review boundary

The current product specification is ready for user review. No further live
OpenClaw changes, branch merges, upstream publication, or subagent work should
begin until:

1. the product specification is reviewed and amended as needed;
2. the handoff accurately reflects the accepted specification;
3. the documentation snapshot is committed on the planning branch.

# Lunar Park Buzz Fork Document Index

Status: canonical document-control index. Updated 2026-07-31.

This index defines where Lunar Park fork decisions, specifications, execution
state, evidence, branch topology, and session handoffs belong. It exists to
prevent a dated handoff, historical report, operational note, or temporary
branch from silently becoming product or implementation authority.

These files are private fork-planning material. Do not include them in an
upstream `block/buzz` pull request.

## Branch model

The fork uses three durable branch classes:

| Branch class | Example | Purpose | Upstream PR policy |
|---|---|---|---|
| Clean base | `main` | Exact mirror of current `upstream/main`; safe reset/sync target | Never carries Lunar Park work |
| Fork integration | `lunar/integration` | Complete intentional Lunar Park fork delta rebased/replayed on the clean base | Review inventory source only; do not PR wholesale |
| Focused PR branch | `pr/<topic>` | One upstreamable logical change extracted from `lunar/integration` or preserved refs | Candidate for draft PR to `block/buzz` after review |

`main` must stay boring: after each upstream refresh, `origin/main` should match
`upstream/main` exactly. The command below should print `0 0`:

```bash
git fetch origin --prune
git fetch upstream main --prune
git rev-list --left-right --count upstream/main...origin/main
```

`lunar/integration` is the branch to diff when asking "what is our fork?"

```bash
git diff upstream/main...lunar/integration
git log --oneline upstream/main..lunar/integration
```

Focused upstream PR branches must start from the clean base, not from a dirty
working tree:

```bash
git switch -c pr/<topic> origin/main
git cherry-pick <reviewed-commit-or-range>
```

Fork-only runtime material, private deployment configuration, dated handoffs,
and Lunar Park planning docs stay on `lunar/integration` or archives; they must
not be included in upstream PR branches.

Use [`UPSTREAM_PR_PLAYBOOK.md`](UPSTREAM_PR_PLAYBOOK.md) for the focused-branch
checklist, scope gate, validation gate, and reusable PR wording.

## Authority order

When documents disagree, use this order:

1. Code and live state — verify directly when cheap.
2. [`FORK.md`](../FORK.md) — fork purpose, scope, and upstreamability policy.
3. [`SELF_HOSTED_AGENT_INTEGRATION_SPEC.md`](SELF_HOSTED_AGENT_INTEGRATION_SPEC.md)
   — canonical product behavior and acceptance gates for connected resident
   agents.
4. This file — document authority, branch topology, and cleanup rules.
5. [`REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md`](REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md)
   — Buzz-side identity, SSH discovery, storage, and Desktop implementation.
6. [`RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md`](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md)
   — harness-side OpenClaw/Hermes transport, durability, canary, and cutover
   execution.
7. [`RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md`](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md)
   — stable repository and package ownership boundaries.
8. The standalone adapter repository's `README.md`, `ARCHITECTURE.md`, and
   `docs/CHECKPOINT_C.md` — plugin implementation and live-canary evidence.
9. [`SESSION_HANDOFF_2026-07-31.md`](SESSION_HANDOFF_2026-07-31.md) — current
   working snapshot and exact resumption point.
10. Lunar Park KB `projects/buzz/` — operational index and historical connector
    record, not implementation authority.

## Document classes

| Document | Class | Maintained content | Must not contain |
|---|---|---|---|
| `FORK.md` | policy | why the fork exists, permanent boundaries, high-level delta | detailed test logs or live secrets |
| `SELF_HOSTED_AGENT_INTEGRATION_SPEC.md` | product specification | user-visible behavior, identity semantics, acceptance gates, deferred scope | branch hashes or command transcripts |
| `LUNAR_PARK_FORK_INDEX.md` | control index | authority order, branch model, cleanup rules | volatile runtime claims |
| `REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md` | Buzz execution plan | RC work packages, Buzz files, branch state, validation | OpenClaw implementation internals |
| `RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md` | adapter execution plan | WP work packages, plugin state, canary/cutover gates | duplicated product requirements |
| `RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_MAP.md` | architecture reference | stable package/repository ownership | volatile status or next-session instructions |
| `UPSTREAM_PR_PLAYBOOK.md` | PR preparation playbook | focused-branch checklist, scope gate, validation commands, reusable PR wording | product requirements or live runtime state |
| `AGENT_HARNESS_BYOH_ACCEPTANCE.md` | evidence record | Buzz-managed ACP lane evidence | resident-plugin readiness claims |
| `AGENT_HARNESS_ADAPTER_ALIGNMENT_REPORT.md` | historical assessment | the 2026-07-26 architecture review | current execution authority |
| `SESSION_HANDOFF_*.md` | session snapshot | verified heads, runtime state, test observations, next action | new architecture decisions |

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
9. If `origin/main` diverges from `upstream/main`, archive the divergent state
   before restoring `main` to the clean base.
10. If `lunar/integration` is rebuilt, push an archive of the previous head
    before force-updating it.

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

## Current boundary

Current work is fork consolidation and PR preparation after successful live
Selene channel and DM round trips. Before upstream publication:

1. keep `origin/main` identical to `upstream/main`;
2. keep the complete fork delta on `lunar/integration`;
3. extract one upstreamable topic at a time into `pr/<topic>` branches;
4. review each focused branch before opening a draft PR against `block/buzz`;
5. keep Lunar Park runtime/deployment material and private planning docs out of
   upstream PR branches.

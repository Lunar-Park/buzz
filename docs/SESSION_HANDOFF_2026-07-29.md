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

- The native OpenClaw Buzz channel on lunar01 is loaded but disabled.
- The legacy Python connector and webhook adapter remain the production and
  rollback path.
- No native/legacy reply-authority cutover has occurred.
- The corrected narrow no-final canary passed; the full corrected Checkpoint C
  matrix has not been rerun.
- The current Desktop build has connected Selene as metadata but has not
  completed a message round trip through OpenClaw.
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

At handoff time the process is still running as:

```text
Buzz Dev (openclaw-test-integration-2026-07-29)
Vite port 37299
bundle ID xyz.block.buzz.app.dev.lunar-park-openclaw-test-integration-2026-07-29
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
   the native OpenClaw plugin and back as Selene.

Persisted public-only local state:

```text
~/Library/Application Support/
  xyz.block.buzz.app.dev.lunar-park-openclaw-test-integration-2026-07-29/
    agents/connected-agents.json
    agents/teams.json
```

No resident private key is stored there.

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

## 5. Next-session resumption sequence

1. Use the accepted decisions in
   `SELF_HOSTED_AGENT_INTEGRATION_SPEC.md` as the product baseline.
2. Resolve the exact UUID of the Buzz channel used for the Selene test.
3. Compare that channel, the isolated canary pubkey, WP5 `channelIds`, and the
   loaded signing key before enabling anything.
4. Present the exact enable/test/disable commands and rollback boundary.
5. Run one owner mention, verify one signed threaded Selene reply, restart the
   Gateway, and verify no duplicate.
6. Disable the native channel after the test unless a separately approved soak
   begins.
7. Only after that gate passes, discuss the full corrected matrix and
   independently sequence RC5 roster enrollment and RC6 DM parity.

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

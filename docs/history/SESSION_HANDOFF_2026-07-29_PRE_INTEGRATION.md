# Historical Session Handoff — 2026-07-29 Pre-Integration Review

Status: historical evidence only. Superseded by
[`../SESSION_HANDOFF_2026-07-29.md`](../SESSION_HANDOFF_2026-07-29.md).

This snapshot records the RC3/RC4 and OpenClaw retry-policy review before the
integrated Desktop onboarding work and corrected narrow canary rerun. Its
current-state and next-step sections are intentionally preserved as historical
context and must not be used to resume work.

Purpose: hand this session's work to (a) a reviewing agent and (b) a manual
testing pass. Written to be read without the prior conversation.

Scope of this session: act on a code review that returned **needs changes** with
four correctness defects, then publish. No feature work was added beyond the
fixes and one UI defect found while writing this handoff.

---

## 1. Current state

### Lane A — Buzz remote connect (published to `Lunar-Park/buzz`)

| Branch | Head | Contents |
|---|---|---|
| `buzz-pr-identity` | `e14a94ec3` | WP1 — 2 commits |
| `buzz-pr-listen` | `1d65784d5` | WP2 — 4 commits (includes WP1) |
| `buzz-pr-fixtures` | `68513902c` | WP3 — 6 commits (includes WP1–WP2) |
| `buzz-rc-keys` | `26bb74aa2` | RC1 — 1 commit |
| `buzz-rc-profile` | `ce69cc3fe` | RC2 — 1 commit |
| `buzz-rc-discovery` | `ad1c5a334` | RC3 — 2 commits |
| `buzz-rc-custody` | `098711146` | RC4 — 3 commits (RC3 + 1) |

All seven are based on `upstream/main` `22be8bb35`, every commit has a
`Signed-off-by`, all trees clean, all pushed and verified matching `origin`.
**No PRs opened** anywhere — this was explicit.

RC4 is stacked on RC3: `buzz-rc-custody` = `fde50bf7b` + `ad1c5a334` +
`098711146`, and `ad1c5a334` is `buzz-rc-discovery`'s tip. RC3 must be reviewed
and published before RC4.

### Lane B — native OpenClaw plugin

Repo: `/Users/dspury/Lunar-Park/integrations/openclaw-channel-buzz` (branch
`main`).

```
aebd8dd  chore(openclaw): re-pin the vendored fixture source commit
d05a2c0  fix(openclaw): never dead-letter a failure that was classified retryable
df1bdf9  feat(openclaw): validate the silent-ack emoji and cover the seams it runs through
b87b72c  fix(openclaw): bound per-event retries with a dead-letter outcome   ← previous head
```

Working tree clean except `AGENTS.md` and `CLAUDE.md`, which are intentionally
untracked and must stay that way.

**Lane B is not pushed: the repo has no git remote configured.** `git remote -v`
is empty. The three commits are local-only. A remote must be added before any
of this is published or deployed.

Package after the changes:
`lunar-park-openclaw-channel-buzz-0.1.0.tgz`, shasum
`6afb3d1c1e4188e6fecfb9ceb210c8cd0c34b6f0`, 20112 bytes, 21 files.
(That checksum predates commit `aebd8dd`, which only changed
`UPSTREAM_FIXTURE_COMMIT`, `README.md`, and `docs/CHECKPOINT_C.md` — re-run
`npm pack --dry-run --json` if an exact current value is needed.)

### Main checkout

`/Users/dspury/Projects/buzz` is the mixed planning worktree on
`feature/agent-harness-adapter-alignment` at `95fdf9788`. It is intentionally
dirty with planning material only (this file, `FORK.md`, the four `docs/*` plans,
two `scripts/*` acceptance scripts, and modified `AGENTS.md` / `ARCHITECTURE.md` /
`README.md` / `Justfile` / `.github/workflows/ci.yml`). **None of it may enter an
upstream PR.** No implementation work happens here.

---

## 2. What changed this session

### Review defect 1 — the SSH probe accepted unknown host keys (Lane A, RC3)

`desktop/src-tauri/src/managed_agents/remote_probe.rs`

`probe_ssh_host` passed `StrictHostKeyChecking=accept-new`, while its own comment
claimed unknown keys were "a reportable status, not something to accept on the
user's behalf". `accept-new` rejects *changed* keys but silently accepts *and
persists* first-seen ones, so clicking Probe in a dialog wrote to the user's
`~/.ssh/known_hosts`.

Now `StrictHostKeyChecking=yes`. Supporting changes:

- The option list was extracted into `ssh_probe_args(&SshHost) -> Vec<String>`
  so it is assertable. Nothing else in the module reads or writes trust state, so
  this flag *is* the enforcement.
- `classify_ssh_failure` also matches `"you have requested strict checking"` —
  the line ssh emits for a first-seen host, which never appeared under
  `accept-new`.
- `failure_message` now distinguishes an untrusted key from a changed one. A
  changed key is the warning ssh exists to give; rendering it identically to
  routine first-run setup trains the user to dismiss it.

### Review defect 2 — Lane B dead-lettered retryable failures

`src/runtime.ts` `processPendingRecord`

Dead-lettering completes the journal record **and advances the cursor**, so it
destroys the event. The condition was `!isFatalCode(code) && attempts >= MAX`,
with no check of `retryable`. A relay outage, `STATE_IO_FAILED` disk hiccup,
unreachable harness, or `OUTBOUND_UNCERTAIN` send awaiting reconciliation was
therefore discarded after five attempts, and an outage spanning a queued burst
discarded the whole burst.

Both `MAX_EVENT_ATTEMPTS`' own docstring ("bounds any deterministic per-event
failure") and `CompletionMetadata.outcome` ("after `MAX_EVENT_ATTEMPTS`
deterministic failures") already specified deterministic-only. Only the condition
disagreed, so this **restores the documented contract** rather than changing it.

Now gated on `!classified.retryable`. The cap still does its original job: a
deterministic failure is exactly the non-retryable case, and `run()` (line ~155)
escalates when `!retryable`, so a poisonous event still costs at most five
attempts. A retryable failure takes the other branch of that same condition,
where `run()` backs off up to 30s and `recoverPending` replays the record — so it
retries indefinitely and stays visible in `AccountHealth` instead of being
silently swallowed. Both docstrings were amended to state the narrowing.

### Review defect 3 — SSH stanza boundaries were inferred from field values

`desktop/src-tauri/src/managed_agents/ssh_config.rs`

`last_host_line_group()` reconstructed which entries a keyword applied to by
scanning back over entries sharing the tail's current field values. Two real
consequences:

- `Host alpha` / `Host beta` / `Port 2222` — both records were still blank, so
  both matched and both got the port. `probe_ssh_host` forwards it as `-p`, so
  **alpha was probed on beta's port**.
- `Host alpha` / `Port 22` / `Host *` / `Port 2222` — a pattern-only stanza
  contributes no entry, so its keywords landed on the *previous real host*,
  letting a defaults block silently rewrite an explicit port. This one was found
  while fixing the reported case; it was not in the review.

The helper is deleted. `parse_ssh_config_str` now tracks
`current_stanza: Option<Range<usize>>` — the exact entry range each `Host` line
created. `None` before the first `Host` line (global defaults, still ignored);
an empty range for a pattern-only stanza, so its keywords apply to nothing.

### Review defect 4 — a truncated probe reported success

`desktop/src-tauri/src/managed_agents/remote_probe.rs`

`run_probe` required only `PROBE_START`, so a session dropped mid-probe returned
`ok: true` with whatever harnesses had been enumerated before the connection
died. The important part is not "partial data": `parse_probe_output` cannot
distinguish "we never got to look" from "that harness is not installed", so the
two rendered as the same answer and the harness list was short for no visible
reason.

Both markers are now required, with a new `HostProbeErrorKind::Truncated`
(`"truncated"` on the wire, added to the TS union in
`desktop/src/shared/api/remoteAgentTypes.ts`).

### Review item 5 — stale planning hashes

`docs/REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md` and
`docs/RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md` updated to final SHAs, with
a dated entry recording all four defects and their fixes. No `02caeeeb1` or
`e0ecbbd3a` references remain anywhere.

### Additional fixes not in the review

1. **RC4 squashed.** `e0ecbbd3a` (a one-line trailing-blank-line removal) was
   folded into the feature commit. Verified the line originated in RC4, not RC3.

2. **`buzz-rc-discovery` was pointing at the pre-fix RC3.** The SSH fixes landed
   in RC3's commits inside the custody worktree, so the discovery branch would
   have published the insecure probe on its own. Reset to `ad1c5a334`; the delta
   between old and new tips is exactly the four SSH-fix files.

3. **`remote_probe.rs` test module extracted** to `remote_probe_tests.rs` via
   `#[path]`. The file was deliberately parked at 996 lines against a hard 1000
   limit and the new tests pushed it to 1222. The ratchet was **not** modified.

4. **Stale doc comment in RC4.** `ConnectedAgent`'s doc still said connected
   agents are "filtered out of `listManagedAgents()`" — that describes the
   abandoned `KeyCustody` design and would send a reader hunting for a filter
   that does not exist.

5. **UI mislabelled every probe failure** (found while writing this handoff).
   `ConnectedAgentsSection.tsx` rendered "machine unreachable" for everything
   except `password_required`. The host answered in every case but
   `unreachable`, and labelling an untrusted host key that way sends the user to
   check the network when the fix is to review a fingerprint — directly
   undermining defect 1's fix, since Buzz no longer accepts keys itself and this
   label is the only prompt. Moved to `reachabilityLabel()` in
   `connectAgentIntent.ts` (following the existing helper + `.test.mjs`
   convention) with a case per kind and `"probe failed"` for unclassified.

6. **Fixture pin re-pinned** `c723de94b` → `68513902c`. Verified
   content-neutral before moving: identical file sets, and all 16 vendored files
   byte-identical both to the new commit and between the two commits. Also
   documented that the SHA is only resolvable on the fork's `buzz-pr-fixtures`
   branch — in a `block/buzz` clone it fails with "bad object", because the
   fixture contract is not upstreamed.

---

## 3. Verification performed

Every new regression test was **falsified**: the production fix was temporarily
reverted and the test confirmed to fail, then restored. A test that passes
against the broken code is not a regression guard. Results:

| Fix | Guard tests | Falsified? |
|---|---|---|
| Host-key strictness | `the_probe_never_accepts_a_host_key_on_the_users_behalf` | yes — reverted flag to `accept-new` |
| Truncated probe | `a_probe_cut_off_after_the_start_marker_is_not_reported_as_success` | yes — removed the `PROBE_END` block |
| Stanza boundaries | 3 of 4 new tests | yes — spliced new tests into the pre-fix parser |
| Lane B dead-letter | all 3 new tests | yes — removed the `!retryable` clause |

Two are deliberate **controls**, not guards, and pass either way — say so if
reviewing them: `a_complete_probe_with_the_same_facts_does_succeed` (so the
`PROBE_END` requirement can't pass by rejecting everything) and
`an_earlier_stanzas_values_are_not_overwritten_by_a_later_one`.

Gate results:

| | RC4 (`098711146`) | RC3 (`ad1c5a334`) | CLI (`68513902c`) | Lane B |
|---|---|---|---|---|
| Rust tests | 1918 pass / 14 ign | 1895 pass / 14 ign | 275 pass / 1 ign† | — |
| Desktop JS | 3784 pass | 3769 pass | — | 22 pass |
| `cargo fmt --check` | clean | clean | — | — |
| `cargo clippy -D warnings` | clean | clean | — | — |
| `tsc --noEmit` | exit 0 | exit 0 | — | typecheck pass |
| `pnpm check` (biome + 3 guards) | exit 0 | exit 0 | — | — |
| size ratchet vs `upstream/main` | exit 0 | exit 0 | — | — |
| `git diff --check` | clean | clean | — | clean |
| `npm audit --omit=dev` | — | — | — | 0 vulns |
| `npm pack --dry-run` | — | — | — | pass |

† the 1 ignored is a `run_from_args` doctest, not a live-relay test.

**RC3 was gated independently** (checked out on its own) specifically so that
publishing it first does not ship a defective probe.

Note: `pnpm check` reports two pre-existing `lint/style/useTemplate` **infos** in
`src/features/agents/lib/personaCatalogRelay.test.mjs`. Upstream debt, last
touched by upstream `a35771fc4`, untouched by any of these branches. Infos do not
fail the run.

---

## 4. For the reviewing agent — judgment calls to scrutinise

These are choices, not facts. A reviewer should second-guess them.

1. **The SSH fixes were folded into RC3's existing commits, rewriting history**,
   rather than added as a follow-up fix commit. Rationale: RC3 is published
   first, so it must be correct standing alone; a fix commit after RC4 would not
   protect it. Cost: `a78843c23` → `fde50bf7b` and `e1805f726` → `ad1c5a334`, so
   any external reference to the old SHAs is stale, and the three previously
   published branches took a `--force-with-lease` push.

2. **A truncated probe returns `ok: false` and withholds all facts** rather than
   showing partial facts with a warning. Rationale: the facts are an unknown
   fraction and a short harness list is indistinguishable from a correct one.
   Reasonable people could prefer degraded-but-visible.

3. **Strict host-key checking is a behavioural regression for first-run users.**
   Probing a host not yet in `known_hosts` now fails where it previously
   succeeded (while silently writing the key). This is the intended posture, but
   it means the error path is now a *common* path, not an edge case — worth
   confirming the wording is good enough to act on. See test item 3 below.

4. **`host` is `String`, not `Option<String>`, on `ConnectedAgentRecord`** (from
   the prior session, still load-bearing). It is what makes the two record types
   mutually unparseable, which is asserted by two tests.

5. **The silent-turn semantic was treated as accepted.** Lane B's next action was
   "explicitly accept or revise: a silent completed turn posts the configured eye
   reaction rather than producing no visible receipt." The user said "go ahead"
   on work gated on it, so `df1bdf9` records it as accepted and argues the emoji
   is therefore load-bearing config worth validating strictly. **If that reading
   is wrong, `df1bdf9` needs amending.**

6. **Lane B commits are unpushed and unreviewed by CI** — no remote exists, so
   nothing has run them outside this machine.

---

## 5. Getting to a testable state

### 5.1 Never run — do this first

`just ci` has **not** been run end to end on any branch this session, or in the
prior one. Verified so far: the desktop Rust crate, the desktop JS suite, and
`buzz-cli`. **Unverified: the relay crates, the web client, and mobile
(Flutter).** RC1–RC4 are desktop-only and WP1–WP3 are CLI-only, so breakage is
unlikely — but "unlikely" is not "checked".

```sh
cd /Users/dspury/Projects/buzz-rc-custody
. ./bin/activate-hermit
just ci                     # fmt + clippy + desktop lint + unit tests + builds
```

Repeat on `buzz-pr-fixtures` (covers WP1–WP3) and `buzz-rc-discovery`.

Integration tests need Postgres + Redis and only matter if `buzz-relay`,
`buzz-db`, or `buzz-auth` were touched — none were:

```sh
just test
```

### 5.2 Automated re-verification of this session's work

```sh
cd /Users/dspury/Projects/buzz-rc-custody && . ./bin/activate-hermit

# the four fixed areas
cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib managed_agents::ssh_config
cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib managed_agents::remote_probe
cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib managed_agents::connected_agents
cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib remote_agent_connect

cd desktop
CHECK_FILE_SIZES_BASE=upstream/main pnpm check    # must be exit 0
pnpm exec tsc --noEmit
pnpm test
pnpm exec node --test src/features/agents/ui/connectAgentIntent.test.mjs
```

The `CHECK_FILE_SIZES_BASE=upstream/main` override matters: without it the guard
resolves `merge-base origin/main HEAD`, and `origin` here is the fork, so the
base is wrong and the result is meaningless.

Lane B:

```sh
cd /Users/dspury/Lunar-Park/integrations/openclaw-channel-buzz
npm test && npm run typecheck && npm audit --omit=dev && npm pack --dry-run
```

### 5.3 Manual testing — the SSH probe (highest risk)

This is the part automated tests cannot cover, and the part most changed.

```sh
cd /Users/dspury/Projects/buzz-rc-custody && . ./bin/activate-hermit
just dev          # full Tauri app; `just desktop-dev` is web-only and has no ssh backend
```

Then in Settings → Agents → Connected Agents:

1. **A host already in `known_hosts`** → probes normally, harness list populated.
2. **A host in `~/.ssh/config` but NOT in `known_hosts`** — the important case.
   Expect: probe fails, label reads `host key not trusted`, and the dialog says
   the key is not yet trusted and to run `ssh <host>` and check the fingerprint.
   Then confirm **`~/.ssh/known_hosts` was not modified** — that is the actual
   defect being fixed:
   ```sh
   shasum ~/.ssh/known_hosts    # before and after probing; must not change
   ```
3. **`ssh <host>` manually, accept the key, probe again** → now succeeds. This is
   the intended trust hand-off.
4. **A host with a deliberately wrong `Port`** → confirm it fails on *that*
   host's port and does not silently use a neighbouring stanza's. Use this ssh
   config to test the parser fix end to end:
   ```
   Host probe-alpha
   Host probe-beta
     Port 2222
   ```
   `probe-alpha` must be probed on the default port, not 2222.
5. **A pattern stanza after a real host** → confirm the real host keeps its own
   port:
   ```
   Host probe-gamma
     Port 22
   Host *
     Port 9999
   ```
6. **Connect a self-hosted agent** with `--harness` unset and with it set;
   confirm `~/Library/Application Support/<bundle>/…/connected-agents.json`
   appears with only the six public fields, and that
   `managed-agents.json` is byte-identical before and after.
7. **Confirm no start/stop control** renders for a connected agent, and that
   Disconnect removes only the local pointer (no tombstone, no process action).

For truncation there is no clean manual repro; the automated test drives
`run_probe` with a real subprocess emitting `PROBE_START` and no `PROBE_END`.

### 5.4 Not attempted, still open

- **Lane A definition-of-done items**: generating a fresh remote identity and
  publishing it against a live relay; Desktop connect → display → mention →
  attach-to-channel; the lunar02 connect-flow validation. None of this has been
  exercised against a running relay.
- **Lane B live Checkpoint C** revalidation of the no-final turn case.
- **Lane B remote** — must be configured before publishing.
- Dialog copy nit: for a non-`password_required` failure the dialog shows
  `probe.error` and drops the "You can still record the agent now"
  reassurance, even though a probeless connect is supported. Pre-existing.
- Manifest/runtime mismatch nit: `openclaw.plugin.json` caps `silentAckEmoji` at
  `maxLength: 64` (UTF-16 units) while `src/config.ts` checks 64 code points.
  The manifest is stricter, so it rejects first; harmless defence in depth.

---

## 6. Upstream coordination — read before filing anything

**Do not open a competing upstream PR.** `block/buzz#2942`
(`bartok9/feat-external-agent-2663`, head `59ccda895`, updated 2026-07-28) is
**open** and now 793 additions across 10 files. It creates
`crates/buzz-cli/src/commands/listen.rs` and `docs/cli-external-agents.md` — the
exact two paths WP2 creates — and contains `#2933`'s `buzz users me` verbatim, so
**#2933 is subsumed by #2942**. WP1–WP3 can no longer be filed as independent
narrow PRs.

Findings against #2942's current head, verified against the relay source, not
assumed:

- `req.rs:1009` `extract_channel_id_from_filters` returns `None` when multiple
  distinct `#h` UUIDs appear *or* when any filter lacks an `h` tag, and
  `subscription.rs:387` states global subscriptions do not receive
  channel-scoped events. #2942 builds one filter with a multi-value `#h`, so:
  - `--channel X --channel Y` → global sub → backlog arrives, then **live
    delivery is silently zero**. `--channel` is documented repeatable.
  - `--mentions-of-me` alone → no `h` tag → global sub → **nothing live ever
    arrives** for the channel-scoped kinds it defaults to.
  - `--channel X --mentions-of-me` (their own `after_help` example) delivers, but
    the mentions filter is a strict subset of the channel filter, so the flag is
    a no-op.
  WP2's fix is one filter *and one REQ per channel* plus
  `resolve_listen_channels`; that is why it is 619 lines against their 279.
- No versioned lifecycle envelope; lifecycle goes to stderr as ad-hoc
  unversioned JSON and EOSE is discarded, so a consumer cannot tell "caught up"
  from "still replaying".
- No `--since`, so every reconnect duplicates or drops with no cursor.
- No executable fixture contract.
- **Contract conflict:** upstream compact output is `{id, content, created_at}`.
  WP1 adds `pubkey`, `tags`, **and `kind`**; #2942 adds `pubkey` and `tags` and
  explicitly asserts `assert!(row.get("kind").is_none(), "compact omits kind")`.
  Their listen defaults span five kinds, so under #2942 a listener gets an
  ambiguous stream it cannot route. If #2942 lands first, that assertion is the
  upstream contract and WP1/WP3 must be re-expressed against it.

Realistic options: review comments on #2942 carrying the relay-scoping evidence,
or a follow-on PR stacked on it. Lead with the multi-channel live-delivery bug —
it is a functional defect in merged behaviour, not a contract preference.

---

## 7. Stop state — unchanged, still binding

- **lunar01 native Buzz channel remains disabled.** Do not re-enable it, and do
  not start connector cutover, without new explicit approval.
- The legacy Python connector and webhook adapter remain the production and
  rollback path. Do not extend them except for production break/fix.
- **Checkpoint C remains failed** until the former no-final turn case passes live
  under an isolated identity.
- Hermes: the compatibility spike passed, but no plugin repo and no MVP exist.
  Do not start it until the shared OpenClaw lifecycle contract passes.
- No production identity or connector authority changed this session. Nothing
  here touched a deployment.
- Resident private keys must never enter the Buzz repo, fixtures, plugin
  manifests, logs, or kind:30177. Lunar Park hostnames, service definitions, and
  identities must stay out of upstream Buzz PRs. (`lunar01`/`lunar02` appear in
  the Lane B plugin repo, which is private Lunar Park material — that is fine;
  they must not cross into `block/buzz`.)
- `/Users/dspury/Projects/buzz-upstream-merge` (`d53ccc486`) is historical only:
  it contains the abandoned `KeyCustody`-on-`ManagedAgentRecord` design. Do not
  continue from it, and do not restore that design.

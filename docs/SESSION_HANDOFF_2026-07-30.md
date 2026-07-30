# Session Handoff — 2026-07-30 (fork consolidation)

Status: current handoff. Supersedes SESSION_HANDOFF_2026-07-29.md (moved to history/).

## 1. Binding stop state

- The fork now has ONE mainline: `main` = upstream/main (06582ee6f, 2026-07-30)
  + 9 gate/docs commits + the rebased 23-commit product stack + Phase 2
  completions. Old origin/main and the local merge lineage are preserved as
  tags (`archive/origin-main-2026-07-24`, `archive/upstream-merge`).
- Gate C canary retirement is on record: OpenClaw-in-Buzz was proven live
  (roster picker end-to-end, connection, channel add, team membership);
  practice usage supersedes the remaining canary matrix. The old §3A runbook
  is historical.
- Phase 3 (live enablement) is READY: see §4 runbook. No lunar01 state was
  changed this session.
- No upstream PRs were opened. Two PR-candidate branches are prepared and
  validated: `pr/keys-generate` (RC1) and `pr/profile-clobber-fix` (RC2).

## 2. What this session did

1. Rebuilt the fork as a patch queue on upstream/main:
   - Cherry-picked docs/deploy + the two clean gates (autostart, personas).
   - Re-derived the deletion-style strips as explicit gates:
     `BUZZ_VOICE_MODEL_AUTODOWNLOAD` (voice model downloads, was commented-out
     code), `VITE_BUZZ_WELCOME_EXPERIENCE` (welcome channel/team/canvas/
     kickoff, was deleted code — upstream code paths now stay intact),
     respond-to `nobody` default (exposed through Rust + TS types).
   - Rebased the 22-commit WP1-3/RC1-4/integration/P-stack lineage onto it
     (3 conflict stops, all resolved; upstream's newer AgentsView layout
     adopted).
2. Phase 2 completions on the new mainline:
   - P2 final wiring: card menu 'Delete' → 'Remove from My Agents' stage-one
     archive dialog; remote-orphan rules moved across; archiving a built-in
     also deactivates its definition.
   - 'Restore Buzz starter agents' in the new-agent menu (ignores the persona
     gate deliberately). Fixes the Fizz/Honey/Bumble unclean-removal problem
     end to end.
   - RC6 Buzz-side: `buzz listen --dms` streams DM conversations with stable
     UUIDs in `h` tags, discovers new conversations by 30s poll,
     `dm_channel_added` lifecycle record under v1 envelope.
   - Visual verification: two-stage-removal screenshot spec (4 shots) plus a
     connect-flow spec (mock host): resolved-identity field, roster picker with
     the primary preselected, and the minted owner-attestation dialog with its
     BUZZ_AUTH_TAG install guidance — all verified. The roster path was also
     live-verified against lunar01 on 2026-07-30.
3. Upstream-relevant fixes found on the way:
   - upstream #3607 broke macOS `cargo clippy --all-targets -D warnings`
     (linux_media dead code) — fixed with cfg gates; upstreamable.
   - persona merge tests now take the pack as a parameter (env-race-free).

## 3. Upstream coordination (refreshed 2026-07-30)

- #2933 (`users me`): functionally identical to our WP1 slice, same JSON
  shape and even function names. Fully superseded when it merges; our copy
  rebases to a no-op. Do not open a competing PR.
- #2942 (external-agent realtime path): their `buzz listen` has `--webhook`
  but no v1 envelope, no lifecycle records, no `--dms`. The OpenClaw plugin
  depends on our envelope contract. When #2942 merges, keep our listen as a
  superset and consider upstreaming `--envelope v1` + `--dms` as a follow-up
  PR on top of theirs.
- Prepared, validated, NOT pushed as PRs:
  - `pr/keys-generate` — RC1 `buzz keys generate` on upstream/main.
  - `pr/profile-clobber-fix` — RC2 kind:10100 read-modify-write fix on
    upstream/main. Submit this one first (live repro story from Gate C).

## 4. Phase 3 — live enablement runbook (lunar01, needs operator)

Sequencing: build → connect flow visual check → adapter config → reply
authority → verify → soak. The legacy connector stays untouched as rollback.

### Step 0 — build and launch the Desktop app from the new main

```sh
cd /Users/dspury/Projects/buzz-main-next   # or a fresh checkout of main
. ./bin/activate-hermit
just desktop-standalone
```

Sanity check while connecting: the identity field offers the resolved host
identity, the roster picker lists durable agents, and the attestation dialog
shows/copies the auth tag (all already verified against the mock host).

### Step 1 — connect Selene through the full flow

Join community `ws://lunar01:3000` (scheme required). Connect agent: pick
`lunar01` → `openclaw` harness → the primary durable agent from the roster →
accept the resolved identity (`channels.buzz.agentPubkey`, canary
`4687f50d…e3c9`). Add to the channels you want Selene in.

### Step 2 — mint + install owner attestation (fixes owner_only, P6)

In the connected agent's owner-attestation dialog, copy the tag. On lunar01:

```bash
OPENCLAW_BIN=/opt/homebrew/bin/openclaw
"$OPENCLAW_BIN" config set channels.buzz.authTag '<paste tag JSON>' --strict-json
"$OPENCLAW_BIN" config validate
```

### Step 3 — restore the canary's full profile (undo the Gate-C-era clobber)

From any machine with the NEW buzz CLI and the agent's key on the host —
run on lunar01 as the agent identity:

```bash
buzz agents profile set \
  --display-name Selene \
  --agent-type openclaw \
  --capabilities conversation \
  --channel-add-policy owner_only
```

(RC2's read-modify-write preserves other fields; policy can now be
`owner_only` because the attestation materializes the owner.)

### Step 4 — point the adapter at the real channels and enable

```bash
"$OPENCLAW_BIN" config set channels.buzz.channelIds '["<real channel uuids>"]' --strict-json
"$OPENCLAW_BIN" config set channels.buzz.enabled true --strict-json
"$OPENCLAW_BIN" config validate
"$OPENCLAW_BIN" gateway restart && "$OPENCLAW_BIN" gateway status
```

For DM parity, run the adapter's listener with `--dms` (new CLI build) so DM
conversations stream with their stable UUIDs.

### Step 5 — reply authority (the one hard rule)

Remove legacy production Selene (`2311ce81…efa6`) from every channel the
native agent serves (owner-signed kind:9001 from Desktop). The legacy
connector holds a global `#p` mention subscription; membership is the only
boundary. Never let both identities serve one channel.

### Step 6 — verify + practice-as-soak

One mention (pick from `@` autocomplete for a real `p` tag) → one threaded
reply; `gateway restart` → no duplicate. Then use it normally. Watch:
adapter `pending` stays 0, no duplicate replies after Gateway respawns, no
legacy-Selene responses in native channels.
Rollback: `channels.buzz.enabled false` + re-add legacy Selene to channels.

## 5. Repository state after this session

| Ref | Meaning |
|---|---|
| `main` (= `origin/main`) | the consolidated patch queue: upstream/main `06582ee6f` + gates + rebased stack + Phase 2 |
| `pr/keys-generate`, `pr/profile-clobber-fix` | validated PR candidates on upstream/main, pushed to origin, no PRs opened |
| `archive/origin-main-2026-07-24` | the pre-consolidation fork main (old strip commits) |
| `archive/upstream-merge` | the retired local merge lineage (contains the retired KeyCustody commit — historical only) |
| `archive/openclaw-test-integration-2026-07-29` | the preserved manual Gate C test point `b19fd1508` |
| `lunar-park/rc-p6-owner-attestation` | the pre-rebase stack; still checked out in `buzz-rc5-roster` (the previously running app build) |
| `feature/agent-harness-adapter-alignment` | the planning branch (docs corpus source), pushed |

Removed worktrees: buzz-upstream-merge, buzz-openclaw-integration,
buzz-pr-identity/listen/fixtures, buzz-rc-keys/profile/discovery/custody.
Kept: buzz-main-next (the new main), buzz-rc5-roster (running app build).

Validation on `main` (ratchet base pinned to upstream/main until origin/main
was updated): workspace fmt + clippy `-D warnings`, Tauri fmt + clippy, all
three desktop guards, workspace unit tests, 307 buzz-cli tests, 2,036 Tauri
tests, 3,878 desktop JS tests, production + e2e Vite builds, web build, and
the new 2-test screenshot spec. Known upstream breakages, not ours:

- mobile `channel_detail_page_test.dart` "keeps follow mode off while a tall
  newest message stays visible" fails identically on pristine upstream/main
  in this environment;
- upstream #3607 made `cargo clippy --all-targets -D warnings` fail on macOS
  (linux_media dead code) — fixed on our main, upstreamable.

## 6. Safety invariants (unchanged)

- No resident nsec/private key/auth tag enters this repository.
- Buzz and a legacy/native adapter never share reply authority for one
  identity/channel pair.
- Connected records never enter managed-agent lifecycle or storage paths.
- SSH probing never accepts or persists a host key.
- No production connector cutover without the §4 sequence and rollback.

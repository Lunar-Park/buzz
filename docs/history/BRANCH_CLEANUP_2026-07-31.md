# Fork branch cleanup manifest — 2026-07-31

This manifest records the remote branch cleanup performed after consolidating
the Lunar Park fork into `lunar/integration`.

## Ground truth before deletion

- `origin/main`: `209536ade` — matches `upstream/main`
- `lunar/integration` pre-manifest head: `383611b2a`
- `git rev-list --left-right --count upstream/main...origin/main`: `0 0`
- `git rev-list --count origin/main..lunar/integration`: `45`
- Fork PRs in `Lunar-Park/buzz`: none
- Open upstream PRs found from Lunar-Park heads: none

`main` remains the clean upstream mirror. `lunar/integration` remains the only
current Lunar Park integration branch and the source branch for local testing.
Upstream PR candidates must be rebuilt as focused `pr/<topic>` branches from
current `origin/main`; the old topic branches below are not valid PR bases.

## Substantive Lunar branch coverage

The old Lunar working branches below were checked against `lunar/integration`
before deletion. Their current work is represented by replayed/rebased commits
on `lunar/integration`; old commit IDs may not be patch-identical after conflict
resolution, but the substantive subjects are present on the integration branch.

| Old subject | Integration commit |
| --- | --- |
| `feat(cli): add buzz keys generate for self-hosted agent identities` | `297c162d0` |
| `fix(cli): stop kind:10100 writes from clobbering the agent profile` | `bf907880d` |
| `feat(cli): add external agent identity primitives` | `656ea3afd` |
| `fix(cli): preserve scoped live listen delivery` | `e7c3d6c92` |
| `test(cli): execute external agent fixture contract` | `c4a84f97d` |
| `chore(desktop): keep remote-discovery additions inside the size guards` | `29ba5fef6` |
| `feat(desktop): connect self-hosted agents Buzz does not own` | `f43c37908` |
| `ci: add the agent-harness ACP acceptance contract` | `e3cfde99f` |
| `docs: correct the relay protocol limits in ARCHITECTURE.md` | `60b79e170` |
| `feat: add configurable agent behavior controls` | `1f13a7514` |

Older dated handoff/timeline documentation from
`feature/agent-harness-adapter-alignment` and
`wip/agent-behavior-live-test-20260731` was intentionally not replayed
verbatim. The current canonical fork shape is documented in
`docs/LUNAR_PARK_FORK_INDEX.md`; recovery copies remain under `archive/*`.

## Branches retained

| Branch | SHA | Reason |
| --- | --- | --- |
| `main` | `209536ade` | clean upstream mirror |
| `lunar/integration` | `383611b2a` before this manifest commit | complete current fork delta |
| `archive/lunar-main-pre-sync-20260731` | `c85b596d0` | recovery snapshot |
| `archive/lunar-park-rc-p6-owner-attestation-20260731` | `c29c4b161` | recovery snapshot |
| `archive/feature-agent-harness-adapter-alignment-20260731` | `c69fbae74` | recovery snapshot |

## Branches approved for deletion

### Stale Lunar working branches

| Branch | SHA |
| --- | --- |
| `wip/agent-behavior-live-test-20260731` | `515a963be` |
| `feature/agent-harness-adapter-alignment` | `c69fbae74` |
| `buzz-rc-custody` | `098711146` |
| `buzz-rc-discovery` | `ad1c5a334` |
| `buzz-rc-profile` | `ce69cc3fe` |
| `buzz-rc-keys` | `26bb74aa2` |
| `buzz-pr-identity` | `e14a94ec3` |
| `buzz-pr-listen` | `1d65784d5` |
| `buzz-pr-fixtures` | `68513902c` |
| `pr/keys-generate` | `5b487b45e` |
| `pr/profile-clobber-fix` | `a468a4999` |

### Screenshot and bot branches

| Branch | SHA |
| --- | --- |
| `agent-screenshots/delkc` | `f29310a1c` |
| `agent-screenshots/klopez4212` | `4643581cd` |
| `agent-screenshots/thomaspblock` | `22a97181d` |
| `agent-screenshots/wpfleger96` | `728896e5a` |
| `renovate/clap-4.6.x-lockfile` | `13ecb5097` |
| `renovate/nostr-0.44.x-lockfile` | `ce934d6c9` |

### Copied or stale collaborator topic branches

These branches were present on the fork remote but are not part of the current
Lunar Park fork trunk. They had no fork PRs and no open upstream PRs found from
Lunar-Park heads at cleanup time.

| Branch | SHA |
| --- | --- |
| `agent/fix-avatar-upload-lifecycle` | `a4417306e` |
| `agent/fix-relay-req-lifecycle-race` | `49a1ed683` |
| `atish/sanitize-animated-images` | `c8cfd93ad` |
| `baxen/align-delete-data-buttons` | `013a1e253` |
| `baxen/production-relay-onboarding` | `12293366f` |
| `brain/remove-agent-directory` | `6864e6184` |
| `carl/preserve-agent-snapshot-text` | `687b9720e` |
| `claydelk/inbox-refactor` | `72933949c` |
| `claydelk/public-project-docs-review` | `22a9ab857` |
| `claydelk/public-readme-refresh` | `428e04b4e` |
| `claydelk/system-content-polish` | `403b17aeb` |
| `duncan/acp-resume-ledger` | `715779ed2` |
| `duncan/agent-config-resolver` | `32c2eeddc` |
| `duncan/agent-usage-archive` | `122b04360` |
| `duncan/nip-rs-unread-model` | `47a476a80` |
| `duncan/openrouter-provider` | `905401db3` |
| `feat/project-work-items-inbox` | `a21b7da4b` |
| `feat/pull-request-review-polish` | `b6d9ac880` |
| `fizz/community-profile-on-join` | `a4446c44f` |
| `fsola/corporate-identity` | `bd822f3ea` |
| `grumplestiltzkin/2287-agent-identity` | `85c4d80fd` |
| `grumplestiltzkin/backend-selector-edit-import` | `b7f370ffd` |
| `grumplestiltzkin/generic-file-upload` | `b194bda75` |
| `inbox-message-edit-action` | `0bb3722fe` |
| `kennylopez-agent-catalog-sharing` | `691b305be` |
| `kennylopez-dictation` | `3fa923dc8` |
| `kennylopez-edit-channel-overlay` | `409efaf78` |
| `lazyjoe/reconnect-testability-refactor` | `eccaeab20` |
| `micn/mesh-state-fix` | `298a0fe28` |
| `pinky/fix-sprig-rolling-release` | `dc9c04ac7` |
| `quietly/internal-owner-only` | `f0173c0b4` |
| `tbates/add-members-search` | `558941a3d` |
| `tomb/lighter-weight-mobile-releasing` | `bfdb9443b` |
| `tomb/mobile-inbox-thread-target` | `098df7ebc` |
| `wpfleger96/community-rail-reorder` | `0c1135586` |

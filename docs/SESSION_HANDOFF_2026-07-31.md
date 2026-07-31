# Session Handoff — 2026-07-31

Status: current handoff for the Buzz connected-agent cleanup and live Selene
test pass. Supersedes
[`SESSION_HANDOFF_2026-07-29.md`](SESSION_HANDOFF_2026-07-29.md).

Read first:

1. [Lunar Park Fork Document Index](LUNAR_PARK_FORK_INDEX.md)
2. [Self-Hosted Agent Integration Specification](SELF_HOSTED_AGENT_INTEGRATION_SPEC.md)
3. [Remote Agent Connect Implementation Plan](REMOTE_AGENT_CONNECT_IMPLEMENTATION_PLAN.md)
4. [Resident Agent Adapter Implementation Plan](RESIDENT_AGENT_ADAPTER_IMPLEMENTATION_PLAN.md)

## 1. Binding stop state

- The current Desktop build completed live Selene replies in a private channel
  and in a DM.
- Channel agent reply placement can be toggled between threaded and inline.
  Threaded remains the default; inline replies were live-tested successfully.
- Owned-agent DMs can be configured to wake without an explicit @mention.
  No-mention DM wake-up was live-tested successfully.
- The DM side panel exposes the same agent-behavior controls as channels,
  without exposing DM name/description/lifecycle edits.
- External/self-hosted agents that are actual channel members remain
  mentionable even when relay directory invocability metadata is missing or
  stale.
- Full Gate C/Gate D cutover evidence is still not complete until the negative
  paths and restart/replay checks in the product spec are run and recorded.
- Do not push Lunar Park planning docs in upstream `block/buzz` PRs.

## 2. Current repository state

```text
worktree  /Users/dspury/Projects/buzz
branch    feature/agent-harness-adapter-alignment
head      c69fbae74
state     dirty; implementation and documentation cleanup in progress
```

Current uncommitted work is in these lanes:

- CLI: `buzz listen`, `buzz users me`, `BUZZ_AUTH_TAG` forwarding, and channel
  agent-behavior update flags.
- Relay/db/sdk: persisted `agent_reply_mode` and `dm_require_mention`, metadata
  validation, owner-agent DM behavior authorization, and update-event tags.
- Desktop: channel/DM behavior toggles, owned-agent DM edit affordance,
  fallback-safe DM participants, and external-agent mention eligibility.
- Docs: CLI README/testing coverage and this integration-spec status update.

## 3. Upstream PR split recommendation

Prefer splitting the current stack before upstream publication:

1. CLI external-agent substrate: `buzz listen`, `users me`, `BUZZ_AUTH_TAG`
   handling, and CLI docs/tests.
2. Channel/DM agent behavior policy: migration, db/relay/sdk surfaces, CLI
   flags, and desktop settings UI.
3. Desktop mention/autocomplete compatibility: external-agent member
   mentionability and DM participant fallback hardening.

Keep OpenClaw/Lunar Park runtime details and resident adapter deployment state
out of upstream PR bodies and commits.

## 4. Validation snapshot

Validation refreshed on 2026-07-31:

```bash
. ./bin/activate-hermit
cargo fmt --package buzz-cli --package buzz-relay --package buzz-sdk --package buzz-db
cargo fmt --manifest-path desktop/src-tauri/Cargo.toml
cd desktop && pnpm format -- src/shared/api/channelTypes.ts src/shared/api/types.ts src/features/channels/ui/AgentBehaviorSettings.tsx src/features/channels/ui/ChannelManagementSheet.tsx
cargo test -p buzz-cli listen
cargo test -p buzz-cli channels
cargo test -p buzz-cli users
cargo test -p buzz-sdk update_channel
cargo test -p buzz-relay handlers::side_effects::tests::edit_metadata_agent_behavior_only
cd desktop && pnpm typecheck
cd desktop && pnpm lint
cd desktop && pnpm check
cargo test --manifest-path desktop/src-tauri/Cargo.toml channels
cd desktop && node --import ./test-loader.mjs --experimental-strip-types --test src/features/agents/lib/agentAutocompleteEligibility.test.mjs
git diff --check
```

All commands above passed. A direct `node --test` invocation without
`desktop/test-loader.mjs` was attempted first and failed because it cannot
resolve the repo's `@/` path alias; the alias-aware command above is the valid
test invocation and passed.

## 5. Remaining live checks

- Negative path: non-owner or unrelated member cannot change DM
  `dm_require_mention` / `agent_reply_mode`.
- Negative path: missing mention still does not wake agents where
  `dm_require_mention=true`.
- Restart/replay: adapter or gateway restart does not duplicate the last
  successful channel or DM turn.
- Single-authority: legacy and native adapters do not both reply for the same
  identity/channel pair.

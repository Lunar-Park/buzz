# Upstream PR playbook

This is the fork-local checklist and wording template for preparing focused
pull requests from the Lunar Park Buzz fork into `block/buzz`.

Do not include this file itself in upstream PR branches unless maintainers ask
for it. It is operating guidance for this fork.

## Branch discipline

Every upstream PR branch starts from the clean upstream mirror:

```bash
git fetch origin --prune
git fetch upstream main --prune
git rev-list --left-right --count upstream/main...origin/main # must be 0 0
git switch -c pr/<topic> origin/main
```

Bring over only the reviewed commits, files, or hunks required for that PR.
Do not branch from `lunar/integration` directly for upstream publication, even
when the intended change already works there.

## Scope gate

Before pushing a PR branch, inspect for fork-only material:

```bash
git diff --name-only origin/main..HEAD
git diff --name-only origin/main..HEAD |
  rg '^(FORK.md|docs/LUNAR_PARK|docs/history|deploy/)'
rg -n 'lunar01|Lunar Park|OpenClaw|Hermes|Selene|dmtri' \
  $(git diff --name-only origin/main..HEAD)
```

Expected result for a generic upstream PR: no matches, unless that exact term is
the reviewed purpose of the PR.

## Validation gate

Run the smallest meaningful checks first, then expand if the PR crosses package
boundaries. Record exact commands and outcomes in the PR body.

For CLI-only PRs:

```bash
. ./bin/activate-hermit
cargo fmt -p buzz-cli --check
cargo clippy -p buzz-cli --all-targets -- -D warnings
cargo test -p buzz-cli
```

For desktop UI PRs, add the relevant desktop node/Tauri tests. For relay or SDK
PRs, add the relevant Rust package tests and database-backed tests only when the
change requires them.

## PR body template

```markdown
## Summary

<One short paragraph explaining what changed and why it belongs in Buzz core.>

## What changed

- <Concrete behavior/API/file-area change>
- <Concrete behavior/API/file-area change>
- <Tests/docs added, if any>

## Why

<Explain the user/developer problem, the previous limitation, and why this
approach fits the existing Buzz architecture. Avoid fork-specific deployment
details.>

## Compatibility / risk

- <Backward-compatible aspects>
- <Known risk or migration concern>
- <Explicitly unchanged behavior>

## Validation

- `<command>` — <result>
- `<command>` — <result>
```

## First PR draft: CLI external-agent foundation

Suggested title:

```text
feat(cli): add external-agent identity and listen primitives
```

Suggested body:

```markdown
## Summary

Adds the CLI primitives needed for a resident external agent to identify itself,
publish a complete agent profile, and consume scoped Buzz events without being
owned or supervised by Desktop.

## What changed

- Adds `buzz users me`, a local identity command that prints the configured
  public identity without making a relay request or exposing private key
  material.
- Adds `buzz listen`, which streams channel-scoped and mention-scoped relay
  events as NDJSON with a versioned envelope suitable for long-running external
  consumers.
- Adds `buzz keys generate`, a local-only identity generation command that
  writes secrets with owner-only permissions and refuses accidental overwrite.
- Adds `buzz agents profile get/set` and routes `channels set-add-policy`
  through the same read-modify-write path so kind:10100 updates do not clobber
  existing profile fields.
- Documents the external-agent CLI contract and adds fixture/subprocess tests
  for identity, listen, fixture normalization, and key generation behavior.

## Why

Buzz already has the relay and profile model needed for agents that run outside
Desktop, but the CLI was missing a small stable contract for those agents to
bootstrap safely. External adapters need to prove local key custody, publish the
agent's own directory record, and receive scoped events without relying on
Desktop-owned process lifecycle or private key transfer.

The implementation keeps the boundary narrow: it adds generic CLI primitives
and tests, without adding runtime-specific adapter code or new relay endpoints.

## Compatibility / risk

- Existing compact message consumers remain compatible; additional compact
  fields are additive.
- `buzz listen` is opt-in and uses explicit channel/mention filters.
- `buzz keys generate` does not require an existing Buzz identity or relay
  connection, and does not print the secret unless explicitly requested.
- kind:10100 writes become safer by preserving existing and unknown profile
  fields instead of publishing partial replacement documents.

## Validation

- `cargo fmt -p buzz-cli --check` — passed
- `cargo clippy -p buzz-cli --all-targets -- -D warnings` — passed
- `cargo test -p buzz-cli` — passed
```

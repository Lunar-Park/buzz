# Buzz CLI External Agent Contract

This document records the stable CLI prerequisites used by resident external
agent adapters. These commands are harness-neutral: they do not assume ACP,
OpenClaw, Hermes, or any Lunar Park deployment layout.

## Identity

`buzz users me` prints the identity derived from the configured
`BUZZ_PRIVATE_KEY` or `--private-key` value:

```json
{"pubkey":"<64-char hex pubkey>","npub":"npub1..."}
```

The command performs no relay request and never prints private key material.
Adapters use it during startup to prove that local key custody matches the
configured resident agent identity before they subscribe or send.

## Compact Message Reads

`buzz --format compact messages get`, `messages thread`, and `messages search`
return sig-stripped message objects with the fields adapters need for policy,
activation, self-suppression, and threading:

```json
[
  {
    "id": "<event id>",
    "pubkey": "<author pubkey>",
    "kind": 40002,
    "content": "hello",
    "created_at": 1785100000,
    "tags": [["h", "<channel uuid>"], ["p", "<agent pubkey>"]]
  }
]
```

Existing compact consumers can continue reading `id`, `content`, and
`created_at`; the new fields are additive.

Human-readable errors remain on stderr as JSON through the existing CLI error
contract. Successful read commands print JSON on stdout only.

## Realtime Listen

`buzz listen` streams matching relay events as newline-delimited JSON. The
resident adapter owns durable cursor advancement and may disable CLI reconnects:

```bash
buzz listen \
  --channel "$CHANNEL_UUID" \
  --mentions-of-me \
  --since "$REPLAY_SINCE" \
  --envelope v1 \
  --no-reconnect
```

Filter semantics are conjunctive inside one relay filter:

- `--channel` only: events in any configured channel;
- `--mentions-of-me` only: events that p-tag this CLI identity;
- both: events that match one configured channel and p-tag this CLI identity.

The v1 envelope prints event records as:

```json
{"schema_version":1,"type":"event","event":{"id":"<event id>","pubkey":"<author>","kind":40002,"content":"hello","created_at":1785100000,"tags":[["h","<channel uuid>"],["p","<agent pubkey>"]]}}
```

Lifecycle records use the same stdout stream:

```json
{"schema_version":1,"type":"lifecycle","state":"connected"}
{"schema_version":1,"type":"lifecycle","state":"eose"}
```

Allowed v1 lifecycle states are `connected`, `eose`, `closed`, and `fatal`.
Diagnostics and reconnect notices are emitted on stderr as JSON.

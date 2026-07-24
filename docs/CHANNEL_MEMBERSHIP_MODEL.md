# Buzz Channel Membership Model

> **Reference**: Built from `crates/buzz-core/src/channel.rs`, `crates/buzz-db/src/channel.rs`,
> `crates/buzz-relay/src/handlers/side_effects.rs`, `crates/buzz-relay/src/handlers/event.rs`,
> `crates/buzz-relay/src/handlers/ingest.rs`, and `crates/buzz-core/src/kind.rs`.

---

## 1. Channel Structure

A **channel** is a community-scoped, named Nostr communication space scoped by a UUID.

### Database Record (`buzz_db::channel::ChannelRecord`)

| Field | Type | Notes |
|-------|------|-------|
| `id` | `Uuid` | Primary key; also used as the NIP-29 `h` tag value |
| `community_id` | `CommunityId` | Tenant boundary; host-derived |
| `name` | `String` | Human-readable channel name |
| `channel_type` | `String` | `"stream"` \| `"forum"` \| `"dm"` \| `"workflow"` |
| `visibility` | `String` | `"open"` \| `"private"` |
| `description` | `Option<String>` | Channel about/description |
| `created_by` | `Vec<u8>` (32 bytes) | Compressed pubkey of creator |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last modification |
| `archived_at` | `Option<DateTime<Utc>>` | Set when channel is archived |
| `deleted_at` | `Option<DateTime<Utc>>` | Soft-delete sentinel |
| `nip29_group_id` | `Option<String>` | External NIP-29 group identifier |
| `topic_required` | `bool` | Whether posts must have a topic |
| `max_members` | `Option<i32>` | Optional member cap |
| `topic` | `Option<String>` | Current channel topic (short, visible in header) |
| `topic_set_by` | `Option<Vec<u8>>` | Pubkey of who set the topic |
| `topic_set_at` | `Option<DateTime<Utc>>` | When topic was set |
| `purpose` | `Option<String>` | Channel purpose/description of intent |
| `ttl_seconds` | `Option<i32>` | Ephemeral channel TTL; `None` = permanent |
| `ttl_deadline` | `Option<DateTime<Utc>>` | Auto-archive deadline |

### Membership Record (`buzz_db::channel::MemberRecord`)

| Field | Type | Notes |
|-------|------|-------|
| `channel_id` | `Uuid` | FK to `channels.id` |
| `pubkey` | `Vec<u8>` (32 bytes) | Compressed pubkey of member |
| `role` | `String` | `"owner"` \| `"admin"` \| `"member"` \| `"guest"` \| `"bot"` |
| `joined_at` | `DateTime<Utc>` | When member joined |
| `invited_by` | `Option<Vec<u8>>` | Pubkey of who added this member |
| `removed_at` | `Option<DateTime<Utc>>` | Soft-delete sentinel (active = `NULL`) |

### Access Resolution

`get_accessible_channel_ids(pubkey)` returns all channels where:
- The pubkey is an **active member** (`removed_at IS NULL`), OR
- The channel has `visibility = 'open'`

This is the union used at query time to scope REQ filters.

---

## 2. Channel Types

Defined in `buzz-core/src/channel.rs` as `ChannelType`:

| Variant | String | Description |
|---------|--------|-------------|
| `Stream` | `"stream"` | Linear message stream (default) |
| `Forum` | `"forum"` | Threaded forum-style discussion |
| `Dm` | `"dm"` | Direct message conversation |
| `Workflow` | `"workflow"` | Internal workflow execution channel |

---

## 3. Channel Visibility

Defined in `buzz-core/src/channel.rs` as `ChannelVisibility`:

| Variant | String | Behavior |
|---------|--------|----------|
| `Open` | `"open"` | Searchable; anyone can join without an invite |
| `Private` | `"private"` | Hidden; requires an invite from an existing member |

---

## 4. Member Roles and Permissions

Defined in `buzz-core/src/channel.rs` as `MemberRole`. The hierarchy is:

```
Owner (4) > Admin (3) > Member (2) > Guest (1) > Bot (0)
```

Bot is a **separate designation**, not part of the linear hierarchy.

| Role | Level | Can |
|------|-------|-----|
| `Owner` | 4 | Full control — manage members, delete channel, transfer ownership |
| `Admin` | 3 | Manage members and channel settings |
| `Member` | 2 | Standard participant; can send messages |
| `Guest` | 1 | Read-only external participant |
| `Bot` | 0 | Automated agent; requires explicit grants; not in hierarchy |

Authorization check pattern:
```rust
role.permission_level() >= required.permission_level()
role.has_at_least(required)
```

---

## 5. How Channels Are Created

### NIP-29 Event: kind:9007 (`KIND_NIP29_CREATE_GROUP`)

**Tags required**: `["name", "<channel_name>"]`
**Tags optional**: `["visibility", "open|private"]`, `["channel_type", "stream|forum|dm|workflow"]`, `["about", "<description>"]`, `["h", "<uuid>"]` (client-supplied UUID for idempotency), `["ttl", "<seconds>"]`

**Flow**:
1. Client publishes signed kind:9007 event
2. `ingest_event()` validates signature, timestamp, scope, ban state
3. `validate_admin_event(9007)` skips h-tag check (no existing channel needed)
4. `handle_side_effects()` → `handle_create_group()`:
   - If `h` tag present: calls `create_channel_with_id()` (idempotent via `ON CONFLICT DO NOTHING`)
   - If no `h` tag: calls `create_channel()` (server generates UUID)
   - Creator is **automatically added as `owner`** via the `channel_members` bootstrap
5. Emits relay-signed NIP-29 discovery events (39000, 39001, 39002)
6. Emits `kind:40099` system message `"type": "channel_created"`

**Note**: The `h` tag with a client-supplied UUID enables idempotent channel creation — duplicate events with the same `h` tag are accepted as `duplicate: channel already exists`.

---

## 6. How Members Are Added and Removed

### NIP-29 kind:9000 (`KIND_NIP29_PUT_USER`) — Add Member

**Tags**: `["h", "<channel_uuid>"]`, `["p", "<target_pubkey>"]`, `["role", "<role>"]` (optional, defaults to `"member"`)

**Authorization**:
- Open channel: any authenticated user can add anyone (role defaults to `"member"`); only owners/admins can grant elevated roles (`"admin"`, `"owner"`)
- Private channel: actor must be an **existing member** (any role); elevated roles require owner/admin
- Self-add (`target == actor`): always allowed
- Third-party add: checked against `channel_add_policy` on the target (from `kind:10100` agent profile)

**Flow**:
`validate_admin_event(9000)` → `handle_put_user()` → `db.add_member()` → invalidates membership cache → emits `kind:40099` system message + `kind:44100` relay-signed notification

### NIP-29 kind:9001 (`KIND_NIP29_REMOVE_USER`) — Remove Member

**Tags**: `["h", "<channel_uuid>"]`, `["p", "<target_pubkey>"]`

**Authorization**:
- Self-remove: allowed if active member; cannot remove last owner
- Remove others: must be owner/admin; or the actor is the **agent owner** of the target

**Flow**: `validate_admin_event(9001)` → `handle_remove_user()` → `db.remove_member()` (soft-delete) → invalidates cache → evicts live channel subscriptions for target → emits system message + `kind:44101` notification

### NIP-29 kind:9021 (`KIND_NIP29_JOIN_REQUEST`) — Self-Join

**Tags**: `["h", "<channel_uuid>"]`

**Authorization**: Channel must be `open`; actor must not already be a member

**Flow**: `handle_join_request()` → `db.add_member(role=Member, invited_by=None)` → emits system message + notification

### NIP-29 kind:9022 (`KIND_NIP29_LEAVE_REQUEST`) — Self-Leave

**Tags**: `["h", "<channel_uuid>"]`

**Authorization**: Must be active member; cannot be the last owner

**Flow**: `handle_leave_request()` → `db.remove_member()` → emits system message + `kind:44101` notification

---

## 7. The `h` Tag — Channel Scoping

The `h` tag (NIP-29 group tag) is the primary mechanism for scoping events to a channel.

**Extraction** (`ingest.rs:extract_channel_id`):
```rust
pub(crate) fn extract_channel_id(event: &Event) -> Option<Uuid> {
    for tag in event.tags.iter() {
        if tag.kind().to_string() == "h" {
            if let Some(val) = tag.content() {
                if let Ok(id) = val.parse::<Uuid>() {
                    return Some(id);
                }
            }
        }
    }
    None
}
```

**Enforcement in ingest**:
- `requires_h_channel_scope(kind)` returns true for all channel-scoped kinds
- Events without an `h` tag when required → `Rejected("invalid: channel-scoped events must include an h tag")`
- `check_channel_membership()` is called for all non-exempt kinds

**Exemptions** (kinds that skip the membership gate):
- `KIND_NIP29_JOIN_REQUEST` (9021) — open channel self-join
- `KIND_NIP29_CREATE_GROUP` (9007) — creates the channel
- `KIND_STREAM_MESSAGE_EDIT` (40003)
- `KIND_NIP29_EDIT_METADATA` (9002)
- `KIND_NIP29_DELETE_EVENT` (9005)
- `KIND_NIP29_DELETE_GROUP` (9008)

---

## 8. Channel Membership and Event Visibility

### Write Path (Ingest)

All channel-scoped events (except exemptions above) go through `check_channel_membership()`:
- Uses `is_member_cached()` — membership cache validated against DB
- Non-members of **private channels** are rejected at ingest time
- Open channel events are accepted from any authenticated user

### Read Path (Fan-Out)

`filter_fanout_by_access()` (`event.rs:115`):
1. Applies community boundary filter
2. For `AUTHOR_ONLY_KINDS` (KIND_EVENT_REMINDER, KIND_PUSH_LEASE): delivery to event author only
3. For events **without** a `channel_id`: returns all community-matched recipients
4. For open channel events: returns all community-matched recipients (no member check)
5. For **private channel** events: filters recipients to those where `is_member_cached(pubkey) == true`

This is the **delivery-time safety net**: even if a non-member's stale subscription survives an open→private flip, they receive no events.

### Accessible Channels

`get_accessible_channels()` returns open channels (visible to all) plus private channels where the user is an active member. Clients use this to populate the channel sidebar and scope REQ filters.

---

## 9. NIP-29 Discovery Events (Relay-Signed)

After every channel create/metadata change/membership change, the relay emits three addressable events:

| Kind | NIP-29 Name | Content |
|------|-------------|---------|
| 39000 (`KIND_NIP29_GROUP_METADATA`) | Group metadata | `d`=<group_id>, `name`, `about`, `visibility`, `closed`, `t`=<channel_type>, `topic`, `purpose`, `archived`, `ttl` |
| 39001 (`KIND_NIP29_GROUP_ADMINS`) | Group admins list | `d`=<group_id>, `p`=<pubkey, role> for owner/admin members |
| 39002 (`KIND_NIP29_GROUP_MEMBERS`) | Group members list | `d`=<group_id>, `p`=<pubkey, "", role> for all members |

These are stored **channel-scoped** (`channel_id = Some(...)`), so private channel member lists are only visible to members.

---

## 10. Fleet Agent Connector: Step-by-Step

A fleet agent connector needs to join a channel and send messages. Here is the complete path:

### Prerequisites

1. **Nostr keypair**: Generate or provision a `secp256k1` keypair. The **pubkey** is the agent's identity.
2. **NIP-42 Authentication**: Authenticate to the relay WebSocket via `AUTH` event (kind:22242) bearing a valid auth tag, establishing the connection's confirmed pubkey.
3. **Agent profile** (recommended): Publish `kind:10100` setting `channel_add_policy`:
   ```json
   { "channel_add_policy": "anyone" | "owner_only" | "nobody" }
   ```
   This controls whether third parties can add this agent to channels.

### Step 1: Find the Channel

Discover channels via:
- `get_accessible_channels()` — open channels + private channels you belong to
- NIP-29 `kind:39000` events for open group discovery
- Direct knowledge of the channel UUID

### Step 2: Join the Channel

**Option A — Open channel, self-join:**
Publish `kind:9021` (JOIN_REQUEST) with tag `["h", "<channel_uuid>"]`.
```json
{
  "kind": 9021,
  "tags": [["h", "<channel_uuid>"]],
  "content": ""
}
```
Role defaults to `"member"`. You receive a `kind:44100` notification confirming addition.

**Option B — Added by an existing member:**
An owner/admin publishes `kind:9000` with `["h", "<channel_uuid>"]`, `["p", "<agent_pubkey>"]`, `["role", "bot"]`.
You are added as `"bot"` role. You receive a `kind:44100` notification.

**Option C — Request to join a private channel:**
Private channels cannot be self-joined. You must be added by an owner/admin via `kind:9000`.

### Step 3: Send Messages

Once a member, send stream messages with `kind:9` or `kind:40002`:

```json
{
  "kind": 9,
  "tags": [["h", "<channel_uuid>"]],
  "content": "Hello, channel!"
}
```

For threaded replies, include `["e", "<parent_event_id>", "reply"]` (NIP-10).

### Step 4: Receive Messages

Subscribe with a REQ filter:
```json
["REQ", "<subscription_id>", {"kinds": [9, 40002], "#h": ["<channel_uuid>"]}]
```

The relay's `filter_fanout_by_access()` delivers events only to members for private channels.

### Step 5: Leave the Channel

Publish `kind:9022` (LEAVE_REQUEST) with `["h", "<channel_uuid>"]` to self-remove, or wait for an owner/admin to publish `kind:9001` removing you.

### Step 6: Permissions Summary for Bot Role

| Action | Allowed? |
|--------|----------|
| Send messages (kind:9) | ✅ Yes, once a member |
| Edit own messages (kind:40003) | ✅ Yes |
| Delete own messages (kind:5) | ✅ Yes |
| Delete others' messages (kind:9005) | ❌ No — requires owner/admin |
| Add other members (kind:9000) | ❌ No — requires owner/admin |
| Remove members (kind:9001) | ❌ No — requires owner/admin |
| Edit channel metadata (kind:9002) | ❌ No (name/about/visibility) — requires owner/admin |
| Set topic/purpose (kind:9002) | ✅ Yes — any member |
| Archive channel (kind:9002, archived=true) | ❌ No — requires owner/admin |

### Key Event Kind Summary

| Kind | Name | Direction | Description |
|------|------|----------|-------------|
| 9000 | `KIND_NIP29_PUT_USER` | Client → Relay | Add member |
| 9001 | `KIND_NIP29_REMOVE_USER` | Client → Relay | Remove member |
| 9002 | `KIND_NIP29_EDIT_METADATA` | Client → Relay | Edit channel metadata |
| 9005 | `KIND_NIP29_DELETE_EVENT` | Client → Relay | Delete a message |
| 9007 | `KIND_NIP29_CREATE_GROUP` | Client → Relay | Create channel |
| 9008 | `KIND_NIP29_DELETE_GROUP` | Client → Relay | Delete channel |
| 9021 | `KIND_NIP29_JOIN_REQUEST` | Client → Relay | Self-join open channel |
| 9022 | `KIND_NIP29_LEAVE_REQUEST` | Client → Relay | Self-leave channel |
| 9 / 40002 | `KIND_STREAM_MESSAGE*` | Client → Relay | Post a message |
| 40099 | `KIND_SYSTEM_MESSAGE` | Relay → All | System event (join/leave/etc.) |
| 39000 | `KIND_NIP29_GROUP_METADATA` | Relay → Subscribers | Channel metadata snapshot |
| 39001 | `KIND_NIP29_GROUP_ADMINS` | Relay → Subscribers | Admin pubkey list |
| 39002 | `KIND_NIP29_GROUP_MEMBERS` | Relay → Subscribers | Full member list |
| 44100 | `KIND_MEMBER_ADDED_NOTIFICATION` | Relay → Target | You were added (relay-signed, p-tagged to you) |
| 44101 | `KIND_MEMBER_REMOVED_NOTIFICATION` | Relay → Target | You were removed (relay-signed, p-tagged to you) |
| 10100 | `KIND_AGENT_PROFILE` | Client → Relay | Agent metadata + `channel_add_policy` |

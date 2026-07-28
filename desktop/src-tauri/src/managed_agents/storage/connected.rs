//! The three-way partition of `managed-agents.json`, and store access for the
//! newest third: connected self-hosted agents under [`KeyCustody::Remote`].
//!
//! A child module of `storage` rather than more lines in `storage.rs`, which
//! already carries a documented "queued to be split" size override. As a child
//! it still reaches `storage`'s private `load_agent_store` and
//! `write_agent_store`, so the split cost no widening of visibility.
//!
//! The partition is the load-bearing part of key custody. `load_managed_agents`
//! — the reader behind spawn, deploy, auto-start restore, owner-signed
//! kind:30177 reconcile, profile republish, and delete-with-tombstone — filters
//! connected agents out at the source. Code that has never heard of key custody
//! therefore cannot act on an agent Buzz does not own, instead of every such
//! site needing a guard someone has to remember to add.

use tauri::AppHandle;

use super::{load_agent_store, write_agent_store};
use crate::managed_agents::ManagedAgentRecord;

/// Which of the three kinds of record a row in the store is.
///
/// An enum rather than three independent predicates so the partition is total
/// and disjoint by construction: [`store_half`] returns exactly one of these
/// for every record, and a future fourth kind forces every `match` to be
/// revisited instead of silently falling into a wrong bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StoreHalf {
    /// Key-less definition (a former persona), folded into this store. Keys are
    /// minted on first start.
    Definition,
    /// A self-hosted agent whose key lives on another machine. Buzz holds the
    /// public half only.
    Connected,
    /// An agent Buzz minted and holds the key for — the only kind any lifecycle
    /// path may act on.
    Owned,
}

/// Classify one record. The single definition of the partition, shared by every
/// reader and writer so they cannot drift into disagreeing about it.
pub(super) fn store_half(record: &ManagedAgentRecord) -> StoreHalf {
    if record.pubkey.is_empty() {
        // Checked first: a key-less record has no identity to have custody of,
        // and its `key_custody` is meaningless rather than merely default.
        StoreHalf::Definition
    } else if record.key_custody.is_local() {
        StoreHalf::Owned
    } else {
        StoreHalf::Connected
    }
}

/// Split out the halves an instance-side save must carry through untouched.
///
/// This is what keeps the dozens of existing `load … mutate …
/// save_managed_agents` call sites correct now that [`load_managed_agents`]
/// filters connected agents out: every one of them hands back a vector with no
/// connected records in it, so without re-reading them here the next unrelated
/// save would erase every connected agent.
///
/// [`load_managed_agents`]: super::load_managed_agents
pub(super) fn preserved_halves(
    existing: &[ManagedAgentRecord],
) -> (Vec<ManagedAgentRecord>, Vec<ManagedAgentRecord>) {
    let take = |half: StoreHalf| -> Vec<ManagedAgentRecord> {
        existing
            .iter()
            .filter(|record| store_half(record) == half)
            .cloned()
            .collect()
    };
    (take(StoreHalf::Definition), take(StoreHalf::Connected))
}

/// Sort connected records the way instances are sorted, for stable diffs.
fn sort_by_name_then_pubkey(records: &mut [ManagedAgentRecord]) {
    records.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.pubkey.cmp(&right.pubkey))
    });
}

/// Load the connected self-hosted agents.
///
/// No key hydration: there is no key to hydrate. A keyring lookup here would
/// query for a secret that by definition does not exist locally, and a miss
/// would be indistinguishable from an outage.
pub(crate) fn load_connected_agents(app: &AppHandle) -> Result<Vec<ManagedAgentRecord>, String> {
    let mut records = load_agent_store(app)?;
    records.retain(|record| store_half(record) == StoreHalf::Connected);
    Ok(records)
}

/// Save the connected self-hosted agents, preserving the definitions and the
/// owned instances — the custody-side mirror of `save_managed_agents`.
///
/// Never routed through `persist_agent_keys`: a connected record has no key,
/// and offering one to the keyring would create an entry Buzz would later
/// hydrate as if it owned the identity.
pub(crate) fn save_connected_agents(
    app: &AppHandle,
    connected: &[ManagedAgentRecord],
) -> Result<(), String> {
    let mut raw = load_agent_store(app)?;
    let definitions: Vec<ManagedAgentRecord> = raw
        .iter()
        .filter(|record| store_half(record) == StoreHalf::Definition)
        .cloned()
        .collect();
    raw.retain(|record| store_half(record) == StoreHalf::Owned);

    let mut connected = connected.to_vec();
    connected.retain(|record| store_half(record) == StoreHalf::Connected);
    sort_by_name_then_pubkey(&mut connected);

    write_agent_store(app, definitions, connected, raw)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{record_with_key, record_with_pubkey_and_key};
    use super::{preserved_halves, store_half, StoreHalf};
    use crate::managed_agents::{spawn_key_refusal, KeyCustody, ManagedAgentRecord};

    fn connected_record(pubkey: &str, host: &str) -> ManagedAgentRecord {
        let mut record = record_with_pubkey_and_key(pubkey, "");
        record.key_custody = KeyCustody::Remote {
            host: host.to_string(),
        };
        record
    }

    #[test]
    fn custody_is_the_only_thing_separating_the_two_keyed_halves() {
        // Both keyed records have a pubkey; one has an empty key because Buzz
        // never held it, the other because Buzz holds it in the keyring. Only
        // `key_custody` tells them apart, which is exactly why the field exists.
        let owned = record_with_pubkey_and_key("owned", "");
        let connected = connected_record("connected", "lunar02");
        let definition = record_with_pubkey_and_key("", "");

        assert_eq!(store_half(&owned), StoreHalf::Owned);
        assert_eq!(store_half(&connected), StoreHalf::Connected);
        assert_eq!(store_half(&definition), StoreHalf::Definition);
    }

    #[test]
    fn a_key_less_record_is_a_definition_regardless_of_custody() {
        // Defensive: a hand-edited store could carry a custody tag on a
        // key-less row. It has no identity to have custody of, so it must stay
        // a definition rather than becoming a connected agent with no pubkey.
        let mut record = record_with_pubkey_and_key("", "");
        record.key_custody = KeyCustody::Remote {
            host: "lunar02".to_string(),
        };
        assert_eq!(store_half(&record), StoreHalf::Definition);
    }

    #[test]
    fn an_instance_side_save_cannot_erase_connected_agents() {
        // The highest-risk regression in this change. Every existing call site
        // does `load_managed_agents` → mutate → `save_managed_agents`, and
        // `load_managed_agents` no longer returns connected records — so the
        // save path MUST re-read them. If `preserved_halves` ever stops
        // returning the connected third, dozens of unrelated saves start
        // silently deleting the user's connected agents.
        let store = vec![
            record_with_pubkey_and_key("", ""),
            connected_record("connected", "lunar02"),
            record_with_pubkey_and_key("owned", "nsec1real"),
        ];

        let (definitions, connected) = preserved_halves(&store);

        assert_eq!(definitions.len(), 1);
        assert_eq!(connected.len(), 1);
        assert_eq!(connected[0].pubkey, "connected");
        assert_eq!(
            connected[0].key_custody.remote_host(),
            Some("lunar02"),
            "the host must survive the re-read — it is the probe target"
        );
        // The owned third is supplied by the caller, never by this function;
        // returning it too would double it into the written store.
        assert!(
            definitions
                .iter()
                .chain(connected.iter())
                .all(|record| record.pubkey != "owned"),
            "preserved_halves must not return the caller's own third"
        );
    }

    #[test]
    fn remote_custody_refusal_states_the_fact_instead_of_blaming_the_keyring() {
        // Both cases have an empty `private_key_nsec`, so the ONLY thing that
        // distinguishes them is custody. Getting this wrong sends a user whose
        // setup is working correctly to go debug their keychain.
        let record = connected_record("connected", "lunar02");
        let refusal = spawn_key_refusal(&record).expect("must still refuse");
        assert!(refusal.contains("lunar02"));
        assert!(refusal.contains("self-hosted"));
        assert!(
            !refusal.to_lowercase().contains("keyring"),
            "custody refusal must not read as an outage: {refusal}"
        );

        let outage = record_with_key("");
        let outage_refusal = spawn_key_refusal(&outage).expect("must refuse");
        assert!(
            outage_refusal.contains("keyring"),
            "a genuine outage must still say so: {outage_refusal}"
        );
    }
}

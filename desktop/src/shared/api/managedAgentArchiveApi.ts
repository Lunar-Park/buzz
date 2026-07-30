import { invokeTauri } from "@/shared/api/tauri";
import type {
  ArchivedAgent,
  ManagedAgentRemovalPreview,
} from "@/shared/api/remoteAgentTypes";

/**
 * Two-stage removal for agents Buzz owns.
 *
 * `deleteManagedAgent` is irreversible in ways a dialog cannot undo — it destroys
 * the key, tombstones the directory record, and publishes a NIP-IA archive. These
 * put a reversible step in front of it and gate the irreversible half behind that
 * step.
 */

/** What `Remove from My Agents` would affect. Read-only. */
export async function describeManagedAgentRemoval(
  pubkey: string,
): Promise<ManagedAgentRemovalPreview> {
  return await invokeTauri<ManagedAgentRemovalPreview>(
    "describe_managed_agent_removal",
    { pubkey },
  );
}

/**
 * Stage one: stop the agent and hide it, keeping its identity.
 *
 * Reversible with `restoreArchivedAgent`. Publishes nothing.
 */
export async function archiveManagedAgent(
  pubkey: string,
): Promise<ArchivedAgent> {
  return await invokeTauri<ArchivedAgent>("archive_managed_agent", { pubkey });
}

/** The agents currently archived on this machine. */
export async function listArchivedAgents(): Promise<ArchivedAgent[]> {
  return await invokeTauri<ArchivedAgent[]>("list_archived_agents");
}

/**
 * Move an archived agent back into normal use, intact.
 *
 * Refuses on a name or pubkey collision rather than creating a duplicate.
 */
export async function restoreArchivedAgent(pubkey: string): Promise<void> {
  await invokeTauri<void>("restore_archived_agent", { pubkey });
}

/**
 * Stage two: destroy an archived agent's identity. Irreversible.
 *
 * Only available for an already-archived agent — the backend refuses an
 * unarchived pubkey, so "archive first" cannot be bypassed. Callers must state
 * that this cannot erase already-published events, relay audit history, or copies
 * other clients hold.
 */
export async function permanentlyDeleteArchivedAgent(
  pubkey: string,
): Promise<void> {
  await invokeTauri<void>("permanently_delete_archived_agent", { pubkey });
}

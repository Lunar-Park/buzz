import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  listArchivedAgents,
  permanentlyDeleteArchivedAgent,
  restoreArchivedAgent,
} from "@/shared/api/managedAgentArchiveApi";
import { managedAgentsQueryKey } from "@/features/agents/hooks";

export const archivedAgentsQueryKey = ["archived-agents"] as const;

/**
 * Agents removed from normal surfaces but still recoverable.
 *
 * No polling: an archived agent has no process and no relay presence, so its row
 * changes only when the user restores or permanently deletes one.
 */
export function useArchivedAgentsQuery() {
  return useQuery({
    queryKey: archivedAgentsQueryKey,
    queryFn: listArchivedAgents,
    staleTime: 30_000,
  });
}

/**
 * Restore and permanent-delete actions.
 *
 * Both invalidate the managed-agent list as well as this one: a restore adds a
 * row there, and a permanent delete removes an identity the rest of the app may
 * still be showing from cache.
 */
export function useArchivedAgentActions() {
  const queryClient = useQueryClient();

  async function invalidate() {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: archivedAgentsQueryKey }),
      queryClient.invalidateQueries({ queryKey: managedAgentsQueryKey }),
    ]);
  }

  const restore = useMutation({
    mutationFn: (pubkey: string) => restoreArchivedAgent(pubkey),
    onSuccess: invalidate,
  });

  const permanentlyDelete = useMutation({
    mutationFn: (pubkey: string) => permanentlyDeleteArchivedAgent(pubkey),
    onSuccess: invalidate,
  });

  return { permanentlyDelete, restore };
}

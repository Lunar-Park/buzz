import type { ConnectedAgent } from "@/shared/api/remoteAgentTypes";

/**
 * Normalize a relay URL so two spellings of one community compare equal.
 *
 * Mirrors the backend's `normalize_community_url`. Three spellings reach this
 * code — what the user typed, what the workspace override holds, and what a
 * record stored — and comparing raw strings would split one community into
 * several, hiding agents from the community they belong to.
 */
export function normalizeCommunityUrl(url: string): string {
  return url.trim().replace(/\/+$/, "").toLowerCase();
}

/**
 * The connected agents that belong in the community currently open.
 *
 * Buzz derives a community from its relay host, so a connected agent is only
 * meaningful inside the community whose relay its adapter listens to. Before this
 * filter one record appeared in every community the user had, including ones
 * where the agent's key cannot even authenticate — which is what made a
 * connection look ready when it could never work.
 *
 * A record with no community matches every community. Those predate the field:
 * Buzz cannot know which community they were made in, and hiding them would look
 * exactly like data loss to the user who connected them. They are shown until
 * the user reconnects them, which stamps the community.
 *
 * An unknown active community shows everything rather than nothing — an empty
 * list would read as "you have no agents" when the real state is "Buzz does not
 * know which community this is".
 */
export function connectedAgentsForCommunity(
  agents: ConnectedAgent[],
  activeRelayUrl: string | null | undefined,
): ConnectedAgent[] {
  if (!activeRelayUrl) return agents;
  const active = normalizeCommunityUrl(activeRelayUrl);
  return agents.filter((agent) => {
    if (!agent.community) return true;
    return normalizeCommunityUrl(agent.community) === active;
  });
}

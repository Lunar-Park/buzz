import type { ConnectedAgent } from "@/shared/api/remoteAgentTypes";
import type { AgentTeam } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export type ResolvedTeamConnectedAgents = {
  hasMissingConnectedAgents: boolean;
  missingConnectedAgentPubkeys: string[];
  resolvedConnectedAgents: ConnectedAgent[];
};

export function resolveTeamConnectedAgents(
  team: Pick<AgentTeam, "connectedAgentPubkeys">,
  connectedAgents: readonly ConnectedAgent[],
): ResolvedTeamConnectedAgents {
  const agentsByPubkey = new Map(
    connectedAgents.map((agent) => [normalizePubkey(agent.pubkey), agent]),
  );
  const resolvedConnectedAgents: ConnectedAgent[] = [];
  const missingConnectedAgentPubkeys: string[] = [];

  for (const candidate of team.connectedAgentPubkeys) {
    const pubkey = normalizePubkey(candidate);
    const agent = agentsByPubkey.get(pubkey);
    if (agent) {
      resolvedConnectedAgents.push(agent);
    } else {
      missingConnectedAgentPubkeys.push(pubkey);
    }
  }

  return {
    hasMissingConnectedAgents: missingConnectedAgentPubkeys.length > 0,
    missingConnectedAgentPubkeys,
    resolvedConnectedAgents,
  };
}

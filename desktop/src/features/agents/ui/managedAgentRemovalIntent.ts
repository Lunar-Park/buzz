import type {
  ArchivedAgent,
  ManagedAgentRemovalPreview,
} from "@/shared/api/remoteAgentTypes";

/**
 * Wording and gating for the two-stage removal of a Buzz-owned agent.
 *
 * Kept out of the components so what the user is told before an irreversible
 * action is directly testable, rather than being reviewed by reading JSX.
 */

/**
 * The things stage one will actually do, in plain terms.
 *
 * Only effects that are true for this agent appear. A dialog listing "stops the
 * agent" for something that is not running teaches the user to skim the list,
 * which is the opposite of what a confirmation is for.
 */
export function removalEffects(preview: ManagedAgentRemovalPreview): string[] {
  const effects: string[] = [];
  if (preview.isRunning) {
    effects.push("Stops the agent running on this machine.");
  }
  effects.push("Hides it from agent cards, pickers, and mentions.");
  if (preview.teamNames.length === 1) {
    effects.push(`Removes it from the ${preview.teamNames[0]} team.`);
  } else if (preview.teamNames.length > 1) {
    effects.push(
      `Removes it from ${preview.teamNames.length} teams: ${preview.teamNames.join(", ")}.`,
    );
  }
  if (preview.hasLocalKey) {
    effects.push("Keeps its identity and key, so you can restore it later.");
  }
  return effects;
}

/**
 * What stage one does *not* do.
 *
 * Stated because the previous single-step delete did all of it, so a user who
 * knows the old behaviour will assume this one destroys the identity too.
 */
export function removalReassurance(): string {
  return "Nothing is published to the relay and no key is destroyed. You can restore this agent from Archived agents.";
}

/**
 * The boundary a permanent delete cannot cross.
 *
 * The specification requires this be stated rather than implying the identity
 * disappears from the network: signed events are already out, and other clients
 * and the relay's audit trail keep their own copies.
 */
export function permanentDeleteBoundary(): string[] {
  return [
    "Destroys the private key Buzz holds for this identity. This cannot be undone.",
    "Removes its definition, configuration, and local team references.",
    "Publishes a tombstone so it stops appearing in relay pickers.",
    "Cannot erase messages it already sent, the relay's audit history, or copies other clients hold.",
  ];
}

/**
 * Extra stage-one effects for a provider-deployed agent.
 *
 * Archiving stops only what runs on this machine. The single-step delete
 * carried the remote-orphan warnings, so the stage-one confirmation must
 * carry them too — repointing the menu without moving these rules is how a
 * live deployment gets orphaned silently. Worded as an effect rather than a
 * blocking confirm because stage one is reversible: restoring the record
 * brings the deployment back under management.
 */
export function remoteDeploymentEffects(agent: {
  backend: { type: string };
  backendAgentId?: string | null;
}): string[] {
  if (agent.backend.type !== "provider" || !agent.backendAgentId) {
    return [];
  }
  return [
    "Does not stop its remote deployment — it keeps running on the provider until shut down there. Restore the agent to manage it again.",
  ];
}

/**
 * Whether permanent deletion is offered for this row.
 *
 * The backend refuses any pubkey that is not archived, so this only keeps the UI
 * from presenting an action that would fail — the gate itself lives there, not
 * here.
 */
export function canPermanentlyDelete(agent: ArchivedAgent): boolean {
  return agent.pubkey.length > 0;
}

/**
 * Label for the reversible removal action.
 *
 * One label for every Buzz-owned agent, built-in or not. The old menu called the
 * built-in path "Delete" while it actually deactivated a definition and left a
 * linked instance running, which is why built-in agents could not be removed
 * cleanly — the word promised something the action did not do.
 */
export const REMOVE_ACTION_LABEL = "Remove from My Agents";

/** Label for the irreversible action, only ever shown from the archived state. */
export const PERMANENT_DELETE_ACTION_LABEL = "Permanently delete identity";

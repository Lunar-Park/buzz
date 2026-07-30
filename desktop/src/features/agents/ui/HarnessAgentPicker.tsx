import { Loader2 } from "lucide-react";

import type { ConnectAgentDraft } from "./connectAgentIntent";
import {
  candidateLabel,
  rosterCandidates,
  rosterStatusMessage,
} from "./connectAgentIntent";

const SELECT_CLASS =
  "flex h-9 w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-xs";

/**
 * Pick which durable agent inside a harness this identity is.
 *
 * A resident harness commonly holds several named agents. The primary arrives
 * preselected and the rest stay visible but unselected, because connecting a
 * machine must not enroll its whole stack — the user picks one, and can come back
 * to add another later.
 *
 * Renders nothing until a harness is chosen: without one there is no roster to
 * ask for, and an empty control would imply Buzz had looked and found nothing.
 *
 * A harness Buzz cannot enumerate is reported as such rather than as a failure.
 * Manual identity entry is the intended path there, so the message points at the
 * field below instead of offering a retry for a problem that does not exist.
 */
export function HarnessAgentPicker({
  disabled,
  draft,
  onSelect,
}: {
  disabled: boolean;
  draft: ConnectAgentDraft;
  onSelect: (agentId: string) => void;
}) {
  if (!draft.harness) return null;

  if (draft.isLoadingRoster) {
    return (
      <p className="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="size-3.5 animate-spin" />
        Listing this harness&apos;s agents…
      </p>
    );
  }

  const candidates = rosterCandidates(draft.roster);
  const status = rosterStatusMessage(draft.roster);

  if (candidates.length === 0) {
    return status ? (
      <p className="text-sm text-muted-foreground">{status}</p>
    ) : null;
  }

  return (
    <div className="space-y-1.5">
      <label className="text-sm font-medium" htmlFor="connect-harness-agent">
        Agent on this harness
      </label>
      <select
        className={SELECT_CLASS}
        disabled={disabled}
        id="connect-harness-agent"
        onChange={(event) => onSelect(event.target.value)}
        value={draft.harnessAgentId}
      >
        <option value="">Not recorded</option>
        {candidates.map((candidate) => (
          <option key={candidate.agentId} value={candidate.agentId}>
            {candidateLabel(candidate)}
          </option>
        ))}
      </select>
      <p className="text-sm text-muted-foreground">
        Which agent on this machine this identity is. Only the one you pick is
        connected — the others stay untouched, and you can add another later.
      </p>
    </div>
  );
}

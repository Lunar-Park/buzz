import * as React from "react";
import { AlertTriangle, Loader2, RotateCcw, Trash2 } from "lucide-react";

import type { ArchivedAgent } from "@/shared/api/remoteAgentTypes";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";

import {
  canPermanentlyDelete,
  permanentDeleteBoundary,
  PERMANENT_DELETE_ACTION_LABEL,
} from "./managedAgentRemovalIntent";
import {
  useArchivedAgentActions,
  useArchivedAgentsQuery,
} from "./useArchivedAgents";

/**
 * Agents removed from normal surfaces but still recoverable — stage one's
 * destination, and the only place stage two can be reached from.
 *
 * Renders nothing when empty. An always-present "Archived agents (0)" heading
 * would give the agents view a permanent reminder of a state most users never
 * enter.
 */
export function ArchivedAgentsSection() {
  const query = useArchivedAgentsQuery();
  const { permanentlyDelete, restore } = useArchivedAgentActions();
  const [pendingDelete, setPendingDelete] =
    React.useState<ArchivedAgent | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  const agents = query.data ?? [];
  if (agents.length === 0) return null;

  return (
    <section className="space-y-3">
      <div className="space-y-1">
        <h3 className="text-sm font-medium">Archived agents</h3>
        <p className="text-sm text-muted-foreground">
          Removed from your agents but not destroyed. Buzz still holds their
          identities, so they can be restored.
        </p>
      </div>

      <ul className="space-y-2">
        {agents.map((agent) => (
          <li
            className="flex items-center justify-between gap-3 rounded-2xl border px-4 py-3"
            key={agent.pubkey}
          >
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">{agent.name}</p>
              <p className="text-2xs text-muted-foreground">
                Archived {agent.archivedAt}
              </p>
            </div>
            <div className="flex shrink-0 gap-2">
              <Button
                disabled={restore.isPending}
                onClick={() => {
                  setError(null);
                  restore.mutateAsync(agent.pubkey).catch((cause) => {
                    setError(
                      cause instanceof Error ? cause.message : String(cause),
                    );
                  });
                }}
                size="sm"
                type="button"
                variant="outline"
              >
                {restore.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <RotateCcw />
                )}
                Restore
              </Button>
              {canPermanentlyDelete(agent) ? (
                <Button
                  className="text-destructive hover:text-destructive"
                  onClick={() => setPendingDelete(agent)}
                  size="sm"
                  type="button"
                  variant="ghost"
                >
                  <Trash2 />
                  Delete
                </Button>
              ) : null}
            </div>
          </li>
        ))}
      </ul>

      {error ? <p className="text-sm text-destructive">{error}</p> : null}

      <PermanentDeleteDialog
        agent={pendingDelete}
        isPending={permanentlyDelete.isPending}
        onCancel={() => setPendingDelete(null)}
        onConfirm={(pubkey) => {
          setError(null);
          permanentlyDelete
            .mutateAsync(pubkey)
            .then(() => setPendingDelete(null))
            .catch((cause) => {
              setError(cause instanceof Error ? cause.message : String(cause));
              setPendingDelete(null);
            });
        }}
      />
    </section>
  );
}

/**
 * Stage two: the destructive confirmation, separate by design.
 *
 * An agent cannot go from visible to key-destroyed in one action — it must be
 * archived first, and the backend refuses any pubkey that is not. This dialog
 * spells out both what is destroyed and what cannot be: signed events are already
 * out, and the relay's audit trail and other clients keep their own copies.
 */
function PermanentDeleteDialog({
  agent,
  isPending,
  onCancel,
  onConfirm,
}: {
  agent: ArchivedAgent | null;
  isPending: boolean;
  onCancel: () => void;
  onConfirm: (pubkey: string) => void;
}) {
  return (
    <Dialog
      onOpenChange={(next) => (next ? undefined : onCancel())}
      open={Boolean(agent)}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangle className="size-4 text-destructive" />
            {PERMANENT_DELETE_ACTION_LABEL}
          </DialogTitle>
          <DialogDescription>
            {agent ? `${agent.name} cannot be recovered after this.` : null}
          </DialogDescription>
        </DialogHeader>

        <ul className="space-y-1.5 text-sm">
          {permanentDeleteBoundary().map((line) => (
            <li className="flex gap-2 text-muted-foreground" key={line}>
              <span aria-hidden="true">·</span>
              <span>{line}</span>
            </li>
          ))}
        </ul>

        <div className="flex justify-end gap-2">
          <Button
            disabled={isPending}
            onClick={onCancel}
            type="button"
            variant="ghost"
          >
            Keep it archived
          </Button>
          <Button
            disabled={isPending || !agent}
            onClick={() => {
              if (agent) onConfirm(agent.pubkey);
            }}
            type="button"
            variant="destructive"
          >
            {isPending ? <Loader2 className="animate-spin" /> : <Trash2 />}
            {PERMANENT_DELETE_ACTION_LABEL}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

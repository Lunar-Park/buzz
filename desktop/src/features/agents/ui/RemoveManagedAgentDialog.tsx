import * as React from "react";
import { Loader2 } from "lucide-react";

import { describeManagedAgentRemoval } from "@/shared/api/managedAgentArchiveApi";
import type { ManagedAgentRemovalPreview } from "@/shared/api/remoteAgentTypes";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";

import {
  REMOVE_ACTION_LABEL,
  removalEffects,
  removalReassurance,
} from "./managedAgentRemovalIntent";

/**
 * Stage one of removing an agent Buzz owns: reversible, and specific about it.
 *
 * The dialog loads what the removal would actually affect before offering the
 * button, because "are you sure?" is not answerable without knowing whether the
 * agent is running and which teams lose a member. Only effects that are true for
 * this agent are listed.
 *
 * It also states what stage one does *not* do. The previous single-step delete
 * destroyed the key and published a tombstone, so a user who knows that
 * behaviour will reasonably assume this does too.
 */
export function RemoveManagedAgentDialog({
  agentName,
  agentPubkey,
  extraEffects = [],
  onConfirmRemoval,
  onOpenChange,
  onRemoved,
  open,
}: {
  agentName: string;
  agentPubkey: string;
  /**
   * Caller-known effects the backend preview cannot see (e.g. a provider
   * deployment that keeps running). Appended to the effects list verbatim.
   */
  extraEffects?: string[];
  /**
   * Performs the removal. The caller owns the cascade (channel removal,
   * query invalidation) so this dialog cannot drift from the real action.
   */
  onConfirmRemoval: () => Promise<void>;
  onOpenChange: (open: boolean) => void;
  onRemoved?: () => void;
  open: boolean;
}) {
  const [preview, setPreview] =
    React.useState<ManagedAgentRemovalPreview | null>(null);
  const [isLoading, setIsLoading] = React.useState(false);
  const [isRemoving, setIsRemoving] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setIsLoading(true);
    setError(null);
    setPreview(null);
    void describeManagedAgentRemoval(agentPubkey)
      .then((result) => {
        if (!cancelled) setPreview(result);
      })
      .catch((cause) => {
        if (!cancelled)
          setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [agentPubkey, open]);

  async function remove() {
    setIsRemoving(true);
    setError(null);
    try {
      await onConfirmRemoval();
      onRemoved?.();
      onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsRemoving(false);
    }
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {REMOVE_ACTION_LABEL}: {preview?.name || agentName}
          </DialogTitle>
          <DialogDescription>{removalReassurance()}</DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          {isLoading ? (
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-3.5 animate-spin" />
              Checking what this affects…
            </p>
          ) : preview ? (
            <ul className="space-y-1.5 text-sm">
              {[...removalEffects(preview), ...extraEffects].map((effect) => (
                <li className="flex gap-2 text-muted-foreground" key={effect}>
                  <span aria-hidden="true">·</span>
                  <span>{effect}</span>
                </li>
              ))}
            </ul>
          ) : null}

          {error ? (
            <p className="rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {error}
            </p>
          ) : null}
        </div>

        <div className="flex justify-end gap-2">
          <Button
            disabled={isRemoving}
            onClick={() => onOpenChange(false)}
            type="button"
            variant="ghost"
          >
            Cancel
          </Button>
          <Button
            disabled={isRemoving || isLoading || !preview}
            onClick={() => {
              void remove();
            }}
            type="button"
          >
            {isRemoving ? <Loader2 className="animate-spin" /> : null}
            {REMOVE_ACTION_LABEL}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

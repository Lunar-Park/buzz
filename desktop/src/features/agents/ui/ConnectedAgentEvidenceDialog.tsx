import * as React from "react";
import { Check, Copy, KeyRound, Loader2 } from "lucide-react";

import {
  getConnectedAgentOwnerEvidence,
  mintConnectedAgentOwnerEvidence,
} from "@/shared/api/remoteAgentApi";
import type { ConnectedAgentOwnerEvidence } from "@/shared/api/remoteAgentTypes";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";

/**
 * Issue and hand over the owner attestation that makes a connected agent
 * addressable.
 *
 * The relay refuses to let an owner add its own connected agent to a channel
 * while that agent publishes `owner_only` and no owner has been materialized for
 * it, and a membership-required relay refuses the agent's key outright. A NIP-OA
 * attestation fixes both, but only once it reaches the agent's *host* — so the
 * job of this dialog is to produce the value and make it easy to install.
 *
 * Buzz cannot install it. Writing the adapter's configuration would mean editing
 * a machine Buzz does not administer, which is the same boundary that keeps it
 * from starting or stopping the agent.
 *
 * Existing evidence is read, not re-minted, on open. Re-minting is valid but
 * BIP-340 signing is randomized, so a fresh tag differs from the one already on
 * the host and a user comparing the two could not tell that from a mismatch.
 */
export function ConnectedAgentEvidenceDialog({
  agentName,
  agentPubkey,
  onIssued,
  onOpenChange,
  open,
}: {
  agentName: string;
  agentPubkey: string;
  onIssued?: () => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
}) {
  const [evidence, setEvidence] =
    React.useState<ConnectedAgentOwnerEvidence | null>(null);
  const [isLoading, setIsLoading] = React.useState(false);
  const [isMinting, setIsMinting] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [copied, setCopied] = React.useState(false);

  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setIsLoading(true);
    setError(null);
    setCopied(false);
    void getConnectedAgentOwnerEvidence(agentPubkey)
      .then((result) => {
        if (cancelled) return;
        setEvidence(result);
      })
      .catch((cause) => {
        if (cancelled) return;
        setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [agentPubkey, open]);

  async function issue() {
    setIsMinting(true);
    setError(null);
    try {
      setEvidence(await mintConnectedAgentOwnerEvidence(agentPubkey));
      setCopied(false);
      onIssued?.();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsMinting(false);
    }
  }

  async function copy() {
    if (!evidence) return;
    await navigator.clipboard.writeText(evidence.authTag);
    setCopied(true);
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Owner attestation for {agentName}</DialogTitle>
          <DialogDescription>
            Proves to the relay that you own this agent, so you can add it to
            channels and it can reach a members-only relay. Install it on the
            agent&apos;s machine — Buzz cannot write configuration there.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {isLoading ? (
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-3.5 animate-spin" />
              Checking for an existing attestation…
            </p>
          ) : evidence ? (
            <>
              <div className="space-y-1.5">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-sm font-medium">Auth tag</span>
                  <Button
                    onClick={() => {
                      void copy();
                    }}
                    size="sm"
                    type="button"
                    variant="outline"
                  >
                    {copied ? <Check /> : <Copy />}
                    {copied ? "Copied" : "Copy"}
                  </Button>
                </div>
                <p className="max-h-32 overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-2xs break-all">
                  {evidence.authTag}
                </p>
                <p className="text-sm text-muted-foreground">
                  Issued {evidence.issuedAt}. Not a secret on its own — it only
                  works alongside this agent&apos;s own private key, which stays
                  on its machine.
                </p>
              </div>

              <div className="space-y-1.5">
                <span className="text-sm font-medium">Install it</span>
                <p className="text-sm text-muted-foreground">
                  Set it as the agent&apos;s Buzz auth tag on its host: the
                  environment variable{" "}
                  <span className="font-mono">BUZZ_AUTH_TAG</span>, or{" "}
                  <span className="font-mono">channels.buzz.authTag</span> for
                  the OpenClaw Buzz channel. It takes effect the next time the
                  agent authenticates to the relay.
                </p>
              </div>
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              No attestation has been issued for this agent yet.
            </p>
          )}

          {error ? (
            <p className="rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {error}
            </p>
          ) : null}
        </div>

        <div className="flex justify-end gap-2">
          <Button
            onClick={() => onOpenChange(false)}
            type="button"
            variant="ghost"
          >
            Close
          </Button>
          <Button
            disabled={isMinting || isLoading}
            onClick={() => {
              void issue();
            }}
            type="button"
          >
            {isMinting ? <Loader2 className="animate-spin" /> : <KeyRound />}
            {evidence ? "Issue a new one" : "Issue attestation"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

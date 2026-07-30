import * as React from "react";
import { Check, KeyRound, Loader2, Sparkles } from "lucide-react";

import {
  generateHostAgentIdentity,
  resolveHostAgentIdentity,
} from "@/shared/api/remoteAgentApi";
import type {
  GeneratedHostIdentity,
  HostIdentityResolution,
} from "@/shared/api/remoteAgentTypes";
import { Button } from "@/shared/ui/button";

import { identityOffer } from "./connectAgentIntent";

/**
 * Offer the agent's identity instead of asking the user to go and find it.
 *
 * Three outcomes, each with a different next action:
 *
 * - **Already configured.** The harness reports the identity it signs as; the
 *   user confirms it with one click rather than transcribing an npub.
 * - **None yet.** Generation is offered — but as a two-step confirmation that
 *   names the machine, because it is the only action in this dialog that writes
 *   to someone else's computer.
 * - **Buzz cannot ask.** Reported plainly, with manual entry below unchanged.
 *
 * The secret never comes here. Generation returns a public key and the path the
 * host wrote its secret to; that path is shown so the harness can be pointed at
 * it.
 */
export function HostIdentityField({
  agentId,
  disabled,
  harness,
  hasRosterSelection,
  host,
  isPrimarySelected,
  onUseIdentity,
}: {
  agentId: string;
  disabled: boolean;
  harness: string;
  /** Whether a specific roster agent is selected, rather than none. */
  hasRosterSelection: boolean;
  host: string;
  /**
   * Whether that selection is the harness's primary agent.
   *
   * Load-bearing: a harness reports one configured Buzz identity, belonging to
   * its primary. Offering it for a sibling would connect that agent under the
   * primary's key.
   */
  isPrimarySelected: boolean;
  onUseIdentity: (pubkey: string) => void;
}) {
  const [resolution, setResolution] =
    React.useState<HostIdentityResolution | null>(null);
  const [isResolving, setIsResolving] = React.useState(false);
  const [generated, setGenerated] =
    React.useState<GeneratedHostIdentity | null>(null);
  const [isGenerating, setIsGenerating] = React.useState(false);
  const [confirming, setConfirming] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!host || !harness) {
      setResolution(null);
      return;
    }
    let cancelled = false;
    setIsResolving(true);
    setError(null);
    setGenerated(null);
    setConfirming(false);
    void resolveHostAgentIdentity(host, harness)
      .then((result) => {
        if (!cancelled) setResolution(result);
      })
      .catch(() => {
        // A resolution failure is not worth an error banner: manual entry below
        // still works, and the field is an accelerator, not a requirement.
        if (!cancelled) setResolution(null);
      })
      .finally(() => {
        if (!cancelled) setIsResolving(false);
      });
    return () => {
      cancelled = true;
    };
  }, [harness, host]);

  async function generate() {
    setIsGenerating(true);
    setError(null);
    try {
      const result = await generateHostAgentIdentity(host, agentId || "main");
      setGenerated(result);
      setConfirming(false);
      onUseIdentity(result.pubkey);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setConfirming(false);
    } finally {
      setIsGenerating(false);
    }
  }

  if (!host || !harness) return null;

  const offer = identityOffer({
    hasRosterSelection,
    isPrimarySelected,
    resolvedPubkey: resolution?.pubkey ?? null,
    supported: Boolean(resolution?.supported),
  });

  if (isResolving) {
    return (
      <p className="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="size-3.5 animate-spin" />
        Looking for an existing identity on {host}…
      </p>
    );
  }

  return (
    <div className="space-y-2 rounded-2xl border bg-muted/30 px-4 py-3">
      {generated ? (
        <div className="space-y-1">
          <p className="flex items-center gap-2 text-sm font-medium">
            <Check className="size-3.5" />
            Identity created on {generated.host}
          </p>
          <p className="font-mono text-2xs break-all text-muted-foreground">
            {generated.npub || generated.pubkey}
          </p>
          <p className="text-sm text-muted-foreground">
            Its secret key stays on {generated.host} at{" "}
            <span className="font-mono">{generated.secretKeyPath}</span>. Point
            the agent&apos;s harness at that file.
          </p>
        </div>
      ) : offer === "use-resolved" && resolution?.pubkey ? (
        <div className="space-y-2">
          <p className="text-sm font-medium">Identity found on {host}</p>
          <p className="font-mono text-2xs break-all text-muted-foreground">
            {resolution.pubkey}
          </p>
          <Button
            disabled={disabled}
            onClick={() => onUseIdentity(resolution.pubkey ?? "")}
            size="sm"
            type="button"
            variant="outline"
          >
            <KeyRound />
            Use this identity
          </Button>
        </div>
      ) : resolution?.supported ? (
        <div className="space-y-2">
          <p className="text-sm font-medium">No Buzz identity on {host} yet</p>
          {confirming ? (
            <>
              <p className="text-sm text-muted-foreground">
                This creates a keypair <strong>on {host}</strong>. The secret is
                written there with owner-only permissions and never comes back
                to this machine.
              </p>
              <div className="flex gap-2">
                <Button
                  disabled={isGenerating || disabled}
                  onClick={() => {
                    void generate();
                  }}
                  size="sm"
                  type="button"
                >
                  {isGenerating ? (
                    <Loader2 className="animate-spin" />
                  ) : (
                    <Sparkles />
                  )}
                  Generate on {host}
                </Button>
                <Button
                  disabled={isGenerating}
                  onClick={() => setConfirming(false)}
                  size="sm"
                  type="button"
                  variant="ghost"
                >
                  Cancel
                </Button>
              </div>
            </>
          ) : (
            <Button
              disabled={disabled}
              onClick={() => setConfirming(true)}
              size="sm"
              type="button"
              variant="outline"
            >
              <Sparkles />
              Generate one on {host}
            </Button>
          )}
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">
          Buzz cannot read this harness&apos;s identity — paste the agent&apos;s
          public key below. Run <span className="font-mono">buzz users me</span>{" "}
          on {host} to read it.
        </p>
      )}

      {error ? <p className="text-sm text-destructive">{error}</p> : null}
    </div>
  );
}

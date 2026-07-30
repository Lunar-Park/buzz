import * as React from "react";
import {
  EllipsisVertical,
  KeyRound,
  Loader2,
  RefreshCw,
  Unplug,
  UserPlus,
} from "lucide-react";

import { ConnectedAgentEvidenceDialog } from "./ConnectedAgentEvidenceDialog";
import { useUserProfileQuery } from "@/features/profile/hooks";
import { probeAgentHost } from "@/shared/api/remoteAgentApi";
import type {
  ConnectedAgent,
  HostProbeResult,
} from "@/shared/api/remoteAgentTypes";
import { Badge } from "@/shared/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { AgentIdentityCard } from "./AgentIdentityCard";
import { reachabilityLabel } from "./connectAgentIntent";

/**
 * A self-hosted identity presented in the same library as managed agents.
 *
 * The card deliberately exposes no lifecycle controls: Buzz can open the
 * relay profile, attach membership, check the configured host, or forget its
 * local connection record. The remote process and private key remain external.
 */
export function ConnectedAgentCard({
  agent,
  isPending,
  onAddToChannel,
  onDisconnect,
  onOpenProfile,
}: {
  agent: ConnectedAgent;
  isPending: boolean;
  onAddToChannel: () => void;
  onDisconnect: () => void;
  onOpenProfile: () => void;
}) {
  const profileQuery = useUserProfileQuery(agent.pubkey);
  const [probe, setProbe] = React.useState<HostProbeResult | "pending">();

  const checkHost = React.useCallback(() => {
    setProbe("pending");
    void probeAgentHost(agent.host)
      .then(setProbe)
      .catch((cause) => {
        setProbe({
          host: agent.host,
          ok: false,
          durationMs: 0,
          error: cause instanceof Error ? cause.message : String(cause),
          harnesses: [],
        });
      });
  }, [agent.host]);

  const [evidenceOpen, setEvidenceOpen] = React.useState(false);

  const harnessLabel = agent.harness?.trim() || "Self-hosted";
  // The harness agent id earns a place in the subtitle only when it adds
  // information: for a single-agent harness it repeats the harness name.
  const agentIdLabel = agent.harnessAgentId?.trim();

  return (
    <>
      <AgentIdentityCard
        actions={
          <DropdownMenu modal={false}>
            <DropdownMenuTrigger asChild>
              <button
                aria-label={`${agent.name} connected agent actions`}
                className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                type="button"
              >
                <EllipsisVertical className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="end"
              onCloseAutoFocus={(event) => event.preventDefault()}
            >
              <DropdownMenuItem disabled={isPending} onClick={onAddToChannel}>
                <UserPlus className="h-4 w-4" />
                Add to channel
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setEvidenceOpen(true)}>
                <KeyRound className="h-4 w-4" />
                {agent.hasOwnerEvidence
                  ? "Owner attestation"
                  : "Issue owner attestation"}
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={probe === "pending"}
                onClick={checkHost}
              >
                {probe === "pending" ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <RefreshCw className="h-4 w-4" />
                )}
                Check host
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                className="text-destructive focus:text-destructive"
                disabled={isPending}
                onClick={onDisconnect}
              >
                <Unplug className="h-4 w-4" />
                Disconnect
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        }
        ariaLabel={`${agent.name} connected agent profile`}
        avatarUrl={profileQuery.data?.avatarUrl}
        dataTestId={`connected-agent-${agent.pubkey}`}
        label={profileQuery.data?.displayName?.trim() || agent.name}
        modelLabel={
          agentIdLabel
            ? `${harnessLabel} · ${agentIdLabel} · ${agent.host}`
            : `${harnessLabel} · ${agent.host}`
        }
        onClick={onOpenProfile}
        statusBadge={<ConnectedAgentStatus probe={probe} />}
      />
      <ConnectedAgentEvidenceDialog
        agentName={agent.name}
        agentPubkey={agent.pubkey}
        onOpenChange={setEvidenceOpen}
        open={evidenceOpen}
      />
    </>
  );
}

function ConnectedAgentStatus({
  probe,
}: {
  probe: HostProbeResult | "pending" | undefined;
}) {
  if (!probe) {
    return <Badge variant="secondary">Connected identity</Badge>;
  }

  if (probe === "pending") {
    return (
      <Badge className="gap-1" variant="secondary">
        <Loader2 className="h-3 w-3 animate-spin" />
        Checking host
      </Badge>
    );
  }

  return (
    <Badge variant={probe.ok ? "success" : "warning"}>
      {reachabilityLabel(probe)}
    </Badge>
  );
}

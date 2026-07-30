import { invokeTauri } from "@/shared/api/tauri";
import type {
  ConnectedAgent,
  ConnectedAgentOwnerEvidence,
  HarnessRosterResult,
  HostProbeResult,
  SshHost,
} from "@/shared/api/remoteAgentTypes";

/**
 * Enumerate the user's `~/.ssh/config` host aliases. No connection is attempted;
 * an absent config yields an empty list.
 */
export async function listSshHosts(): Promise<SshHost[]> {
  return await invokeTauri<SshHost[]>("list_ssh_hosts");
}

/**
 * Probe one configured host for agent harnesses and the `buzz` CLI.
 *
 * `host` must be an alias present in `~/.ssh/config`. A host-side failure
 * (unreachable, password-only, unknown key) resolves with `ok: false` and a
 * classified `errorKind`; only a failure to run `ssh` at all rejects.
 */
export async function probeAgentHost(host: string): Promise<HostProbeResult> {
  return await invokeTauri<HostProbeResult>("probe_agent_host", { host });
}

/**
 * Probe the machine Buzz is running on, using the identical probe script so the
 * result is shape-compatible with `probeAgentHost`.
 */
export async function probeLocalAgentHost(): Promise<HostProbeResult> {
  return await invokeTauri<HostProbeResult>("probe_local_agent_host");
}

/**
 * List the durable, named agents one harness holds on a host.
 *
 * The step after `probeAgentHost`: discovery says a harness is present, this
 * says which agents it contains and which is its primary, so the user can
 * enroll one, several, or none rather than the whole stack.
 *
 * Read-only — listing a roster starts nothing and changes no harness state. A
 * harness Buzz cannot enumerate resolves with `supported: false`, which is a
 * prompt for manual identity entry, not an error.
 */
export async function probeHarnessAgents(
  host: string,
  harness: string,
): Promise<HarnessRosterResult> {
  return await invokeTauri<HarnessRosterResult>("probe_harness_agents", {
    host,
    harness,
  });
}

/**
 * List the durable agents of a harness on this machine, shape-compatible with
 * `probeHarnessAgents`.
 */
export async function probeLocalHarnessAgentRoster(
  harness: string,
): Promise<HarnessRosterResult> {
  return await invokeTauri<HarnessRosterResult>(
    "probe_local_harness_agent_roster",
    { harness },
  );
}

/** The self-hosted agents this machine is connected to. */
export async function listConnectedAgents(): Promise<ConnectedAgent[]> {
  return await invokeTauri<ConnectedAgent[]>("list_connected_agents");
}

/**
 * Record a self-hosted agent that already runs on `host`.
 *
 * `pubkey` accepts an npub or 64 hex characters and is normalized to hex by the
 * backend. An nsec is refused with a specific message — a self-hosted agent's
 * secret must never leave its own machine, and this call never transports one.
 * `host` must be an alias present in `~/.ssh/config`, because it is also the
 * reachability probe target.
 */
export async function connectRemoteAgent(input: {
  host: string;
  pubkey: string;
  name: string;
  harness?: string | null;
  /**
   * Which durable agent inside that harness this identity is, e.g. `"main"`.
   * The harness's own routing key, so the Buzz-pubkey-to-harness-agent mapping is
   * stored rather than implied.
   */
  harnessAgentId?: string | null;
}): Promise<ConnectedAgent> {
  return await invokeTauri<ConnectedAgent>("connect_remote_agent", {
    host: input.host,
    pubkey: input.pubkey,
    name: input.name,
    harness: input.harness ?? null,
    harnessAgentId: input.harnessAgentId ?? null,
  });
}

/**
 * Issue a NIP-OA owner attestation for a connected agent.
 *
 * Sign locally with the owner identity and return the tag for the user to install
 * on the agent's host (`BUZZ_AUTH_TAG`, or `channels.buzz.authTag` for the
 * OpenClaw plugin). Without it, an agent publishing `owner_only` cannot be added
 * to a channel by its own owner, and a membership-required relay refuses its key
 * outright.
 *
 * Signs but publishes nothing: the attestation only takes effect through the
 * agent's own later events. The owner's secret never leaves this machine. Only
 * agents with a connection record can be attested to — this is not a
 * general-purpose owner signing oracle.
 */
export async function mintConnectedAgentOwnerEvidence(
  pubkey: string,
): Promise<ConnectedAgentOwnerEvidence> {
  return await invokeTauri<ConnectedAgentOwnerEvidence>(
    "mint_connected_agent_owner_evidence",
    { pubkey },
  );
}

/**
 * Re-read an attestation Buzz already issued, or `null` if there is none.
 *
 * Separate from minting so re-showing the value does not sign a new one:
 * re-minting is valid but yields a different signature, which a user comparing it
 * against the tag already on the host cannot distinguish from a mismatch.
 */
export async function getConnectedAgentOwnerEvidence(
  pubkey: string,
): Promise<ConnectedAgentOwnerEvidence | null> {
  return await invokeTauri<ConnectedAgentOwnerEvidence | null>(
    "get_connected_agent_owner_evidence",
    { pubkey },
  );
}

/**
 * Forget a connected agent.
 *
 * Local-only: this removes Buzz's pointer and nothing else. The remote process
 * keeps running, and no tombstone or archive event is published — Buzz never
 * claimed to own this agent, so it has nothing to revoke.
 */
export async function disconnectRemoteAgent(pubkey: string): Promise<void> {
  await invokeTauri<void>("disconnect_remote_agent", { pubkey });
}

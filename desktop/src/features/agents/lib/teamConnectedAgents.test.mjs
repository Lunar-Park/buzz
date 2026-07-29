import assert from "node:assert/strict";
import test from "node:test";

import { resolveTeamConnectedAgents } from "./teamConnectedAgents.ts";

const SELENE =
  "4687f50de3a9e235e28eb58d68b0746062d7be6401bbf78a766bbd6f96ffe3c9";

test("resolves connected team members by normalized public key", () => {
  const result = resolveTeamConnectedAgents(
    { connectedAgentPubkeys: [SELENE.toUpperCase()] },
    [
      {
        pubkey: SELENE,
        name: "Selene",
        host: "lunar01",
        harness: "openclaw",
        createdAt: "2026-07-29T00:00:00Z",
        updatedAt: "2026-07-29T00:00:00Z",
      },
    ],
  );

  assert.deepEqual(
    result.resolvedConnectedAgents.map((agent) => agent.name),
    ["Selene"],
  );
  assert.deepEqual(result.missingConnectedAgentPubkeys, []);
});

test("reports connected public keys not configured on this device", () => {
  const result = resolveTeamConnectedAgents(
    { connectedAgentPubkeys: [SELENE] },
    [],
  );

  assert.equal(result.hasMissingConnectedAgents, true);
  assert.deepEqual(result.missingConnectedAgentPubkeys, [SELENE]);
});

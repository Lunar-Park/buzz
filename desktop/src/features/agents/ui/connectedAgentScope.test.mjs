import assert from "node:assert/strict";
import test from "node:test";

import {
  connectedAgentsForCommunity,
  normalizeCommunityUrl,
} from "./connectedAgentScope.ts";

const LUNAR01 = "ws://lunar01:3000";
const HOSTED = "wss://lunarpark.communities.buzz.xyz";

function agent(overrides = {}) {
  return {
    pubkey: "4687f50de3a9e235e28eb58d68b0746062d7be6401bbf78a766bbd6f96ffe3c9",
    name: "Selene",
    host: "lunar01",
    harness: "openclaw",
    harnessAgentId: "main",
    community: LUNAR01,
    createdAt: "2026-07-29T00:00:00Z",
    updatedAt: "2026-07-29T00:00:00Z",
    hasOwnerEvidence: false,
    ...overrides,
  };
}

test("an agent shows in the community it was connected in", () => {
  assert.equal(connectedAgentsForCommunity([agent()], LUNAR01).length, 1);
});

test("an agent does not show in a different community", () => {
  // The bug this closes: one record appeared everywhere, including communities
  // where its key cannot authenticate at all.
  assert.deepEqual(connectedAgentsForCommunity([agent()], HOSTED), []);
});

test("a legacy record with no community shows everywhere", () => {
  // It predates the field. Hiding it would look like data loss to the user who
  // connected it.
  const legacy = agent({ community: undefined });
  assert.equal(connectedAgentsForCommunity([legacy], LUNAR01).length, 1);
  assert.equal(connectedAgentsForCommunity([legacy], HOSTED).length, 1);
});

test("scoping ignores trailing slashes and case", () => {
  assert.equal(
    connectedAgentsForCommunity(
      [agent({ community: "WS://Lunar01:3000/" })],
      LUNAR01,
    ).length,
    1,
  );
  assert.equal(
    normalizeCommunityUrl("  wss://Relay.Example.com// "),
    "wss://relay.example.com",
  );
});

test("an unknown active community shows everything rather than nothing", () => {
  // An empty list would read as "you have no agents" when the real state is
  // "Buzz does not know which community this is".
  assert.equal(connectedAgentsForCommunity([agent()], null).length, 1);
  assert.equal(connectedAgentsForCommunity([agent()], "").length, 1);
});

test("mixed communities are separated", () => {
  const agents = [
    agent({ pubkey: "a".repeat(64), community: LUNAR01 }),
    agent({ pubkey: "b".repeat(64), community: HOSTED }),
    agent({ pubkey: "c".repeat(64), community: undefined }),
  ];
  assert.deepEqual(
    connectedAgentsForCommunity(agents, LUNAR01).map((a) => a.pubkey),
    ["a".repeat(64), "c".repeat(64)],
  );
});

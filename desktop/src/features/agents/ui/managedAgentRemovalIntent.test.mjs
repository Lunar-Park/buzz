import assert from "node:assert/strict";
import test from "node:test";

import {
  canPermanentlyDelete,
  permanentDeleteBoundary,
  removalEffects,
  removalReassurance,
  remoteDeploymentEffects,
  PERMANENT_DELETE_ACTION_LABEL,
  REMOVE_ACTION_LABEL,
} from "./managedAgentRemovalIntent.ts";

function preview(overrides = {}) {
  return {
    pubkey: "a".repeat(64),
    name: "Fizz",
    isRunning: false,
    teamNames: [],
    hasLocalKey: true,
    ...overrides,
  };
}

test("a stopped agent is not described as being stopped", () => {
  // Listing an effect that will not happen teaches the user to skim the list,
  // which defeats the point of a confirmation.
  const effects = removalEffects(preview({ isRunning: false }));
  assert.ok(!effects.some((line) => /Stops the agent/.test(line)));
});

test("a running agent is told it will be stopped", () => {
  const effects = removalEffects(preview({ isRunning: true }));
  assert.ok(effects.some((line) => /Stops the agent/.test(line)));
});

test("one team is named, several are counted and named", () => {
  assert.ok(
    removalEffects(preview({ teamNames: ["Welcome Team"] })).some((line) =>
      line.includes("the Welcome Team team"),
    ),
  );
  const many = removalEffects(preview({ teamNames: ["Alpha", "Beta"] }));
  assert.ok(many.some((line) => line.includes("2 teams: Alpha, Beta")));
});

test("no team reference is mentioned when there are none", () => {
  const effects = removalEffects(preview({ teamNames: [] }));
  assert.ok(!effects.some((line) => /team/i.test(line)));
});

test("retaining the identity is stated, because the old delete destroyed it", () => {
  const effects = removalEffects(preview({ hasLocalKey: true }));
  assert.ok(effects.some((line) => /Keeps its identity/.test(line)));
  const keyless = removalEffects(preview({ hasLocalKey: false }));
  assert.ok(!keyless.some((line) => /Keeps its identity/.test(line)));
});

test("hiding from mentions is always stated", () => {
  assert.ok(
    removalEffects(preview()).some((line) =>
      /pickers, and mentions/.test(line),
    ),
  );
});

test("stage one says what it does not do", () => {
  const text = removalReassurance();
  assert.match(text, /Nothing is published/);
  assert.match(text, /no key is destroyed/);
  assert.match(text, /restore/i);
});

test("the permanent-delete boundary names what cannot be erased", () => {
  // Required by the specification: implying the identity disappears from the
  // network would be a false promise.
  const lines = permanentDeleteBoundary();
  assert.ok(lines.some((line) => /cannot be undone/i.test(line)));
  assert.ok(
    lines.some((line) => /Cannot erase messages it already sent/.test(line)),
  );
  assert.ok(lines.some((line) => /audit history/.test(line)));
  assert.ok(lines.some((line) => /other clients hold/.test(line)));
});

test("permanent delete is offered only for a real identity", () => {
  assert.ok(canPermanentlyDelete({ pubkey: "a".repeat(64) }));
  assert.ok(!canPermanentlyDelete({ pubkey: "" }));
});

test("a provider deployment is told it keeps running", () => {
  // The single-step delete carried these warnings; stage one must too, or a
  // live deployment is orphaned silently the first time the new path is used.
  const effects = remoteDeploymentEffects({
    backend: { type: "provider" },
    backendAgentId: "deploy-1",
  });
  assert.equal(effects.length, 1);
  assert.match(effects[0], /keeps running/);
  assert.match(effects[0], /Restore/);
});

test("local agents get no remote-deployment warning", () => {
  assert.deepEqual(
    remoteDeploymentEffects({
      backend: { type: "local" },
      backendAgentId: "x",
    }),
    [],
  );
  assert.deepEqual(
    remoteDeploymentEffects({
      backend: { type: "provider" },
      backendAgentId: null,
    }),
    [],
  );
});

test("the reversible action never uses the word Delete", () => {
  // The old menu labelled a deactivation "Delete", which is why built-in agents
  // could not be removed cleanly — the word promised more than the action did.
  assert.equal(REMOVE_ACTION_LABEL, "Remove from My Agents");
  assert.ok(!/delete/i.test(REMOVE_ACTION_LABEL));
  assert.match(PERMANENT_DELETE_ACTION_LABEL, /Permanently delete/);
});

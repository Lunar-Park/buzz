import assert from "node:assert/strict";
import test from "node:test";

import { welcomeExperienceEnabled } from "./forkGates.ts";

test("welcome experience is disabled by default", () => {
  assert.equal(welcomeExperienceEnabled(undefined), false);
  assert.equal(welcomeExperienceEnabled(""), false);
  assert.equal(welcomeExperienceEnabled("0"), false);
  assert.equal(welcomeExperienceEnabled("false"), false);
});

test("welcome experience enables on explicit opt-in", () => {
  assert.equal(welcomeExperienceEnabled("1"), true);
  assert.equal(welcomeExperienceEnabled("true"), true);
  assert.equal(welcomeExperienceEnabled(" 1 "), true);
});

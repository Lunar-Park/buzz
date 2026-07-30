/**
 * Screenshot spec for the P2 two-stage removal surfaces:
 *   1. The agent card menu's reversible "Remove from My Agents" item.
 *   2. The stage-one confirmation with its loaded effects list.
 *   3. The Archived agents section that appears after confirming.
 *   4. The new-agent menu's "Restore Buzz starter agents" item.
 *
 * Every shot is scoped to its subject so the PNGs stay distinct.
 */

import { expect, test } from "@playwright/test";

import { installMockBridge, TEST_IDENTITIES } from "../helpers/bridge";
import { waitForAnimations } from "../helpers/animations";

const SHOTS = "test-results/two-stage-removal-screenshots";

const SCOUT_PERSONA = {
  id: "custom:scout",
  displayName: "Scout",
  systemPrompt: "You scout things.",
};

const SCOUT_AGENT = {
  pubkey: TEST_IDENTITIES.alice.pubkey,
  name: "Scout",
  status: "running" as const,
  personaId: SCOUT_PERSONA.id,
};

async function gotoAgentsView(page: import("@playwright/test").Page) {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("open-agents-view")).toBeVisible({
    timeout: 10_000,
  });
  await page.getByTestId("open-agents-view").click();
  await expect(page.getByTestId("agents-library-personas")).toBeVisible({
    timeout: 10_000,
  });
}

test.describe("two-stage removal screenshots", () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test("removal flow: menu, stage one, archived section", async ({ page }) => {
    await installMockBridge(page, {
      personas: [SCOUT_PERSONA],
      managedAgents: [SCOUT_AGENT],
    });
    await gotoAgentsView(page);

    // Shot 01 — the card menu offers the reversible label, not "Delete".
    await page.getByLabel("Open actions for Scout").click();
    const removeItem = page.getByRole("menuitem", {
      name: "Remove from My Agents",
    });
    await expect(removeItem).toBeVisible();
    await waitForAnimations(page);
    await page.screenshot({
      path: `${SHOTS}/01-card-menu-remove-item.png`,
      clip: { x: 0, y: 80, width: 720, height: 560 },
    });

    // Shot 02 — stage one loads the preview and lists only true effects.
    await removeItem.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toContainText("Remove from My Agents: Scout");
    await expect(dialog).toContainText("Stops the agent running", {
      timeout: 10_000,
    });
    await expect(dialog).toContainText("Keeps its identity and key");
    await waitForAnimations(page);
    await dialog.screenshot({ path: `${SHOTS}/02-stage-one-confirmation.png` });

    // Shot 03 — confirming reveals the Archived agents section with the row.
    await dialog.getByRole("button", { name: "Remove from My Agents" }).click();
    await expect(page.getByText("Archived agents")).toBeVisible({
      timeout: 10_000,
    });
    const archivedRow = page.getByText("Scout", { exact: true }).last();
    await expect(archivedRow).toBeVisible();
    await waitForAnimations(page);
    await page.screenshot({
      path: `${SHOTS}/03-archived-agents-section.png`,
      clip: { x: 0, y: 340, width: 1280, height: 480 },
    });
  });

  test("new-agent menu offers starter-agent restore", async ({ page }) => {
    await installMockBridge(page, {
      personas: [SCOUT_PERSONA],
      managedAgents: [SCOUT_AGENT],
    });
    await gotoAgentsView(page);

    await page.getByTestId("new-agent-card").click();
    const restoreItem = page.getByTestId("restore-starter-agents-menu-item");
    await expect(restoreItem).toBeVisible();
    await waitForAnimations(page);
    await page.screenshot({
      path: `${SHOTS}/04-restore-starter-agents-item.png`,
      clip: { x: 0, y: 80, width: 900, height: 640 },
    });
  });
});

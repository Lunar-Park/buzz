/**
 * Screenshot spec for the connect-a-self-hosted-agent flow (RC3/RC4/P1/P3/P6):
 *   1. The connect dialog with host probe, harness, roster picker (primary
 *      preselected), and the resolved host identity offered for acceptance.
 *   2. The owner-attestation dialog after minting, showing the auth tag and
 *      where it goes on the host.
 *
 * The mock bridge answers every ssh-shaped call locally; the live path was
 * verified against lunar01 on 2026-07-30 (roster picker end to end).
 */

import { expect, test } from "@playwright/test";

import { installMockBridge } from "../helpers/bridge";
import { waitForAnimations } from "../helpers/animations";

const SHOTS = "test-results/connect-agent-screenshots";

async function waitForInvokeBridge(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => {
      const tauriWindow = window as Window & {
        __BUZZ_E2E_INVOKE_MOCK_COMMAND__?: unknown;
        __TAURI_INTERNALS__?: { invoke?: unknown };
      };
      return (
        typeof tauriWindow.__BUZZ_E2E_INVOKE_MOCK_COMMAND__ === "function" ||
        typeof tauriWindow.__TAURI_INTERNALS__?.invoke === "function"
      );
    },
    null,
    { timeout: 5_000 },
  );
}

async function gotoAgentsView(page: import("@playwright/test").Page) {
  for (const attempt of [0, 1]) {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await waitForInvokeBridge(page);
    try {
      await expect(page.getByTestId("open-agents-view")).toBeVisible({
        timeout: 10_000,
      });
      break;
    } catch (error) {
      if (attempt === 1) throw error;
    }
  }
  await page.getByTestId("open-agents-view").click();
  await expect(page.getByTestId("agents-library-personas")).toBeVisible({
    timeout: 10_000,
  });
}

async function openConnectDialog(page: import("@playwright/test").Page) {
  await page.getByTestId("new-agent-card").click();
  await page
    .getByRole("menuitem", { exact: true, name: "Connect self-hosted agent" })
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("Connect an agent");
  return dialog;
}

test.describe("connect agent screenshots", () => {
  test.use({ viewport: { width: 1280, height: 1000 } });

  test("connect dialog: roster picker and resolved identity", async ({
    page,
  }) => {
    await installMockBridge(page);
    await gotoAgentsView(page);
    const dialog = await openConnectDialog(page);

    // The mock host auto-selects and its probe reveals the harness dropdown.
    await expect(dialog.locator("#connect-agent-harness")).toBeVisible({
      timeout: 10_000,
    });
    await dialog.locator("#connect-agent-harness").selectOption("openclaw");

    // Roster: primary preselected, rest visible but unselected (P3).
    const rosterSelect = dialog.locator("#connect-harness-agent");
    await expect(rosterSelect).toBeVisible({ timeout: 10_000 });
    await expect(rosterSelect).toHaveValue("main");

    // Identity field resolves the harness's configured pubkey (P1).
    await expect(dialog.getByText("Use this identity")).toBeVisible({
      timeout: 10_000,
    });
    await waitForAnimations(page);
    await dialog.screenshot({
      path: `${SHOTS}/05-connect-dialog-roster-and-identity.png`,
    });

    // Accept the resolved identity, then connect.
    await dialog.getByText("Use this identity").click();
    const connectButton = dialog.getByRole("button", {
      exact: true,
      name: "Connect",
    });
    await expect(connectButton).toBeEnabled();
    await connectButton.click();
    await expect(dialog).not.toBeVisible({ timeout: 10_000 });

    // The connected agent renders as a normal card (RC4).
    await expect(page.getByLabel("Selene connected agent actions")).toBeVisible(
      { timeout: 10_000 },
    );

    // Owner attestation (P6): mint and show the tag with install guidance.
    await page.getByLabel("Selene connected agent actions").click();
    await page
      .getByRole("menuitem", { name: /owner attestation|attestation/i })
      .click();
    const evidenceDialog = page.getByRole("dialog");
    await expect(evidenceDialog).toBeVisible();
    const mint = evidenceDialog.getByRole("button", {
      name: /mint|issue|attest/i,
    });
    if (await mint.isVisible().catch(() => false)) {
      await mint.click();
    }
    await expect(evidenceDialog).toContainText("BUZZ_AUTH_TAG", {
      timeout: 10_000,
    });
    await waitForAnimations(page);
    await evidenceDialog.screenshot({
      path: `${SHOTS}/06-owner-attestation-dialog.png`,
    });
  });
});

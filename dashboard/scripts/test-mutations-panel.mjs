/**
 * Playwright smoke: Dataflow → Field mutations (CPG) panel.
 *
 * Prereq (ecommerce-java with CFG + dashboard):
 *   cargo build --release
 *   ./scripts/build-dashboard.sh   # if UI changed
 *   rgctl -r rgbuilder-tests/ecommerce-java discover . -l java -e target \
 *     --with-cfg --with-dashboard
 *   rgctl -r rgbuilder-tests/ecommerce-java serve --port 8080
 *
 * Usage:
 *   DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/test-mutations-panel.mjs
 */

import { chromium } from "playwright";

const BASE = process.env.DASHBOARD_URL ?? "http://127.0.0.1:8080/";
const TYPE = process.env.MUTATIONS_TYPE ?? "ShoppingCart";

async function waitForServer() {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${BASE.replace(/\/$/, "")}/api/health`);
      if (res.ok) return;
    } catch {
      // retry
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`server not ready at ${BASE}`);
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();

await waitForServer();

const mutationsJson = await fetch(`${BASE.replace(/\/$/, "")}/mutations_index.json`).then((r) =>
  r.json(),
);

await page.goto(BASE, { waitUntil: "networkidle", timeout: 60000 });
await page.waitForSelector(".rb-app", { timeout: 30000 });
await sleep(800);

await page.locator(".rb-main-tabs").getByRole("button", { name: "Dataflow", exact: true }).click();
await page.getByTestId("mutations-panel").waitFor({ state: "visible", timeout: 15000 });

const typeInput = page.getByTestId("mutations-type-input");
await typeInput.fill("");
await typeInput.fill(TYPE);
await sleep(300);

const exclude = page.getByTestId("mutations-exclude-ctors");
if (!(await exclude.isChecked())) {
  await exclude.check();
}

await page.getByTestId("mutations-table").waitFor({ state: "visible", timeout: 10000 });
const rowCount = await page.getByTestId("mutations-row").count();
const firstRow = page.getByTestId("mutations-row").first();
const line = await firstRow.getAttribute("data-line");
const functionId = await firstRow.getAttribute("data-function-id");
await firstRow.click();
await sleep(1200);

const selectedActive = await page.locator(".function-list-item.active").count();

// Function should load: graph stage or source panel appears
const sourceVisible = await page.locator(".dataflow-source-panel").isVisible().catch(() => false);
const graphVisible = await page.locator(".dataflow-graph-panel").isVisible().catch(() => false);
const loadingGone = !(await page.getByText("Loading function…").isVisible().catch(() => false));

const report = {
  base: BASE,
  mutationsAvailable: mutationsJson.available === true,
  mutationsWriteCount: mutationsJson.write_count ?? 0,
  typesIncludesShoppingCart: Array.isArray(mutationsJson.types)
    ? mutationsJson.types.includes(TYPE)
    : false,
  rowCount,
  clickedLine: line,
  clickedFunctionId: functionId,
  sourceVisible,
  graphVisible,
  loadingGone,
  selectedActive,
};

console.log(JSON.stringify(report, null, 2));

await browser.close();

const ok =
  report.mutationsAvailable &&
  report.mutationsWriteCount > 0 &&
  report.typesIncludesShoppingCart &&
  report.rowCount > 0 &&
  Boolean(report.clickedLine) &&
  Boolean(report.clickedFunctionId) &&
  report.loadingGone &&
  report.selectedActive >= 1 &&
  (report.sourceVisible || report.graphVisible);

process.exit(ok ? 0 : 1);

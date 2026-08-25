/**
 * Capture dashboard screenshots for docs/design/*.md
 *
 * Prereq: indexed repo + HTTP server, e.g.
 *   rgctl -r /path/to/gbuilder discover . --all
 *   rgctl -r /path/to/gbuilder serve --port 8080
 *
 * Usage:
 *   DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/capture-design-screenshots.mjs
 */

import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";

const BASE = process.env.DASHBOARD_URL ?? "http://127.0.0.1:8080/";
const ROOT = path.resolve(import.meta.dirname, "../..");
const OUT_ROOT =
  process.env.DESIGN_DOCS_SCREENSHOT_DIR ??
  path.join(ROOT, "docs/images/design");

/** Known-good gbuilder symbols (override via env for other repos). */
const FN_DATAFLOW = process.env.CAPTURE_FN_DATAFLOW ?? "addEmbeddingSimilarityEdges";
const FN_SLICE = process.env.CAPTURE_FN_SLICE ?? "addEmbeddingSimilarityEdges";
const FN_TAINT = process.env.CAPTURE_FN_TAINT ?? "clearFileGraph";
const FN_CFG = process.env.CAPTURE_FN_CFG ?? "addEmbeddingSimilarityEdges";
const FN_BLAST = process.env.CAPTURE_FN_BLAST ?? "addEmbeddingSimilarityEdges";

const SLICE_LINE = Number(process.env.CAPTURE_SLICE_LINE ?? "45");
const SLICE_VAR = process.env.CAPTURE_SLICE_VAR ?? "threshold";

const SEMANTIC_QUERY = process.env.CAPTURE_SEMANTIC_QUERY ?? "embedding similarity graph";

const FEATURES = [
  "semantic-search",
  "blast-radius",
  "program-slicing",
  "taint-analysis",
  "cfg",
  "pdg",
  "dominance",
  "gql",
  "graph-metrics",
  "migration-planner",
  "ci-policy-checks",
];

for (const feature of FEATURES) {
  fs.mkdirSync(path.join(OUT_ROOT, feature), { recursive: true });
}

function out(feature, name) {
  return path.join(OUT_ROOT, feature, name);
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function clickTab(page, label) {
  await page.locator(".rb-main-tabs").getByRole("button", { name: label, exact: true }).click();
  await sleep(600);
}

/** Wait until WASM worker is ready (blast/functions tabs need it). */
async function waitForWasm(page) {
  await page.waitForFunction(
    () => {
      const msg = document.body.textContent ?? "";
      if (msg.includes("WASM engine required for blast-radius")) return false;
      if (msg.includes("Waiting for WASM engine")) return false;
      return true;
    },
    { timeout: 90000 },
  );
  await sleep(1500);
}

async function waitForSigmaCanvas(page, hostSelector) {
  await page.waitForFunction(
    (sel) => {
      const host = document.querySelector(sel);
      const canvas = host?.querySelector("canvas");
      return Boolean(host && canvas && canvas.height >= 32);
    },
    hostSelector,
    { timeout: 30000 },
  );
  await sleep(800);
}

async function selectFunction(page, name) {
  const search = page.locator('.function-list-sidebar input[type="search"]');
  if (await search.count()) {
    await search.fill("");
    await sleep(200);
    await search.fill(name);
    await sleep(500);
  }
  const item = page.locator(".function-list-item", {
    has: page.locator(".function-list-item-name", { hasText: name }),
  });
  await item.first().waitFor({ state: "visible", timeout: 15000 });
  await item.first().click();
  await sleep(600);
}

async function selectFirstFunction(page) {
  const item = page.locator(".function-list-item").first();
  await item.waitFor({ state: "visible", timeout: 15000 });
  await item.click();
  await sleep(600);
}

async function waitForDataflowGraph(page) {
  await page.locator(".dataflow-graph-panel").waitFor({ state: "visible", timeout: 25000 });
  await waitForSigmaCanvas(page, ".dataflow-graph-panel .sigma-host");
}

async function waitForBlastResults(page) {
  await page.getByText("Callers of", { exact: false }).waitFor({ state: "visible", timeout: 20000 });
  await page.waitForFunction(
    () => {
      const el = document.querySelector(".blast-view .card-body .fs-4.fw-semibold.text-primary");
      return el && el.textContent && el.textContent.trim().length > 0;
    },
    { timeout: 20000 },
  );
  await sleep(500);
}

async function loadCfgForSelection(page) {
  const loadBtn = page.getByRole("button", { name: /Load CFG graph/i });
  if (await loadBtn.count()) {
    await loadBtn.click();
    await sleep(500);
  }
  await page.locator(".cfg-detail, .cfg-graph-panel").first().waitFor({ state: "visible", timeout: 25000 });
  await waitForSigmaCanvas(page, ".cfg-graph-panel .sigma-host, .cfg-graph-wrap .sigma-host");
}

async function screenshotMainPanel(page, feature, name) {
  const panel = page.locator(".function-list-main").first();
  if (await panel.count()) {
    await panel.screenshot({ path: out(feature, name) });
  } else {
    await page.screenshot({ path: out(feature, name), fullPage: true });
  }
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

await page.goto(BASE, { waitUntil: "networkidle", timeout: 120000 });
await page.waitForSelector(".rb-app", { timeout: 60000 });
await sleep(2500);
await waitForWasm(page);

// --- Semantic search ---
await clickTab(page, "Search");
await page.waitForSelector(".search-view", { timeout: 30000 });
const searchInput = page.locator('.search-view input[type="search"]');
if (await searchInput.isEnabled()) {
  await searchInput.fill(SEMANTIC_QUERY);
  await page.locator('.search-view button[type="submit"]').click();
  await page.locator(".search-results tbody tr").first().waitFor({ state: "visible", timeout: 30000 }).catch(() => {});
  await sleep(800);
}
await screenshotMainPanel(page, "semantic-search", "semantic-search-results.png");

// --- Graph / GQL ---
await clickTab(page, "Graph Visualization");
await page.waitForSelector(".graph-panel.h-100", { timeout: 30000 });
await sleep(2500);
await page.locator(".graph-panel.h-100").screenshot({ path: out("gql", "gql-metagraph.png") });
await page.screenshot({ path: out("gql", "gql-overview.png"), fullPage: true });

// --- Graph metrics (Functions tab) ---
await clickTab(page, "Functions");
await page.waitForSelector(".functions-view table, .functions-table", { timeout: 30000 });
await sleep(2000);
const prHeader = page.getByRole("button", { name: /Sort by PR/i });
if (await prHeader.count()) await prHeader.click();
await sleep(800);
await page.screenshot({ path: out("graph-metrics", "graph-metrics-functions-table.png"), fullPage: true });

// --- CFG ---
await clickTab(page, "CFG / PDG Analysis");
await page.waitForSelector(".function-list-item", { timeout: 30000 });
await selectFunction(page, FN_CFG);
const cfgIndex = await page.evaluate(async () => {
  const res = await fetch("./cfg_index.json");
  return res.json();
});
if (cfgIndex.detail_mode === "archive_only") {
  await loadCfgForSelection(page);
} else {
  await page.locator(".cfg-detail").waitFor({ state: "visible", timeout: 25000 });
  await waitForSigmaCanvas(page, ".cfg-graph-panel .sigma-host, .cfg-graph-wrap .sigma-host");
}
await screenshotMainPanel(page, "cfg", "cfg-control-flow.png");
const cfgGraph = page.locator(".cfg-graph-panel, .cfg-graph-col").first();
if (await cfgGraph.count()) {
  await cfgGraph.screenshot({ path: out("cfg", "cfg-graph-canvas.png") });
}

// --- PDG (Dataflow tab) ---
await clickTab(page, "Dataflow");
await page.waitForSelector(".function-list-item", { timeout: 30000 });
await selectFunction(page, FN_DATAFLOW);
await page.locator("#df-view").selectOption("dataflow");
const varSelect = page.locator("#df-var");
await varSelect.waitFor({ state: "visible", timeout: 15000 });
const vars = await varSelect.locator("option").evaluateAll((nodes) =>
  nodes.map((n) => n.getAttribute("value") ?? "").filter(Boolean),
);
if (vars.length > 0) await varSelect.selectOption(vars[0]);
await waitForDataflowGraph(page);
await screenshotMainPanel(page, "pdg", "pdg-dataflow.png");

// --- Dominance ---
await page.locator("#df-view").selectOption("dominator");
await sleep(1500);
await waitForDataflowGraph(page);
await screenshotMainPanel(page, "dominance", "dominance-tree.png");

// --- Program slicing ---
await clickTab(page, "Program Slicing");
await page.waitForSelector(".slice-view", { timeout: 30000 });
await selectFunction(page, FN_SLICE);
await page.locator("#slice-line").fill(String(SLICE_LINE));
await page.locator("#slice-var").fill(SLICE_VAR);
await page.getByRole("button", { name: "Compute slice" }).click();
await page.getByText(/slice:/i).waitFor({ state: "visible", timeout: 25000 });
await sleep(1000);
await screenshotMainPanel(page, "program-slicing", "slice-editor.png");

// --- Blast radius ---
await clickTab(page, "Blast Radius");
await page.waitForSelector(".blast-view", { timeout: 30000 });
await waitForWasm(page);
await selectFunction(page, FN_BLAST);
await waitForBlastResults(page);
await screenshotMainPanel(page, "blast-radius", "blast-radius-impact.png");
const blastMetrics = page.locator(".blast-view .row.g-2").first();
if (await blastMetrics.count()) {
  await blastMetrics.screenshot({ path: out("blast-radius", "blast-radius-metrics.png") });
}

// --- Taint ---
await clickTab(page, "Taint Analysis");
await page.waitForSelector(".taint-view", { timeout: 30000 });
await selectFunction(page, FN_TAINT);
await page.locator(".taint-view table tbody tr").first().waitFor({ state: "visible", timeout: 20000 });
await sleep(800);
await screenshotMainPanel(page, "taint-analysis", "taint-flows.png");

// --- Migration ---
await clickTab(page, "Migration");
await page.waitForSelector(".migration-view tbody tr", { timeout: 30000 });
await sleep(2500);
await page.screenshot({ path: out("migration-planner", "migration-unified-roadmap.png"), fullPage: true });
const graphSection = page.locator(".migration-graph-section");
if (await graphSection.count()) {
  await graphSection.scrollIntoViewIfNeeded();
  await sleep(2000);
  await graphSection.screenshot({ path: out("migration-planner", "migration-package-graph.png") });
}
const tableSection = page.locator(".migration-table-section");
if (await tableSection.count()) {
  await tableSection.screenshot({ path: out("migration-planner", "migration-packages-table.png") });
}

// --- CI policy (blast scores for threshold calibration) ---
await clickTab(page, "Blast Radius");
await selectFunction(page, FN_BLAST);
await waitForBlastResults(page);
await screenshotMainPanel(page, "ci-policy-checks", "policy-blast-scores.png");

const report = {
  base: BASE,
  outRoot: OUT_ROOT,
  functions: { FN_DATAFLOW, FN_SLICE, FN_TAINT, FN_CFG, FN_BLAST },
  features: Object.fromEntries(
    FEATURES.map((f) => [
      f,
      fs.readdirSync(path.join(OUT_ROOT, f)).filter((n) => n.endsWith(".png")),
    ]),
  ),
};

console.log(JSON.stringify(report, null, 2));
await browser.close();

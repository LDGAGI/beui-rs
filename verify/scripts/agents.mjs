// Lane-c runtime smoke: do the agents pages mount without panicking, and do the
// pieces the source promises actually reach the DOM?
//
// There is no beUI reference image for these pages in this environment, so this
// is the check a pixel diff would not replace anyway: the page boots, every
// `data-slot` the source emits is present, the composer accepts input and
// submits on Enter, and the console stays clean. Screenshots are written for the
// record.
//
// Usage: node verify/scripts/agents.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(
  process.argv[2] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/agents");

const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".json": "application/json",
};

const failures = [];
function check(name, ok, detail = "") {
  const line = `${ok ? "ok  " : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`;
  console.log(line);
  if (!ok) failures.push(line);
}

async function serve(dir) {
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url, "http://localhost");
      let path = join(dir, decodeURIComponent(url.pathname));
      if (url.pathname === "/" || url.pathname.endsWith("/")) {
        path = join(path, "index.html");
      }
      const body = await readFile(path);
      response.writeHead(200, {
        "content-type": MIME[extname(path)] ?? "application/octet-stream",
      });
      response.end(body);
    } catch {
      response.writeHead(404).end("not found");
    }
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  return { server, origin: `http://127.0.0.1:${server.address().port}` };
}

async function main() {
  await mkdir(OUT, { recursive: true });
  const { server, origin } = await serve(DIST);
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();

  const ignored = /_dioxus/;
  const consoleErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error" && !ignored.test(message.text())) {
      consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => consoleErrors.push(String(error)));

  await page.goto(origin, { waitUntil: "networkidle" });

  // --- agents page -----------------------------------------------------------
  await page.click('[data-page-link="agents"]');
  await page.waitForSelector('[data-state="complete"], [data-state="streaming"]');
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(250);

  // Code block markup.
  const codeLines = await page.locator('[data-state="streaming"] span.grid').count();
  check("CodeBlock renders numbered rows", codeLines > 0, `${codeLines} rows`);
  check(
    "CodeBlock shows the Write status",
    (await page.getByText("Writing").count()) > 0,
  );

  // Todo list markup (the trigger carries an aria label on its section).
  check(
    "TodoList trigger is present",
    (await page.locator('section[aria-label="Agent task list"]').count()) > 0,
  );
  check(
    "ToolResult data-state is present",
    (await page.locator('[data-state]').count()) > 0,
  );

  await page.screenshot({ path: join(OUT, "agents-light.png"), fullPage: true });

  // --- conversation page -----------------------------------------------------
  await page.click('[data-page-link="conversation"]');
  await page.waitForSelector('[data-slot="message"]');
  await page.waitForTimeout(250);

  const slots = [
    '[data-slot="message"]',
    '[data-slot="message-avatar"]',
    '[data-slot="message-content"]',
    '[data-slot="message-bubble"]',
    '[data-slot="message-bubble-content"]',
    '[data-slot="message-bubble-collapsible"]',
    '[data-slot="message-scroller"]',
    '[data-slot="message-bubble-group"]',
  ];
  for (const slot of slots) {
    check(`conversation has ${slot}`, (await page.locator(slot).count()) > 0);
  }

  const rows = await page.locator('[data-slot="message"]').count();
  check("scroller renders message rows", rows >= 3, `${rows} rows`);
  check(
    "typing indicator is present",
    (await page.locator('[data-slot="message-typing"]').count()) > 0,
  );
  check(
    "marker is present",
    (await page.locator('[data-slot="message-marker"]').count()) > 0,
  );

  // The composer.
  const textarea = page.locator("textarea");
  check("prompt textarea is present", (await textarea.count()) === 1);
  await textarea.fill("hello agent");
  const value = await textarea.inputValue();
  check("composer accepts input", value === "hello agent", value);

  // Enter submits: after submit the source clears the textarea (uncontrolled).
  await textarea.press("Enter");
  await page.waitForTimeout(150);
  const afterEnter = await textarea.inputValue();
  check("Enter clears the composer", afterEnter === "", JSON.stringify(afterEnter));

  // Shift+Enter must NOT submit (keeps the newline).
  await textarea.fill("line one");
  await textarea.press("Shift+Enter");
  await textarea.type("line two");
  const shifted = await textarea.inputValue();
  check("Shift+Enter inserts a newline", shifted.includes("\n"), JSON.stringify(shifted));

  // Toggling the add-to-prompt popover.
  await page.click('button[aria-label="Add to prompt"]');
  await page.waitForTimeout(120);
  check("actions popover opens", (await page.getByText("Add file").count()) > 0);

  // Toggling the collapsible.
  await page.click('[data-slot="message-bubble-collapsible"] button');
  await page.waitForTimeout(120);
  const state = await page
    .locator('[data-slot="message-bubble-collapsible"]')
    .getAttribute("data-state");
  check("collapsible toggles open", state === "open", String(state));

  await page.screenshot({ path: join(OUT, "conversation-light.png"), fullPage: true });

  // --- activity page -------------------------------------------------------
  await page.click('[data-page-link="activity"]');
  await page.waitForSelector('[data-state="pending"]');
  await page.waitForTimeout(250);

  check("ToolApproval pending card", (await page.locator('[data-state="pending"]').count()) > 0);
  check("ToolApproval approved card", (await page.locator('[data-state="approved"]').count()) > 0);
  check("AgentActivity working", (await page.locator('[data-state="working"]').count()) > 0);
  check(
    "AgentActivity completed",
    (await page.locator('[data-state="open"], [data-state="closed"]').count()) > 0,
  );
  const activityRows = await page.locator('[role="listitem"]').count();
  check("activity rows render", activityRows >= 6, `${activityRows} rows`);
  check(
    "shimmer is present",
    (await page.locator(".beui-text-shimmer").count()) > 0,
  );
  check(
    "agent progress is present",
    (await page.getByText("Churning").count()) > 0,
  );

  // Toggling the approval details replays the grid-rows disclosure.
  const detailsButton = page
    .locator('[data-state="pending"]')
    .getByRole("button", { name: "View details" });
  await detailsButton.click();
  await page.waitForTimeout(150);
  const expanded = await detailsButton.getAttribute("aria-expanded");
  check("approval details expand", expanded === "true", String(expanded));

  await page.screenshot({ path: join(OUT, "activity-light.png"), fullPage: true });

  // --- approval page -------------------------------------------------------
  await page.click('[data-page-link="approval"]');
  await page.waitForSelector('[role="checkbox"]');
  await page.waitForTimeout(250);

  check("ApprovalCard pending", (await page.locator('[data-state="pending"]').count()) >= 2);
  const checkboxes = await page.locator('[role="checkbox"]').count();
  check("question checkboxes render", checkboxes >= 3, `${checkboxes} checkboxes`);
  check(
    "custom input renders",
    (await page.getByPlaceholder("Add another response…").count()) > 0,
  );

  // Answer the first question and advance to the second.
  await page.locator('[role="checkbox"]').first().click();
  await page.waitForTimeout(100);
  await page.getByRole("button", { name: "Next question" }).click();
  await page.waitForTimeout(150);
  const stepLabel = await page.getByText("2/2").count();
  check("ApprovalCard advances to step 2", stepLabel > 0);
  const radios = await page.locator('[role="radio"]').count();
  check("step 2 renders radios", radios >= 2, `${radios} radios`);

  // FileDiff rows.
  check("FileDiff complete", (await page.locator('[data-state="complete"]').count()) > 0);
  check(
    "FileDiff viewport",
    (await page.locator('[data-slot="file-diff-viewport"]').count()) > 0,
  );
  const diffRows = await page
    .locator('[data-slot="file-diff-viewport"] .grid')
    .count();
  check("FileDiff rows render", diffRows >= 5, `${diffRows} rows`);

  await page.screenshot({ path: join(OUT, "approval-light.png"), fullPage: true });

  // --- generation page -----------------------------------------------------
  await page.click('[data-page-link="generation"]');
  await page.waitForSelector('[data-slot="image-generation"]');
  await page.waitForTimeout(300);

  const cards = await page.locator('[data-slot="image-generation"]').count();
  check("ImageGeneration cards render", cards >= 3, `${cards} cards`);
  check("generating card", (await page.locator('[data-state="generating"]').count()) > 0);
  check("complete card", (await page.locator('[data-state="complete"]').count()) > 0);
  check("error card", (await page.locator('[data-state="error"]').count()) > 0);
  check("dither canvas mounts", (await page.locator('[data-beui-dither]').count()) > 0);
  check(
    "dither canvas is painted",
    await page.evaluate(() => {
      const canvas = document.querySelector('[data-beui-dither]');
      if (!canvas) return false;
      const ctx = canvas.getContext('2d');
      if (!ctx || !canvas.width) return false;
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      for (let i = 3; i < data.length; i += 4) if (data[i] > 0) return true;
      return false;
    }),
  );
  check("retry button on the error card", (await page.getByText("Try again").count()) > 0);
  const orbs = await page.locator('[data-slot="voice-orb"]').count();
  check("voice orbs render", orbs >= 2, `${orbs} orbs`);

  // The chat shell folds via its provided context.
  const wrapper = page.locator('[data-slot="sidebar-wrapper"]');
  check(
    "sidebar starts expanded",
    (await wrapper.getAttribute("data-state")) === "expanded",
  );
  await page.getByRole("button", { name: "Toggle sidebar" }).click();
  await page.waitForTimeout(250);
  check(
    "toggle collapses the sidebar",
    (await wrapper.getAttribute("data-state")) === "collapsed",
  );

  await page.screenshot({ path: join(OUT, "generation-light.png"), fullPage: true });

  // --- sidebar page --------------------------------------------------------
  await page.click('[data-page-link="sidebar"]');
  await page.waitForSelector('[role="tree"]');
  await page.waitForTimeout(250);

  const rowCount = await page.locator('[role="treeitem"]').count();
  check("tree renders flat rows", rowCount === 6, `${rowCount} rows`);
  check(
    "src folder starts expanded",
    (await page.locator('[data-beui-row="src"]').getAttribute("aria-expanded")) === "true",
  );

  // Collapse and re-expand the folder.
  await page.locator('[data-beui-row="src"]').click();
  await page.waitForTimeout(150);
  check(
    "folder collapses",
    (await page.locator('[data-beui-row="src"]').getAttribute("aria-expanded")) === "false",
  );
  check("collapsed hides children", (await page.locator('[role="treeitem"]').count()) === 4);
  await page.locator('[data-beui-row="src"]').click();
  await page.waitForTimeout(150);
  check("folder re-expands", (await page.locator('[role="treeitem"]').count()) === 6);

  // Selecting a file.
  await page.locator('[data-beui-row="api"]').click();
  await page.waitForTimeout(100);
  check(
    "clicking a file selects it",
    (await page.locator('[data-beui-row="api"]').getAttribute("aria-selected")) === "true",
  );

  // The row menu: rename.
  await page.locator('[data-beui-row="readme"]').hover();
  await page.getByRole("button", { name: "Actions for README.md" }).click();
  await page.waitForSelector('[data-sidebar-resource-menu]');
  check("row menu opens", (await page.getByText("Move up").count()) > 0);
  await page.getByRole("button", { name: "Rename" }).click();
  const renameInput = page.getByRole("textbox", { name: "Rename README.md" });
  check("rename input appears", (await renameInput.count()) === 1);
  await renameInput.fill("README-v2.md");
  await renameInput.press("Enter");
  await page.waitForTimeout(150);
  check(
    "rename commits",
    (await page.getByText("README-v2.md").count()) > 0,
  );

  // Keyboard navigation on the tree.
  await page.locator('[data-beui-row="src"]').focus();
  await page.keyboard.press("ArrowDown");
  await page.waitForTimeout(100);
  check(
    "ArrowDown moves focus",
    await page.evaluate(() => document.activeElement?.getAttribute("data-beui-row") === "lib"),
  );

  await page.screenshot({ path: join(OUT, "sidebar-light.png"), fullPage: true });

  // --- charts page ---------------------------------------------------------
  await page.click('[data-page-link="charts"]');
  await page.waitForSelector('[data-slot="status-bar"]');
  await page.waitForTimeout(250);

  check("status-bar plot renders", (await page.locator('[data-slot="status-bar-plot"]').count()) === 1);
  const periods = await page.locator('[data-status-period]').count();
  check("status-bar periods", periods === 12, `${periods} periods`);
  check("status-bar legend", (await page.locator('[data-slot="status-bar-legend"] li').count()) === 5);
  const tableRows = await page.locator('[data-slot="status-bar"] table tbody tr').count();
  check("screen-reader table", tableRows === 12, `${tableRows} rows`);

  // Inspecting with the keyboard opens the tooltip.
  const statusSlider = page.locator('[data-slot="status-bar"] input[type="range"]');
  await statusSlider.focus();
  await page.keyboard.press("ArrowRight");
  await page.waitForTimeout(150);
  check("keyboard inspect opens tooltip", (await page.locator('[role="tooltip"]').count()) === 1);
  const described = await statusSlider.getAttribute("aria-describedby");
  check("slider points at the tooltip", described !== null && described.length > 0);

  // Donut chart.
  check("donut svg renders", (await page.locator('svg[role="img"][aria-label="Traffic by channel"]').count()) === 1);
  const legendRows = await page.locator('[data-donut-legend]').count();
  check("donut legend rows", legendRows === 4, `${legendRows} rows`);
  check("centre shows the total", (await page.getByText("10,000").count()) > 0);
  await page.locator('[data-donut-legend]').first().hover();
  await page.waitForTimeout(200);
  check("hover shows the segment value", (await page.getByText("4,200").count()) > 0);
  check("hover shows the caption", (await page.getByText("Direct · 42.0%").count()) > 0);
  await page.locator('[data-donut-legend]').first().focus();
  await page.keyboard.press("ArrowDown");
  await page.waitForTimeout(100);
  check(
    "legend ArrowDown moves focus",
    await page.evaluate(() => document.activeElement?.textContent?.includes("Search") === true),
  );

  // Treemap.
  check("treemap renders", (await page.locator('[data-slot="treemap"]').count()) === 1);
  const tiles = await page.locator('[data-treemap-tile]').count();
  check("treemap tiles", tiles === 7, `${tiles} tiles`);
  check("treemap legend", (await page.locator('[aria-label="Treemap legend"] li').count()) === 3);
  check(
    "treemap screen-reader table",
    (await page.locator('[data-slot="treemap"] table tbody tr').count()) === 7,
  );
  const firstTile = page.locator('[data-treemap-tile]').first();
  const box = await firstTile.boundingBox();
  check("tiles are measured and laid out", box !== null && box.width > 0 && box.height > 0);
  await firstTile.hover();
  await page.waitForTimeout(200);
  check("hovering a tile opens the tooltip", (await page.locator('[role="tooltip"]').count()) === 1);
  await firstTile.focus();
  const focusedBefore = await page.evaluate(() => document.activeElement?.getAttribute("data-treemap-tile"));
  await page.keyboard.press("ArrowRight");
  await page.waitForTimeout(120);
  const focusedAfter = await page.evaluate(() => document.activeElement?.getAttribute("data-treemap-tile"));
  check(
    "treemap arrow key moves focus",
    focusedBefore !== null && focusedAfter !== focusedBefore,
    `${focusedBefore} -> ${focusedAfter}`,
  );

  // Composition chart.
  const composition = page.locator('section[aria-label="Traffic composition"]');
  check("composition renders", (await composition.count()) === 1);
  const bars = await composition.locator("svg rect").count();
  check("composition bars render", bars >= 20, `${bars} bars`);
  const compLegend = await composition.locator('button[aria-label^="Highlight "]').count();
  check("composition legend", compLegend === 4, `${compLegend} entries`);
  await composition.locator('button[aria-label^="Highlight "]').first().click();
  await page.waitForTimeout(120);
  check(
    "legend click pins a series",
    (await composition.locator('button[aria-label^="Highlight "]').first().getAttribute("aria-pressed")) === "true",
  );
  await composition.getByRole("slider").focus();
  await page.waitForTimeout(150);
  check("composition tooltip opens", (await composition.locator('[role="tooltip"]').count()) === 1);

  // Bump chart.
  const bump = page.locator('section[aria-label="League standings"]');
  check("bump chart renders", (await bump.count()) === 1);
  const bumpPaths = await bump.locator("svg path").count();
  check("bump chart paths render", bumpPaths >= 12, `${bumpPaths} paths`);
  const bumpPoints = await bump.locator('button[aria-label*="rank"]').count();
  check("bump chart points", bumpPoints === 23, `${bumpPoints} points`);
  const bumpLegend = await bump.locator('button[aria-label^="Highlight "]').count();
  check("bump chart legend", bumpLegend === 4, `${bumpLegend} entries`);
  await bump.locator('button[aria-label*="rank"]').first().hover();
  await page.waitForTimeout(200);
  check("bump point tooltip opens", (await bump.locator('[role="tooltip"]').count()) === 1);
  await bump.locator('button[aria-label^="Highlight "]').first().click();
  await page.waitForTimeout(120);
  check(
    "bump legend pins a series",
    (await bump.locator('button[aria-label^="Highlight "]').first().getAttribute("aria-pressed")) === "true",
  );

  // Candlestick chart.
  const candlePlot = page.locator('[data-beui-candlestick]');
  check("candlestick renders", (await candlePlot.count()) === 1);
  const candleGroups = await candlePlot.locator("svg g").count();
  check("candles render", candleGroups === 12, `${candleGroups} candles`);
  check(
    "crosshair readout present",
    ((await candlePlot.getAttribute("aria-valuetext")) ?? "").includes("open"),
  );
  const candleBefore = await candlePlot.getAttribute("aria-valuenow");
  await candlePlot.focus();
  await page.keyboard.press("ArrowLeft");
  await page.waitForTimeout(120);
  const candleAfter = await candlePlot.getAttribute("aria-valuenow");
  check(
    "arrow key moves the crosshair",
    candleBefore !== candleAfter,
    `${candleBefore} -> ${candleAfter}`,
  );

  // Funnel chart.
  const funnel = page.locator('section[aria-label="Signup funnel"]');
  check("funnel renders", (await funnel.count()) === 1);
  check("funnel stages render", (await funnel.locator("svg path").count()) === 5);
  check(
    "funnel stage list",
    (await funnel.locator('ol[aria-label="Funnel stages"] li').count()) === 5,
  );
  check("funnel summary", (await funnel.getByText("Overall conversion").count()) > 0);
  await funnel.locator("ol li button").first().hover();
  await page.waitForTimeout(150);
  check("funnel tooltip opens", (await funnel.locator('[role="tooltip"]').count()) === 1);

  // Heat calendar.
  const heatCells = await page.locator("[data-heat-cell]").count();
  check("heat calendar cells render", heatCells > 90 && heatCells <= 112, `${heatCells} cells`);
  check(
    "heat calendar legend",
    (await page.locator('button[aria-label^="Show activity level"]').count()) === 5,
  );
  await page.locator("[data-heat-cell]").first().hover();
  await page.waitForTimeout(150);
  check(
    "heat calendar tooltip opens",
    (await page.getByRole("tooltip").filter({ hasText: "commits" }).count()) >= 1,
  );
  await page.locator("[data-heat-cell]").nth(3).click();
  await page.waitForTimeout(120);
  check(
    "heat calendar cell pins",
    (await page.locator("[data-heat-cell]").nth(3).getAttribute("aria-pressed")) === "true",
  );

  // Liquidity heatmap.
  const liquidity = page.locator('section[aria-label="Order book depth"]');
  check("liquidity renders", (await liquidity.count()) === 1);
  const liquidityCells = await liquidity.locator("[data-cell]").count();
  check("liquidity cells render", liquidityCells === 120, `${liquidityCells} cells`);
  check("liquidity trace renders", (await liquidity.locator("svg path").count()) === 2);
  await liquidity.locator("[data-cell]").first().hover();
  await page.waitForTimeout(150);
  check("liquidity tooltip opens", (await liquidity.locator('[role="tooltip"]').count()) === 1);
  const liquidityCell = liquidity.locator("[data-cell]").nth(1);
  await liquidityCell.focus();
  await page.keyboard.press("ArrowRight");
  await page.waitForTimeout(120);
  check(
    "liquidity arrow key moves focus",
    await page.evaluate(() => document.activeElement?.getAttribute("data-cell") === "2"),
  );

  // Order book.
  const orderBook = page.locator('section[aria-label="ETH/USD"]');
  check("order book renders", (await orderBook.count()) === 1);
  const bookRows = await orderBook.locator("tbody tr").count();
  check("order book rows render", bookRows === 18, `${bookRows} rows`);
  check("order book spread", (await orderBook.getByText(/Spread/).count()) > 0);
  await orderBook.locator("tbody tr").first().hover();
  await page.waitForTimeout(150);
  check("order book depth summary", (await orderBook.getByText("Avg").count()) > 0);

  // Price target fan.
  const fan = page.locator('svg[role="group"][aria-label^="Price target"]');
  check("price target fan renders", (await fan.count()) === 1);
  check("fan history path", (await page.locator("[data-price-history]").count()) === 1);
  const fanTargets = await fan.locator('g[role="button"]').count();
  check("fan projections render", fanTargets === 3, `${fanTargets} projections`);
  const fanBox = await fan.boundingBox();
  if (fanBox) {
    await page.mouse.move(fanBox.x + fanBox.width * 0.92, fanBox.y + fanBox.height * 0.4);
  }
  await page.waitForTimeout(150);
  check("fan overlay opens", (await page.locator('[role="tooltip"]').count()) >= 1);

  // Returns calendar.
  const returnsCells = await page.locator("[data-return-cell]").count();
  check("returns calendar cells render", returnsCells === 91, `${returnsCells} cells`);
  await page.locator("[data-return-cell]").first().hover();
  await page.waitForTimeout(150);
  check("returns tooltip opens", (await page.locator('[role="tooltip"]').count()) >= 1);
  await page.locator("[data-return-cell]").first().click();
  await page.waitForTimeout(120);
  check(
    "returns month pins",
    (await page.locator("[data-return-cell]").first().getAttribute("aria-pressed")) === "true",
  );

  // Volume profile.
  const volume = page.locator('[data-slot="volume-profile"]');
  check("volume profile renders", (await volume.count()) === 1);
  const volumeBins = await page.locator("[data-volume-bin]").count();
  check("volume bins render", volumeBins === 30, `${volumeBins} bins`);
  check("volume POC marked", (await page.locator("[data-poc]").count()) === 1);
  await volume.getByRole("slider").focus();
  await page.waitForTimeout(150);
  check("volume tooltip opens", (await volume.locator('[role="tooltip"]').count()) === 1);

  await page.screenshot({ path: join(OUT, "charts-light.png"), fullPage: true });

  // Dark mode for the record.
  await page.evaluate(() => document.documentElement.classList.add("dark"));
  await page.waitForTimeout(120);
  await page.screenshot({ path: join(OUT, "charts-dark.png"), fullPage: true });

  check("no console errors", consoleErrors.length === 0, consoleErrors.join(" | "));

  await browser.close();
  server.close();

  if (failures.length) {
    console.error(`\n${failures.length} failed`);
    process.exit(1);
  }
  console.log("\nall checks passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

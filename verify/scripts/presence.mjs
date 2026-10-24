// Presence verification: does an exiting subtree really stay mounted, really
// stop taking input, and really come back when the exit is interrupted?
//
// A pixel diff cannot answer any of those — the exiting frame and the mounted
// frame look the same in a screenshot. What it can check is *when* things happen
// and what the DOM says while they do.
//
// Usage: node scripts/presence.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(
  process.argv[2] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/presence");

const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
};

// `ENTER_MS` / `EXIT_MS` in `src/pages/presence.rs`.
const EXIT_MS = 300;

const failures = [];
function check(name, ok, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
  if (!ok) failures.push(name);
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

const probe = (page, name) =>
  page.getAttribute(`[data-probe="${name}"]`, "data-probe-value");

/** What the DOM says about the panel right now. */
const layer = (page) =>
  page.evaluate(() => {
    const panel = document.querySelector("[data-presence-panel]");
    const backdrop = document.querySelector("[data-presence-backdrop]");
    return {
      mounted: Boolean(panel),
      inert: panel ? panel.inert : null,
      inertAttribute: panel ? panel.hasAttribute("inert") : null,
      pointerEvents: panel ? getComputedStyle(panel).pointerEvents : null,
      opacity: panel ? Number(getComputedStyle(panel).opacity) : null,
      backdropMounted: Boolean(backdrop),
      backdropPointerEvents: backdrop
        ? getComputedStyle(backdrop).pointerEvents
        : null,
    };
  });

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
  await page.click('[data-page-link="presence"]');
  await page.waitForSelector("[data-presence-toggle]");
  await page.waitForTimeout(300);

  const stage = await page.locator("[data-presence-stage]").boundingBox();

  // --- enter ---------------------------------------------------------------
  check("nothing is mounted before opening", !(await layer(page)).mounted);

  await page.click("[data-presence-toggle]");
  await page.waitForTimeout(60);
  const entering = await layer(page);
  check("opening mounts both layers", entering.mounted && entering.backdropMounted);
  check(
    "an entering layer is not inert and takes pointer events",
    entering.inert === false &&
      entering.pointerEvents === "auto" &&
      entering.backdropPointerEvents === "auto",
    JSON.stringify(entering),
  );

  await page.waitForTimeout(EXIT_MS + 120);
  const open = await layer(page);
  check(
    "the enter animation settles at full opacity",
    open.opacity === 1,
    String(open.opacity),
  );
  await page.screenshot({ path: join(OUT, "open.png") });

  // --- exit ----------------------------------------------------------------
  const exit = await page.evaluate(async () => {
    const started = performance.now();
    document.querySelector("[data-presence-toggle]").click();
    const samples = [];
    for (;;) {
      const panel = document.querySelector("[data-presence-panel]");
      if (!panel) return { elapsed: performance.now() - started, samples };
      samples.push({
        at: performance.now() - started,
        opacity: Number(getComputedStyle(panel).opacity),
        inert: panel.inert,
        pointerEvents: getComputedStyle(panel).pointerEvents,
      });
      if (performance.now() - started > 3000) {
        return { elapsed: performance.now() - started, samples, timeout: true };
      }
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
  });

  check(
    "the exiting layer stays mounted for the exit, then unmounts",
    !exit.timeout && exit.elapsed > EXIT_MS * 0.7 && exit.elapsed < EXIT_MS * 2.2,
    `${exit.elapsed.toFixed(0)}ms of ${EXIT_MS}ms declared`,
  );
  check(
    "the exit animates rather than waiting out a timer",
    exit.samples.length > 3 &&
      exit.samples[0].opacity > exit.samples[exit.samples.length - 1].opacity &&
      exit.samples[exit.samples.length - 1].opacity < 0.2,
    `${exit.samples.length} frames, ${exit.samples[0]?.opacity.toFixed(2)} → ${exit.samples[exit.samples.length - 1]?.opacity.toFixed(2)}`,
  );
  // The first sample is taken in the same task as the click, before Dioxus has
  // committed the render that flips the gate — that is the point of the gate:
  // it flips in the commit that *starts* the exit, a frame later, not at the end
  // of the exit. Everything after that frame has to be inert.
  const afterFirstFrame = exit.samples.slice(1);
  check(
    "the gate releases interaction in the first frame of the exit",
    afterFirstFrame.length > 0 && afterFirstFrame[0].at < 40,
    `${afterFirstFrame[0]?.at.toFixed(1)}ms`,
  );
  check(
    "an exiting layer is inert and takes no pointer events",
    afterFirstFrame.every(
      (sample) => sample.inert && sample.pointerEvents === "none",
    ),
    JSON.stringify(exit.samples.slice(0, 2)),
  );
  check(
    "on_exit_complete fires once for the exit",
    (await probe(page, "presence-exits")) === "1",
    String(await probe(page, "presence-exits")),
  );

  // --- the gate releases interaction while the exit plays -------------------
  await page.click("[data-presence-toggle]");
  await page.waitForTimeout(400);
  const clicksBefore = await probe(page, "backdrop-clicks");
  await page.click("[data-presence-toggle]");
  await page.waitForTimeout(80);
  // The panel is still mounted and the backdrop still covers the stage; the
  // gate has already released it, so this click must land on nothing.
  await page.mouse.click(stage.x + 20, stage.y + stage.height - 20);
  await page.waitForTimeout(EXIT_MS + 200);
  check(
    "a click on the backdrop during the exit does not activate it",
    (await probe(page, "backdrop-clicks")) === clicksBefore,
    `${clicksBefore} → ${await probe(page, "backdrop-clicks")}`,
  );

  // --- interrupted exit ----------------------------------------------------
  await page.click("[data-presence-toggle]");
  await page.waitForTimeout(400);
  const exitBefore = await probe(page, "presence-exits");
  await page.click("[data-presence-toggle]");
  await page.waitForTimeout(90);
  await page.click("[data-presence-toggle]");
  const mountedThroughout = [];
  for (let i = 0; i < 10; i++) {
    await page.waitForTimeout(30);
    mountedThroughout.push((await layer(page)).mounted);
  }
  await page.waitForTimeout(EXIT_MS + 200);
  const reentered = await layer(page);
  check(
    "an interrupted exit never unmounts the layer",
    mountedThroughout.every(Boolean),
    mountedThroughout.map((m) => (m ? "1" : "0")).join(""),
  );
  check(
    "the layer is left present and interactive",
    reentered.mounted && reentered.inert === false && reentered.opacity === 1,
    JSON.stringify(reentered),
  );
  check(
    "an interrupted exit does not report a completed exit",
    Number(await probe(page, "presence-exits")) === Number(exitBefore),
    `${exitBefore} → ${await probe(page, "presence-exits")}`,
  );
  await page.screenshot({ path: join(OUT, "reentered.png") });

  // --- popLayout -----------------------------------------------------------
  const stackY = (id) =>
    page
      .locator(`[data-stack-item="${id}"]`)
      .boundingBox()
      .then((box) => (box ? Math.round(box.y) : -1));

  check("the stack starts with three rows", (await page.locator("[data-stack-item]").count()) === 3);
  const thirdBefore = await stackY(3);
  const firstBefore = await stackY(1);

  await page.click('[data-stack-dismiss="1"]');
  await page.waitForTimeout(60);
  const during = await page.evaluate(() =>
    [...document.querySelectorAll("[data-stack-item]")].map((row) => ({
      id: row.dataset.stackItem,
      // `popLayout` wraps each child, and the wrapper is what gets popped.
      wrapperPosition: getComputedStyle(row.parentElement).position,
      y: Math.round(row.getBoundingClientRect().y),
    })),
  );
  const thirdDuring = during.find((row) => row.id === "3")?.y ?? -1;
  const firstDuring = during.find((row) => row.id === "1");

  check(
    "the dismissed row is still mounted while it exits",
    during.length === 3,
    during.map((row) => row.id).join(","),
  );
  check(
    "the dismissed row is popped out of layout",
    firstDuring?.wrapperPosition === "absolute",
    String(firstDuring?.wrapperPosition),
  );
  check(
    "its siblings move up in the same commit",
    thirdDuring < thirdBefore,
    `row 3 ${thirdBefore} → ${thirdDuring}`,
  );
  check(
    "the remaining rows end up adjacent",
    thirdDuring - firstBefore < 100,
    `row 3 at ${thirdDuring}, row 1 was at ${firstBefore}`,
  );

  await page.waitForTimeout(EXIT_MS + 250);
  check(
    "the dismissed row is removed once its exit finishes",
    (await page.locator("[data-stack-item]").count()) === 2,
    String(await page.locator("[data-stack-item]").count()),
  );

  await page.click("[data-stack-reset]");
  await page.waitForTimeout(EXIT_MS + 250);
  check(
    "resetting brings the row back",
    (await page.locator("[data-stack-item]").count()) === 3,
  );
  await page.screenshot({ path: join(OUT, "stack.png") });

  check("no console errors", consoleErrors.length === 0, consoleErrors.join(" | "));

  await browser.close();
  server.close();

  console.log(`\nscreenshots: ${OUT}`);
  if (failures.length) {
    console.log(`\n${failures.length} check(s) failed: ${failures.join(", ")}`);
    process.exit(1);
  }
  console.log("\nall checks passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

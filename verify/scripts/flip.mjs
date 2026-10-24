// Layout-animation verification: does a layout change actually flip?
//
// A pixel diff cannot tell a FLIP from a delayed CSS transition, and neither can
// a screenshot: the whole point is what happens *between* the frames. What can
// be checked is the signature of the technique itself — the layout box is
// already the new one while the visual box is still the old one — and that the
// motion between the two is continuous, which a snap or a flash is not.
//
// `getBoundingClientRect()` is the visual box (transforms included);
// `offsetWidth` / `offsetHeight` / the parent's box are the layout box. A FLIP
// is exactly the gap between the two at t=0 decaying to nothing.
//
// Usage: node scripts/flip.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(
  process.argv[2] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/flip");

const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
};

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

/** Where the indicator is, visually and in layout, right now. */
const indicatorState = (page) =>
  page.evaluate(() => {
    const indicator = document.querySelector("[data-flip-indicator]");
    if (!indicator) return null;
    const rect = indicator.getBoundingClientRect();
    const host = indicator.closest("[data-flip-tab]");
    const hostRect = host.getBoundingClientRect();
    return {
      host: host.dataset.flipTab,
      left: rect.left,
      width: rect.width,
      hostLeft: hostRect.left,
      hostWidth: hostRect.width,
      transform: getComputedStyle(indicator).transform,
    };
  });

/**
 * Sample every animation frame while `action` runs in the page.
 *
 * The target is named explicitly: both demos live on one page, so a `querySelector`
 * that fell back from one to the other would silently measure the wrong element
 * — which is what a first version of this did.
 */
async function sampleFrames(page, selector, hostSelector, action, frames = 70) {
  return page.evaluate(
    async ({ selector, hostSelector, action, frames }) => {
      const samples = [];
      const started = performance.now();
      new Function(action)();
      for (let i = 0; i < frames; i++) {
        const target = document.querySelector(selector);
        if (target) {
          const rect = target.getBoundingClientRect();
          const host = hostSelector
            ? target.closest(hostSelector)
            : target.parentElement;
          const hostRect = host.getBoundingClientRect();
          samples.push({
            at: performance.now() - started,
            left: rect.left,
            top: rect.top,
            width: rect.width,
            height: rect.height,
            host: host.dataset.flipTab ?? null,
            hostLeft: hostRect.left,
            hostWidth: hostRect.width,
            hostHeight: hostRect.height,
            transform: getComputedStyle(target).transform,
            layoutHeight: target.offsetHeight,
          });
        }
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      return samples;
    },
    { selector, hostSelector, action, frames },
  );
}

/** Wait for the transform to be dropped, i.e. for the flip to be over. */
async function waitForSettled(page, selector, timeoutMs = 3000) {
  return page.evaluate(
    async ({ selector, timeoutMs }) => {
      const started = performance.now();
      for (;;) {
        const element = document.querySelector(selector);
        if (!element) return { settled: false, elapsed: performance.now() - started };
        if (getComputedStyle(element).transform === "none") {
          return { settled: true, elapsed: performance.now() - started };
        }
        if (performance.now() - started > timeoutMs) {
          return {
            settled: false,
            elapsed: performance.now() - started,
            transform: getComputedStyle(element).transform,
          };
        }
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
    },
    { selector, timeoutMs },
  );
}

function maxStep(samples, key) {
  let worst = 0;
  for (let i = 1; i < samples.length; i++) {
    worst = Math.max(worst, Math.abs(samples[i][key] - samples[i - 1][key]));
  }
  return worst;
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
  await page.click('[data-page-link="flip"]');
  await page.waitForSelector("[data-flip-indicator]");
  await page.waitForTimeout(400);

  // --- layoutId: the indicator changes parent ------------------------------
  const overview = await indicatorState(page);
  const activityHostLeft = await page.evaluate(
    () =>
      document.querySelector('[data-flip-tab="activity"]').getBoundingClientRect()
        .left,
  );
  const activityHostWidth = await page.evaluate(
    () =>
      document.querySelector('[data-flip-tab="activity"]').getBoundingClientRect()
        .width,
  );

  check(
    "the indicator starts inside the first tab, at its box",
    overview.host === "overview" &&
      Math.abs(overview.left - overview.hostLeft) < 1.5,
    JSON.stringify(overview),
  );
  check(
    "the tabs have different widths, so the flip scales",
    Math.abs(overview.hostWidth - activityHostWidth) > 8,
    `${overview.hostWidth.toFixed(0)} vs ${activityHostWidth.toFixed(0)}`,
  );

  const flip = await sampleFrames(
    page,
    "[data-flip-indicator]",
    "[data-flip-tab]",
    `document.querySelector('[data-flip-tab="activity"]').click()`,
  );
  const moved = flip.filter((sample) => sample.host === "activity");
  check(
    "the DOM moves to the new tab on the first frame",
    moved.length > 0 && moved[0].at < 40,
    `${moved[0]?.at.toFixed(1)}ms`,
  );
  check(
    "the new node is pinned where the old one was — no flash at the new box",
    moved.length > 0 && Math.abs(moved[0].left - overview.left) < 3,
    `${moved[0]?.left.toFixed(1)} vs ${overview.left.toFixed(1)}`,
  );
  check(
    "its layout box is already the new tab's",
    moved.length > 0 && Math.abs(moved[0].hostLeft - activityHostLeft) < 1.5,
    `${moved[0]?.hostLeft.toFixed(1)} vs ${activityHostLeft.toFixed(1)}`,
  );
  check(
    "the visual box is the old tab's while the layout box is the new one",
    moved.length > 0 && Math.abs(moved[0].width - overview.width) < 4,
    `visual ${moved[0]?.width.toFixed(1)}, old ${overview.width.toFixed(1)}`,
  );

  const total = Math.abs(activityHostLeft - overview.left);
  const worstStep = maxStep(moved, "left");
  check(
    "the travel is continuous rather than a jump",
    worstStep < total * 0.5,
    `largest frame step ${worstStep.toFixed(1)}px of ${total.toFixed(1)}px total`,
  );
  const settled = moved[moved.length - 1];
  check(
    "it lands on the new tab's box",
    Math.abs(settled.left - activityHostLeft) < 1.5,
    `${settled.left.toFixed(1)} vs ${activityHostLeft.toFixed(1)}`,
  );
  const indicatorSettle = await waitForSettled(page, "[data-flip-indicator]");
  check(
    "the transform is dropped once the travel is invisible",
    indicatorSettle.settled,
    `${indicatorSettle.elapsed.toFixed(0)}ms${indicatorSettle.transform ? `, left ${indicatorSettle.transform}` : ""}`,
  );

  await page.screenshot({ path: join(OUT, "indicator.png") });

  // --- an interrupted flip continues from where it looks --------------------
  const interrupted = await page.evaluate(async () => {
    const activity = document.querySelector('[data-flip-tab="activity"]');
    const settings = document.querySelector('[data-flip-tab="settings"]');
    activity.click();
    // Let it get properly under way, then redirect it mid-flight.
    await new Promise((resolve) => setTimeout(resolve, 90));
    const before = document
      .querySelector("[data-flip-indicator]")
      .getBoundingClientRect().left;
    settings.click();
    const immediately = document
      .querySelector("[data-flip-indicator]")
      .getBoundingClientRect().left;
    const frames = [];
    for (let i = 0; i < 60; i++) {
      frames.push(
        document.querySelector("[data-flip-indicator]").getBoundingClientRect()
          .left,
      );
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    return { before, immediately, frames };
  });
  const settingsLeft = await page.evaluate(
    () =>
      document.querySelector('[data-flip-tab="settings"]').getBoundingClientRect()
        .left,
  );
  check(
    "an interrupted flip continues from where the element looks",
    Math.abs(interrupted.immediately - interrupted.before) < 12,
    `${interrupted.before.toFixed(1)} → ${interrupted.immediately.toFixed(1)}`,
  );
  check(
    "and does not jump to the new box on the way",
    maxStep(
      interrupted.frames.map((left) => ({ left })),
      "left",
    ) <
      Math.abs(settingsLeft - interrupted.before) * 0.5,
    `largest step ${maxStep(
      interrupted.frames.map((left) => ({ left })),
      "left",
    ).toFixed(1)}px`,
  );
  await page.waitForTimeout(700);
  check(
    "it still lands on the tab it was sent to",
    Math.abs((await indicatorState(page)).left - settingsLeft) < 1.5,
    String((await indicatorState(page)).left),
  );
  check(
    "the tab selection followed the clicks",
    (await probe(page, "tab")) === "settings",
    String(await probe(page, "tab")),
  );

  // --- layout: one element whose box changes -------------------------------
  // Scroll first and measure after: a flip is pinned in viewport coordinates, so
  // scrolling between the "before" reading and the samples would move both.
  await page.locator("[data-flip-size-toggle]").scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  const before = await page.evaluate(() => {
    const box = document.querySelector("[data-flip-box]");
    return { height: box.getBoundingClientRect().height, layout: box.offsetHeight };
  });

  const grown = await sampleFrames(
    page,
    "[data-flip-box]",
    null,
    `document.querySelector('[data-flip-size-toggle]').click()`,
  );
  // The first sample is taken in the same task as the click, before Dioxus has
  // committed the render that changes the class; the flip starts on the first
  // sample whose layout height is the new one.
  const changed = grown.filter((sample) => sample.layoutHeight !== before.layout);
  check(
    "the layout height jumps to the new value within a frame",
    changed.length > 0 && changed[0].at < 40,
    `${changed[0]?.at.toFixed(1)}ms, ${changed[0]?.layoutHeight}px`,
  );
  check(
    "the visual height is still the old one on that frame",
    changed.length > 0 && Math.abs(changed[0].height - before.height) < 3,
    `visual ${changed[0]?.height.toFixed(1)} vs before ${before.height.toFixed(1)}`,
  );
  const target = grown.reduce(
    (height, sample) => Math.max(height, sample.layoutHeight),
    0,
  );
  const grownStep = maxStep(changed, "height");
  check(
    "the height grows continuously",
    grownStep < (target - before.height) * 0.5,
    `largest frame step ${grownStep.toFixed(1)}px of ${(target - before.height).toFixed(1)}px`,
  );
  const grownLast = grown[grown.length - 1];
  check(
    "it settles on the new height",
    Math.abs(grownLast.height - grownLast.layoutHeight) < 1.5,
    `${grownLast.height.toFixed(1)} vs ${grownLast.layoutHeight.toFixed(1)}`,
  );
  const boxSettle = await waitForSettled(page, "[data-flip-box]");
  check(
    "the transform is dropped once the travel is invisible",
    boxSettle.settled,
    `${boxSettle.elapsed.toFixed(0)}ms`,
  );
  await page.screenshot({ path: join(OUT, "box.png") });

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

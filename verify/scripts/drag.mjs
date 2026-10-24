// Drag verification: does a drag follow the pointer, damp by the elastic
// factor beyond its bounds, and throw with the velocity it was released at?
//
// These are the checks a screenshot cannot make — the elastic ratio is a number,
// and the throw is a function of the pointer's speed, not of where the pointer
// stopped. Both are read out of the DOM: the element's transform is the offset
// the drag renders.
//
// The velocity the throw uses is sampled per *frame*, so a "slow" release has to
// pause before letting go (the tracker reports zero once the value has been
// still for 30ms, exactly as motion's `MotionValue.getVelocity` does) and a
// "fast" one has to move in one frame.
//
// Usage: node scripts/drag.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(
  process.argv[2] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/drag");

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
  page.getAttribute(`[data-probe="${name}"]`, "data-probe-value").then(Number);
const probeText = (page, name) =>
  page.getAttribute(`[data-probe="${name}"]`, "data-probe-value");

/** The rendered offset, read back out of the transform. */
async function offset(page, selector) {
  return page.evaluate((selector) => {
    const element = document.querySelector(selector);
    if (!element) return null;
    const transform = getComputedStyle(element).transform;
    if (transform === "none") return { x: 0, y: 0 };
    const parts = transform.match(/matrix.*\((.+)\)/)?.[1].split(",") ?? [];
    return { x: Number(parts[4] ?? 0), y: Number(parts[5] ?? 0) };
  }, selector);
}

/**
 * Drag from the middle of `selector` by `(dx, dy)`.
 *
 * `speed` is `"slow"` (many small waits, then a pause so the release has no
 * velocity) or `"fling"` (one frame, so the release carries the pointer's).
 */
async function dragBy(page, selector, dx, dy, speed) {
  // `page.mouse` works in viewport coordinates and does not scroll, so the
  // element has to be on screen before its box is read.
  await page.locator(selector).scrollIntoViewIfNeeded();
  const box = await page.locator(selector).boundingBox();
  const x = box.x + box.width / 2;
  const y = box.y + Math.min(box.height / 2, 20) + 4;
  await page.mouse.move(x, y);
  await page.mouse.down();
  if (speed === "fling") {
    // Two moves in one frame: the tracker sees the whole displacement as one
    // frame's worth of motion.
    await page.mouse.move(x + dx * 0.5, y + dy * 0.5);
    await page.mouse.move(x + dx, y + dy);
    await page.mouse.up();
  } else {
    // Slow enough to stay under every fling threshold, then a pause long enough
    // (motion's 30ms staleness window) that the release carries no velocity.
    const steps = 12;
    for (let step = 1; step <= steps; step++) {
      await page.mouse.move(x + (dx * step) / steps, y + (dy * step) / steps);
      await page.waitForTimeout(50);
    }
    await page.waitForTimeout(120);
    await page.mouse.up();
  }
  return { x, y };
}

/**
 * Flick `selector` by `(dx, dy)` with both moves in one task.
 *
 * `page.mouse` awaits a round trip per move, and in a debug wasm build the page
 * takes tens of milliseconds to re-render between two of them — so a "fling"
 * written with it is delivered at a speed no finger ever moves at. A real fast
 * pointer delivers several moves per frame, which is exactly what this does, and
 * it is the case the per-frame velocity tracker is written for.
 */
async function flick(page, selector, dx, dy) {
  await page.locator(selector).scrollIntoViewIfNeeded();
  const box = await page.locator(selector).boundingBox();
  await page.evaluate(
    ({ selector, dx, dy, x, y }) => {
      const element = document.querySelector(selector);
      const at = (clientX, clientY) => ({
        bubbles: true,
        cancelable: true,
        pointerId: 7,
        pointerType: "mouse",
        isPrimary: true,
        buttons: 1,
        button: 0,
        clientX,
        clientY,
      });
      element.dispatchEvent(new PointerEvent("pointerdown", at(x, y)));
      element.dispatchEvent(
        new PointerEvent("pointermove", at(x + dx * 0.5, y + dy * 0.5)),
      );
      element.dispatchEvent(new PointerEvent("pointermove", at(x + dx, y + dy)));
      element.dispatchEvent(new PointerEvent("pointerup", at(x + dx, y + dy)));
    },
    { selector, dx, dy, x: box.x + box.width / 2, y: box.y + 20 },
  );
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
  await page.click('[data-page-link="drag"]');
  await page.waitForSelector('[data-sheet="true"]');
  await page.waitForTimeout(300);

  // --- bottom sheet: asymmetric elasticity ---------------------------------
  check(
    "the sheet starts at rest",
    (await probe(page, "sheet-offset")) === 0,
    String(await probe(page, "sheet-offset")),
  );

  // 100px of finger: past the elastic bound but under the 120px dismiss
  // threshold, which reads the raw offset.
  await dragBy(page, '[data-sheet="true"]', 0, 100, "slow");
  const sheetDown = await probe(page, "sheet-offset");
  check(
    "pulled down it keeps 40% of the finger's travel",
    Math.abs(sheetDown - 40) < 6,
    `${sheetDown} for 100px (expected ~40)`,
  );
  await page.waitForTimeout(900);
  check(
    "released without a fling it springs back to the only legal position",
    Math.abs(await probe(page, "sheet-offset")) < 1,
    String(await probe(page, "sheet-offset")),
  );
  check(
    "and the component was told what happened",
    (await probeText(page, "sheet-outcome")) === "sprang back",
    String(await probeText(page, "sheet-outcome")),
  );

  await page.click("[data-sheet-reset]");
  await page.waitForTimeout(150);
  await dragBy(page, '[data-sheet="true"]', 0, -100, "slow");
  const sheetUp = await probe(page, "sheet-offset");
  check(
    "pulled up it keeps 2%",
    Math.abs(sheetUp + 2) < 3,
    `${sheetUp} for -100px (expected ~-2)`,
  );
  await page.waitForTimeout(900);
  await page.click("[data-sheet-reset]");
  await page.waitForTimeout(150);

  // A fling does not need distance: 100px of travel at speed is enough.
  await dragBy(page, '[data-sheet="true"]', 0, 100, "fling");
  await page.waitForTimeout(200);
  check(
    "a downward fling dismisses it",
    (await probeText(page, "sheet-open")) === "false",
    `${await probeText(page, "sheet-outcome")}, open=${await probeText(page, "sheet-open")}`,
  );
  await page.click("[data-sheet-reset]");
  await page.waitForTimeout(150);

  // --- toast: the threshold reads the raw offset, not the damped one --------
  check(
    "the toast starts at rest",
    (await probe(page, "toast-x")) === 0,
    String(await probe(page, "toast-x")),
  );

  await dragBy(page, '[data-toast="true"]', 40, 0, "slow");
  check(
    "dragged 40px it shows 18% of the travel",
    Math.abs((await probe(page, "toast-x")) - 7.2) < 1,
    `${await probe(page, "toast-x")} for 40px (expected ~7.2)`,
  );
  check(
    "a slow 40px release does not dismiss it",
    (await probeText(page, "toast-visible")) === "true",
    String(await probeText(page, "toast-outcome")),
  );
  await page.waitForTimeout(600);

  // 80px of finger, 14px of card: the threshold sees the finger.
  await dragBy(page, '[data-toast="true"]', 80, 0, "slow");
  check(
    "80px of finger dismisses it although the card only showed 14px",
    (await probeText(page, "toast-visible")) === "false",
    `${await probeText(page, "toast-outcome")} (card at ${await probe(page, "toast-x")})`,
  );
  await page.click("[data-toast-reset]");
  await page.waitForTimeout(150);

  // A fast short flick dismisses on velocity alone: 40px of finger, well under
  // the 72px threshold.
  await flick(page, '[data-toast="true"]', 40, 0);
  await page.waitForTimeout(80);
  check(
    "a 40px flick dismisses on velocity alone",
    (await probeText(page, "toast-visible")) === "false",
    String(await probeText(page, "toast-outcome")),
  );
  await page.click("[data-toast-reset]");
  await page.waitForTimeout(150);

  // --- swipeable row: free inside, 4% beyond, snaps on release -------------
  check(
    "the row starts at rest",
    (await probe(page, "row-x")) === 0,
    String(await probe(page, "row-x")),
  );

  await dragBy(page, '[data-row="true"]', 60, 0, "slow");
  check(
    "inside the action width it follows the finger exactly",
    Math.abs((await probe(page, "row-x")) - 60) < 4,
    `${await probe(page, "row-x")} for 60px`,
  );
  await page.waitForTimeout(500);

  await dragBy(page, '[data-row="true"]', 300, 0, "slow");
  const rowBeyond = await probe(page, "row-x");
  // 96 free + 4% of the 204 beyond.
  check(
    "past the bound only 4% of the excess moves",
    Math.abs(rowBeyond - (96 + (300 - 96) * 0.04)) < 8,
    `${rowBeyond} for 300px (expected ~104)`,
  );
  await page.waitForTimeout(600);
  check(
    "released past half it snaps open",
    Math.abs((await probe(page, "row-snapped")) - 96) < 1,
    String(await probeText(page, "row-last")),
  );
  check(
    "and the offset animates to the snapped position",
    Math.abs((await probe(page, "row-x")) - 96) < 1.5,
    String(await probe(page, "row-x")),
  );

  await dragBy(page, '[data-row="true"]', -30, 0, "slow");
  await page.waitForTimeout(500);
  check(
    "released inside half it snaps closed",
    Math.abs((await probe(page, "row-x"))) < 1.5 &&
      Math.abs((await probe(page, "row-snapped"))) < 1,
    `${await probe(page, "row-x")} / snapped ${await probe(page, "row-snapped")}`,
  );

  await page.screenshot({ path: join(OUT, "drag.png") });
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

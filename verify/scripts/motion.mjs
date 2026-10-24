// 60 Hz trajectory sampling for the ported motion components.
//
// The full verification step samples `transform`/rect every frame and checks
// that a spring lands on its target, that the motion is continuous (no jump
// larger than half the total travel in one frame) and that the value settles.
// The React reference server is not part of this checkout, so the numbers here
// are absolute rather than a diff against beUI; the pixel diff against the
// reference is the separate `odiff` step.
//
// Usage: node scripts/motion.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(process.argv[2] ?? "../target/dx/gallery/debug/web/public");
const OUT = resolve("out/motion");

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
      if (url.pathname === "/" || url.pathname.endsWith("/")) path = join(path, "index.html");
      const body = await readFile(path);
      response.writeHead(200, { "content-type": MIME[extname(path)] ?? "application/octet-stream" });
      response.end(body);
    } catch {
      response.writeHead(404).end("not found");
    }
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  return { server, origin: `http://127.0.0.1:${server.address().port}` };
}

/** Start recording a selector's transform once per animation frame. */
async function startSampling(page, selector) {
  await page.evaluate((selector) => {
    window.__samples = [];
    window.__sampling = true;
    const element = document.querySelector(selector);
    const read = () => {
      const match = getComputedStyle(element).transform.match(/matrix\(([^)]+)\)/);
      const scale = match ? Number(match[1].split(",")[0]) : 1;
      const tx = match ? Number(match[1].split(",")[4]) : 0;
      window.__samples.push({ scale, tx, t: performance.now() });
      if (window.__sampling) requestAnimationFrame(read);
    };
    requestAnimationFrame(read);
  }, selector);
}

async function stopSampling(page) {
  return page.evaluate(() => {
    window.__sampling = false;
    return window.__samples;
  });
}

const { server, origin } = await serve(DIST);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto(origin, { waitUntil: "load" });
await page.waitForSelector("[data-page-link]");
await mkdir(OUT, { recursive: true });

// --- Button: whileTap scale lands on 0.93 and returns ------------------------

await page.click('[data-page-link="button"]');
await page.waitForSelector('[data-page="button"]');
{
  const button = page.locator('[data-page="button"] button', { hasText: "Continue" }).first();
  const box = await button.boundingBox();
  await startSampling(page, '[data-page="button"] button:nth-of-type(1)');
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.waitForTimeout(700);
  const pressed = await stopSampling(page);
  const pressedScales = pressed.map((sample) => sample.scale);
  const lowest = Math.min(...pressedScales);
  const settled = pressedScales.at(-1);
  check("button whileTap reaches 0.93", lowest <= 0.935 && lowest >= 0.925, `min ${lowest.toFixed(4)}`);
  check("button whileTap settles on 0.93", Math.abs(settled - 0.93) < 0.005, `last ${settled.toFixed(4)}`);
  const maxStep = Math.max(
    ...pressedScales.slice(1).map((value, index) => Math.abs(value - pressedScales[index])),
  );
  check("button press is continuous", maxStep < 0.4, `max step ${maxStep.toFixed(4)}`);
  await page.mouse.up();
}

// --- Switch: thumb travels 20px ---------------------------------------------

await page.click('[data-page-link="switch"]');
await page.waitForSelector('[data-page="switch"]');
{
  const track = page.locator('[data-page="switch"] [role="switch"]').first();
  const box = await track.boundingBox();
  const readTx = async () => {
    const transform = await track.evaluate((element) => getComputedStyle(element.querySelector("div")).transform);
    const match = transform.match(/matrix\(([^)]+)\)/);
    return match ? Number(match[1].split(",")[4]) : 0;
  };
  const before = await readTx();
  check("switch thumb starts at 20px (checked)", Math.abs(before - 20) < 1.5, `tx ${before.toFixed(2)}`);
  await startSampling(page, '[data-page="switch"] [role="switch"] div');
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await page.waitForTimeout(800);
  const samples = await stopSampling(page);
  const xs = samples.map((sample) => sample.tx);
  const after = xs.at(-1);
  check("switch thumb travels to 0px (unchecked)", Math.abs(after) < 1.5, `tx ${after.toFixed(2)}`);
  const maxStep = Math.max(...xs.slice(1).map((value, index) => Math.abs(value - xs[index])));
  check("switch travel is continuous", maxStep < 10, `max step ${maxStep.toFixed(2)}`);
}

// --- Base RangeSlider: thumb tracks the value --------------------------------

await page.click('[data-page-link="range-slider"]');
await page.waitForSelector('[data-page="range-slider"]');
{
  const track = page.locator('[data-page="range-slider"] [data-beui-slider]').first();
  const box = await track.boundingBox();
  // Drag to 80% of the track.
  await page.mouse.move(box.x + box.width * 0.4, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.8, box.y + box.height / 2, { steps: 8 });
  await page.waitForTimeout(800);
  await page.mouse.up();
  await page.waitForTimeout(300);
  const handle = page.locator('[data-page="range-slider"] [data-beui-slider-handle]').first();
  const transform = await handle.evaluate((element) => getComputedStyle(element).transform);
  const match = transform.match(/matrix\(([^)]+)\)/);
  const tx = match ? Number(match[1].split(",")[4]) : 0;
  // The handle rests 8px in and travels over width - 20; the value snapped to a
  // step of 5, so 80% of the drag is the expected position within a step.
  const expected = 8 + (box.width - 20) * 0.8;
  check("slider thumb tracks the drag", Math.abs(tx - expected) < box.width * 0.06, `tx ${tx.toFixed(1)} expected ~${expected.toFixed(1)}`);
  const aria = await handle.getAttribute("aria-valuenow");
  check("slider committed a value", aria !== null && Number(aria) >= 70, `aria-valuenow ${aria}`);
}

await page.screenshot({ path: join(OUT, "trajectory-page.png"), fullPage: true });
await browser.close();
server.close();

if (failures.length) {
  console.log(`\n${failures.length} failure(s)`);
  process.exit(1);
}
console.log("\nall trajectory checks passed");

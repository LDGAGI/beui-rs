// Switch — the four ways this component's port can drift from
// `components/motion/switch.tsx`, each measured on the built gallery.
//
//   1. Travel. The port animates `translateX` where the source re-aligns flex
//      and lets `layout` animate the jump. That only works while the track
//      stays `justify-start`: the source's `justify-end` on the checked track
//      places the thumb *and* the translate moves it, so it lands 20px further
//      right — 16px of it outside the track, on top of the label. The unchecked
//      state cannot show this, because its translate is 0.
//   2. Squish centring. `scale: 0.9` must shrink about the circle's own centre.
//      The source puts the `ml-1`/`mr-1` stretch classes on an inner `size-5`
//      spacer; on the painted circle they drag it sideways instead.
//   3. Shake timing. The source's disabled shake is `delay 0.2 + duration 0.6`
//      over five values, so its keyframes sit at 200/350/500/650/800ms. Even
//      spacing across a single 800ms clip runs the whole shake 240ms fast.
//   4. Mount and keyboard. `initial={false}` must not slide the thumb in, and
//      the squish is gated on `pointerdown`, so Space must not squish.
//
// Usage: node scripts/switch.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(process.argv[2] ?? "../target/dx/gallery/debug/web/public");
const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".json": "application/json",
};

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

const failures = [];
const check = (ok, label, detail) => {
  console.log(`${ok ? "PASS" : "FAIL"}  ${label}${detail ? `  — ${detail}` : ""}`);
  if (!ok) failures.push(label);
};

const { server, origin } = await serve(DIST);
const browser = await chromium.launch();
const page = await browser.newPage({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 2,
});

const errors = [];
const IGNORE = /_dioxus|WebSocket|404/;
page.on("console", (m) => {
  if (m.type() === "error" && !IGNORE.test(m.text())) errors.push(m.text());
});
page.on("pageerror", (e) => errors.push(String(e)));

// The gallery is a client-routed SPA; a static server cannot serve `/switch`.
const open = async () => {
  await page.goto(origin, { waitUntil: "load" });
  await page.waitForSelector("[data-page-link]", { timeout: 20_000 });
  await page.click('[data-page-link="switch"]');
  await page.waitForSelector('[data-page="switch"]', { timeout: 20_000 });
};

// --- 1. Travel, and the mount state ---
await open();
// Sampled before any settle wait: `initial={false}` means it is already there.
const mount = await page.evaluate(() => {
  const track = document.querySelector('button[role="switch"][data-state="checked"]');
  const outer = track.querySelector("div[style]");
  const t = track.getBoundingClientRect();
  const o = outer.getBoundingClientRect();
  return { offset: +(o.left - t.left).toFixed(2), transform: getComputedStyle(outer).transform };
});
check(
  Math.abs(mount.offset - 24) < 1,
  "mount: checked thumb starts at its target",
  `offset ${mount.offset} (want 24), matching initial={false}`,
);

await page.waitForTimeout(2000);
const geometry = await page.evaluate(() =>
  [...document.querySelectorAll('button[role="switch"]')].map((track) => {
    const outer = track.querySelector("div[style]");
    const t = track.getBoundingClientRect();
    const o = outer.getBoundingClientRect();
    const label = track.parentElement.querySelector("label");
    const l = label?.getBoundingClientRect();
    return {
      state: `${track.dataset.state}${track.disabled ? ",disabled" : ""}`,
      offset: +(o.left - t.left).toFixed(2),
      thumbW: +o.width.toFixed(2),
      trackW: +t.width.toFixed(2),
      labelGap: l ? +(l.left - (t.left + t.width)).toFixed(2) : null,
    };
  }),
);
for (const g of geometry) {
  const want = g.state.startsWith("checked") ? 24 : 4;
  check(
    Math.abs(g.offset - want) < 1 &&
      Math.abs(g.trackW - 48) < 1 &&
      Math.abs(g.thumbW - 20) < 1,
    `geometry: [${g.state}] thumb offset`,
    `offset ${g.offset} (want ${want}), track ${g.trackW}, thumb ${g.thumbW}`,
  );
  check(
    g.labelGap !== null && g.labelGap >= 8,
    `geometry: [${g.state}] clears the label`,
    `gap ${g.labelGap}px (want >= 8)`,
  );
}

// --- 2. Squish centring ---
const circle = () =>
  page.evaluate(() => {
    const track = document.querySelector('button[role="switch"]');
    const c = track.querySelector("div.rounded-full").getBoundingClientRect();
    const t = track.getBoundingClientRect();
    return +(c.left + c.width / 2 - t.left).toFixed(2);
  });
const restCentre = await circle();
const track = await page.$('button[role="switch"]');
const box = await track.boundingBox();
await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
await page.mouse.down();
await page.waitForTimeout(500);
const pressedCentre = await circle();
const scale = await page.evaluate(
  () => getComputedStyle(document.querySelector('button[role="switch"] div[style]')).transform,
);
await page.mouse.up();
await page.waitForTimeout(1200);
check(
  Math.abs(pressedCentre - restCentre) < 1,
  "squish: shrinks about the thumb's own centre",
  `centre ${restCentre} -> ${pressedCentre} while pressed`,
);
check(/matrix\(0\.9,/.test(scale), "squish: press scales the thumb to 0.9", scale);

// --- 3. Shake timing ---
await open();
await page.waitForTimeout(1000);
const disabledTrack = await page.$('button[role="switch"][disabled]');
const dbox = await disabledTrack.boundingBox();

// Sample in-page on animation frames: the source's peak is a single instant, so
// polling from Node at ~20ms misses it and understates the amplitude.
await page.evaluate(() => {
  const track = document.querySelector('button[role="switch"][disabled]');
  const shake = track.querySelector(".rounded-full");
  window.__shake = [];
  const started = performance.now();
  const tick = () => {
    const m = new DOMMatrixReadOnly(getComputedStyle(shake).transform);
    window.__shake.push({ t: performance.now() - started, x: +m.m41.toFixed(3) });
    if (performance.now() - started < 1100) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
});
await page.mouse.move(dbox.x + dbox.width / 2, dbox.y + dbox.height / 2);
await page.mouse.down();
await page.waitForTimeout(1000);
await page.mouse.up();
await page.waitForTimeout(200);

const samples = await page.evaluate(() => window.__shake);
const xs = samples.map((s) => s.x);

// Timing is what the even-spacing bug got wrong, so measure the shape against
// the source's own schedule. `animate(x, [0,-2,2,-1,0], {delay: 0.2, duration:
// 0.6})` holds the first value through the delay, then spaces the five values
// evenly across the duration: the troughs and peaks land at 150/300/450/600ms
// after the motion starts. Even spacing across one 800ms clip put them at
// 200/320/440/560 instead — the first trough is the clearest discriminator.
const t0 = samples.find((s) => Math.abs(s.x) > 0.05)?.t ?? 0;
const rel = samples.filter((s) => s.t >= t0).map((s) => ({ t: s.t - t0, x: s.x }));
let firstTrough = null;
let firstPeak = null;
for (let i = 1; i < rel.length - 1; i++) {
  const [a, b, c] = [rel[i - 1].x, rel[i].x, rel[i + 1].x];
  if (firstTrough === null && b < a && b <= c) firstTrough = rel[i].t;
  if (firstTrough !== null && firstPeak === null && b > a && b >= c) {
    firstPeak = rel[i].t;
    break;
  }
}
const beforeStart = samples.filter((s) => s.t < t0 - 20);

check(
  beforeStart.length > 4 && beforeStart.every((s) => Math.abs(s.x) < 0.05),
  "shake: holds still before the motion starts (the 0.2s delay)",
  `${beforeStart.length} still frames before ${Math.round(t0)}ms`,
);
// The keyframe values are ±2, but each is a single instant: a 60Hz frame grid
// lands near the peak, never exactly on it, so the sampled extremes alone are
// grid-dependent (they ranged 1.76..1.95 across runs). The span between them is
// not — both extrema are sampled close to their instants — so assert on that,
// which still fails hard if the amplitude regresses.
const span = Math.max(...xs) - Math.min(...xs);
check(
  span >= 3.4,
  "shake: full -2..+2 excursion",
  `span ${span.toFixed(2)} (want >= 3.4; keyframes are ±2, a half-amplitude bug gives ~1.85)`,
);
check(
  firstTrough !== null && firstTrough >= 115 && firstTrough <= 185,
  "shake: first trough at the source's 150ms",
  `trough at ${firstTrough === null ? "none" : Math.round(firstTrough)}ms ` +
    `(even spacing would put it at 200ms)`,
);
check(
  firstPeak !== null && firstPeak >= 250 && firstPeak <= 350,
  "shake: peak at the source's 300ms",
  `peak at ${firstPeak === null ? "none" : Math.round(firstPeak)}ms`,
);

// --- 4. Keyboard does not squish ---
await open();
await page.waitForTimeout(800);
const first = await page.$('button[role="switch"]');
await first.focus();
await page.keyboard.down("Space");
await page.waitForTimeout(200);
const spaceScale = await page.evaluate(
  () => getComputedStyle(document.querySelector('button[role="switch"] div[style]')).transform,
);
await page.keyboard.up("Space");
check(
  /matrix\(1,/.test(spaceScale),
  "keyboard: Space does not squish (squish is gated on pointerdown)",
  spaceScale,
);

// --- Reduced motion keeps the geometry, drops the movement ---
await page.emulateMedia({ reducedMotion: "reduce" });
await open();
await page.waitForTimeout(800);
const reduced = await page.evaluate(() => {
  const track = document.querySelector('button[role="switch"]');
  const outer = track.querySelector("div[style]");
  const t = track.getBoundingClientRect();
  const o = outer.getBoundingClientRect();
  return { state: track.dataset.state, offset: +(o.left - t.left).toFixed(2) };
});
check(
  Math.abs(reduced.offset - 24) < 1,
  "reduced motion: thumb still lands on its target",
  `[${reduced.state}] offset ${reduced.offset}`,
);
await page.emulateMedia({ reducedMotion: null });

check(errors.length === 0, "no console or page errors", errors.join("; "));

await browser.close();
server.close();
if (failures.length) {
  console.log(`\n${failures.length} failure(s): ${failures.join(", ")}`);
  process.exit(1);
}
console.log("\nall switch checks passed");

// Per-component smoke: does the page render, and does it throw?
//
// The fast lane's bar for a component is `cargo clippy` + `dx build`, but a
// build that compiles can still panic on mount (a missing context, a rejected
// animation, a bad selector). This drives the built gallery to one page, fails
// on any console error or page error, and writes a screenshot so the page can
// be eyeballed.
//
// Usage: node scripts/smoke.mjs <slug> [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const slug = process.argv[2];
const DIST = resolve(
  process.argv[3] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/smoke");

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

if (!slug) {
  console.error("usage: node scripts/smoke.mjs <slug> [dist-dir]");
  process.exit(2);
}

const { server, origin } = await serve(DIST);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

const errors = [];
// The devtools websocket is a `dx serve` feature; a static file server has no
// `/_dioxus` endpoint, so the handshake 404s. That is not a component error.
const IGNORE = /_dioxus|WebSocket/;
page.on("console", (message) => {
  if (message.type() === "error" && !IGNORE.test(message.text())) {
    errors.push(`console: ${message.text()}`);
  }
});
page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));

try {
  await page.goto(origin, { waitUntil: "load" });
  await page.waitForSelector("[data-page-link]", { timeout: 20_000 });
  await page.click(`[data-page-link="${slug}"]`);
  await page.waitForSelector(`[data-page="${slug}"]`, { timeout: 20_000 });
  await page.waitForTimeout(600);

  await mkdir(OUT, { recursive: true });
  await page.screenshot({ path: join(OUT, `${slug}.png`), fullPage: true });

  const heading = await page.textContent(`[data-page="${slug}"] h1`).catch(() => null);
  console.log(`page  ${slug}  h1=${JSON.stringify(heading)}`);

  // Geometry guards for components whose layout is a fixed track. `switch`
  // animates the thumb with a translate, so a flex alignment that also places
  // it (the source's `justify-end`) would move it twice and push it out of the
  // track — invisible in the unchecked state, where the translate is 0.
  if (slug === "switch") {
    const geo = await page.evaluate(() =>
      [...document.querySelectorAll('button[role="switch"]')].map((track) => {
        const outer = track.querySelector("div[style]");
        const t = track.getBoundingClientRect();
        const o = outer.getBoundingClientRect();
        const label = track.parentElement.querySelector("label");
        const l = label?.getBoundingClientRect();
        return {
          state: track.dataset.state,
          offset: +(o.left - t.left).toFixed(2),
          thumbW: +o.width.toFixed(2),
          trackW: +t.width.toFixed(2),
          labelGap: l ? +(l.left - (t.left + t.width)).toFixed(2) : null,
        };
      }),
    );
    for (const g of geo) {
      // The thumb must sit inside the track: right edge at or before the end.
      if (g.offset + g.thumbW > g.trackW + 0.5) {
        errors.push(
          `geometry: [${g.state}] thumb ends at ${(g.offset + g.thumbW).toFixed(2)}px, ` +
            `past the ${g.trackW}px track`,
        );
      }
      // ...and clear of the label.
      if (g.labelGap !== null && g.labelGap < 8) {
        errors.push(
          `geometry: [${g.state}] thumb-to-label gap ${g.labelGap}px, want >= 8px`,
        );
      }
    }
    console.log(`geo   switch  ${JSON.stringify(geo)}`);
  }
} catch (error) {
  errors.push(`navigation: ${error.message}`);
} finally {
  await browser.close();
  server.close();
}

if (errors.length) {
  for (const error of errors) console.log(`FAIL ${error}`);
  process.exit(1);
}
console.log(`ok   ${slug} — no console or page errors`);

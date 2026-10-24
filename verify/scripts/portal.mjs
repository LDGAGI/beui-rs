// Portal verification: does an overlay really escape its ancestors, and does it
// stack the way the source's z classes say it should?
//
// The checks are geometric on purpose. "It is a child of body" is only half of
// what a portal is for; the half that matters is that `position: fixed` resolves
// against the viewport instead of a transformed ancestor, and that nothing above
// it clips it. Both are read as rectangles and hit tests, and both have a
// negative control in the same stage — the identical element, not portaled.
//
// Usage: node scripts/portal.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(
  process.argv[2] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/portal");

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

const rectOf = (page, selector) =>
  page.evaluate((selector) => {
    const element = document.querySelector(selector);
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    return {
      left: rect.left,
      top: rect.top,
      width: rect.width,
      height: rect.height,
      parent: element.parentElement?.tagName.toLowerCase() ?? null,
      portaled: element.hasAttribute("data-beui-portaled"),
    };
  }, selector);

/** What is on top at a viewport point, and is it inside `selector`? */
const hitTest = (page, x, y, selector) =>
  page.evaluate(
    ({ x, y, selector }) => {
      const hit = document.elementFromPoint(x, y);
      const owner = document.querySelector(selector);
      return {
        tag: hit?.tagName.toLowerCase() ?? null,
        inside: Boolean(hit && owner && (owner === hit || owner.contains(hit))),
        text: hit?.textContent?.trim().slice(0, 24) ?? null,
      };
    },
    { x, y, selector },
  );

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
  await page.click('[data-page-link="portal"]');
  await page.waitForSelector('[data-portal-fixed="true"]');
  await page.waitForTimeout(300);

  // --- escaping a transformed, clipping ancestor ---------------------------
  const stage = await rectOf(page, "[data-portal-stage]");
  const inline = await rectOf(page, "[data-portal-inline]");
  const portaled = await rectOf(page, "[data-portal-fixed]");

  check(
    "the portaled element is a child of body",
    portaled.parent === "body" && portaled.portaled,
    `parent ${portaled.parent}, portaled ${portaled.portaled}`,
  );
  check(
    "the inline control stays where it was rendered",
    inline.parent !== "body" && !inline.portaled,
    `parent ${inline.parent}`,
  );
  check(
    "the inline box is positioned against the stage, not the viewport",
    Math.abs(inline.left - (stage.left + 40)) < 2,
    `inline at ${inline.left.toFixed(0)}, stage at ${stage.left.toFixed(0)} + 40`,
  );
  check(
    "the portaled box is positioned against the viewport",
    Math.abs(portaled.left - 40) < 2,
    `${portaled.left.toFixed(0)} (expected 40)`,
  );
  check(
    "the portaled box is not clipped by the stage",
    (await hitTest(page, portaled.left + portaled.width - 8, portaled.top + 8, "[data-portal-fixed]")).inside,
    JSON.stringify(
      await hitTest(
        page,
        portaled.left + portaled.width - 8,
        portaled.top + 8,
        "[data-portal-fixed]",
      ),
    ),
  );
  check(
    "the inline box is clipped at the stage's edge",
    !(await hitTest(page, inline.left + inline.width - 8, inline.top + 8, "[data-portal-inline]")).inside,
    `stage ends at ${(stage.left + stage.width).toFixed(0)}, inline ends at ${(inline.left + inline.width).toFixed(0)}`,
  );

  await page.screenshot({ path: join(OUT, "escape.png") });

  // --- stacking: z-index decides, not render order -------------------------
  const overlap = { x: 320 + 80, y: 40 + 40 };
  // The panel opens with the lower layer rendered first, so the higher one is
  // mounted second: body order and stack order disagree from the start.
  const first = await page.getAttribute('[data-portal-z="high"]', "data-portal-z-order");
  const topFirst = await hitTest(page, overlap.x, overlap.y, '[data-portal-z="high"]');
  check(
    "with the higher layer rendered second, it is on top",
    first === "second" && topFirst.inside,
    `order ${first}, hit ${JSON.stringify(topFirst)}`,
  );

  await page.click("[data-portal-z-flip]");
  await page.waitForTimeout(200);
  const second = await page.getAttribute('[data-portal-z="high"]', "data-portal-z-order");
  const topSecond = await hitTest(page, overlap.x, overlap.y, '[data-portal-z="high"]');
  check(
    "with the render order flipped, it is still on top",
    second === "first" && topSecond.inside,
    `order ${second}, hit ${JSON.stringify(topSecond)}`,
  );
  check(
    "both layers ended up portaled",
    (await rectOf(page, '[data-portal-z="high"]')).parent === "body" &&
      (await rectOf(page, '[data-portal-z="low"]')).parent === "body",
    `${(await rectOf(page, '[data-portal-z="high"]')).parent} / ${(await rectOf(page, '[data-portal-z="low"]')).parent}`,
  );
  await page.screenshot({ path: join(OUT, "stack.png") });

  // --- a modal scope over a portaled panel ---------------------------------
  const bodyChildrenBefore = await page.evaluate(
    () => document.body.children.length,
  );
  const rootInertBefore = await page.evaluate(
    () => document.querySelector("#main")?.inert ?? null,
  );
  check(
    "the app root is not inert to begin with",
    rootInertBefore === false,
    String(rootInertBefore),
  );

  await page.click("[data-portal-modal-open]");
  await page.waitForTimeout(200);
  const modal = await rectOf(page, "[data-portal-modal]");
  check(
    "the modal is portaled",
    modal.parent === "body" && modal.portaled,
    `parent ${modal.parent}`,
  );
  check(
    "the app root is inert while the portaled modal is open",
    await page.evaluate(() => document.querySelector("#main")?.inert ?? null),
  );
  check(
    "the modal's own frame is not inert",
    (await rectOf(page, "[data-portal-modal]")) !== null &&
      !(await page.evaluate(
        () => document.querySelector("[data-portal-modal]").inert,
      )),
  );
  check(
    "body gained exactly one child",
    (await page.evaluate(() => document.body.children.length)) ===
      bodyChildrenBefore + 1,
    `${bodyChildrenBefore} → ${await page.evaluate(() => document.body.children.length)}`,
  );
  await page.screenshot({ path: join(OUT, "modal.png") });

  await page.keyboard.press("Escape");
  await page.waitForTimeout(250);
  check(
    "closing it releases the app root",
    (await page.evaluate(() => document.querySelector("#main")?.inert ?? null)) ===
      false,
  );
  check(
    "and removes the portaled element from body",
    (await page.evaluate(() => document.body.children.length)) ===
      bodyChildrenBefore,
    `${await page.evaluate(() => document.body.children.length)} vs ${bodyChildrenBefore}`,
  );
  check(
    "the selection was reported to the page",
    (await page.getAttribute('[data-probe="portal-modal-open"]', "data-probe-value")) ===
      "false",
  );

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

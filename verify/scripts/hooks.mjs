// Batch-0 hook verification: the four `lib/hooks/` primitives, driven through
// the browser the way a user drives them.
//
// These are the checks a pixel diff cannot make — that a key moves the
// highlight rather than a position, that mounting open does not fire `onOpen`,
// that a drag past the end of the track still ends at the end, that the page
// behind a modal is really `inert` and really comes back. `foundation.mjs`
// covers the token layer; this covers behaviour.
//
// Usage: node scripts/hooks.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(
  process.argv[2] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/hooks");

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

const activeRow = (page) =>
  page.getAttribute("[data-cursor-active='true']", "data-cursor-row");

async function main() {
  await mkdir(OUT, { recursive: true });
  const { server, origin } = await serve(DIST);
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();

  // A `dx build` bundle still opens the devtools hot-reload socket, which this
  // plain static server does not implement.
  const ignored = /_dioxus/;
  const consoleErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error" && !ignored.test(message.text())) {
      consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => consoleErrors.push(String(error)));

  await page.goto(origin, { waitUntil: "networkidle" });
  await page.click('[data-page-link="hooks"]');
  await page.waitForSelector("[data-cursor-list]");
  await page.evaluate(() => document.fonts.ready);
  // The capability hooks answer over the eval channel a tick after mount, and
  // the slider measures itself through an effect.
  await page.waitForTimeout(400);

  // --- useRowCursor ---------------------------------------------------------
  const input = page.locator("[data-cursor-input]");
  await input.click();
  check("the highlight starts on the first row", (await activeRow(page)) === "Alpha");

  await page.keyboard.press("ArrowDown");
  check("ArrowDown moves to the second row", (await activeRow(page)) === "Bravo");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  check("two more presses move two more rows", (await activeRow(page)) === "Delta");
  await page.keyboard.press("ArrowDown");
  check("the end of the list clamps", (await activeRow(page)) === "Echo");
  await page.keyboard.press("ArrowUp");
  check("ArrowUp steps back", (await activeRow(page)) === "Delta");

  await page.keyboard.press("Enter");
  check("Enter commits the highlighted row", (await probe(page, "chosen")) === "Delta");

  // A new query drops the cursor: it holds a row id stamped with the query it
  // was placed under, so the highlight returns to the first row of the new list
  // rather than landing on whatever row now sits at that index.
  await input.fill("e");
  await page.waitForTimeout(50);
  check(
    "typing re-filters and resets the highlight",
    (await probe(page, "rows")) === "3" && (await activeRow(page)) === "Charlie",
    `rows=${await probe(page, "rows")} active=${await activeRow(page)}`,
  );

  await input.fill("");
  await page.waitForTimeout(50);
  check("clearing the query restores every row", (await probe(page, "rows")) === "5");

  // Enter with no cursor behind it must not commit a stale row: the cursor was
  // dropped when the query changed, and `active` is the first row of the new
  // list, which is the row Enter is then about to commit.
  await input.fill("e");
  await page.waitForTimeout(50);
  await page.keyboard.press("Enter");
  check(
    "Enter after a query change commits the first row of the new list",
    (await probe(page, "chosen")) === "Charlie",
  );
  await input.fill("");
  await page.screenshot({ path: join(OUT, "row-cursor.png") });

  // --- useOnOpen ------------------------------------------------------------
  const draft = page.locator("[data-on-open-draft]");
  await draft.fill("half-written");
  check("onOpen has not fired while closed", (await probe(page, "sessions")) === "0");

  await page.click("[data-on-open-toggle]");
  await page.waitForTimeout(60);
  check("opening fires onOpen once", (await probe(page, "sessions")) === "1");
  check("opening clears the draft", (await draft.inputValue()) === "");

  await draft.fill("second try");
  await page.click("[data-on-open-toggle]");
  await page.waitForTimeout(60);
  check("closing does not fire onOpen", (await probe(page, "sessions")) === "1");
  await page.click("[data-on-open-toggle]");
  await page.waitForTimeout(60);
  check("reopening fires it again", (await probe(page, "sessions")) === "2");
  check("and clears the draft again", (await draft.inputValue()) === "");
  await page.click("[data-on-open-toggle]");
  await page.screenshot({ path: join(OUT, "on-open.png") });

  // --- useSlider ------------------------------------------------------------
  const track = page.locator("[data-beui-slider]").first();
  // `page.mouse` works in viewport coordinates and does not scroll.
  await track.scrollIntoViewIfNeeded();
  const box = await track.boundingBox();
  // The label carries the scope in `data-slider-value` and the number as text;
  // the id itself is an implementation detail (a process-wide counter), so the
  // script addresses the two sliders by position.
  const value = (nth = 0) =>
    page
      .locator("[data-slider-value]")
      .nth(nth)
      .textContent()
      .then((text) => Number(text));

  check("the slider starts at its default", (await value()) === 25);

  // A click at 60% of the track — far enough from the 25 the slider starts on
  // that the check cannot pass by accident.
  await page.mouse.click(box.x + box.width * 0.6, box.y + box.height / 2);
  await page.waitForTimeout(80);
  const clicked = await value();
  check("a click at 60% lands near 60", Math.abs(clicked - 60) <= 2, String(clicked));

  // Mouse capture: dragging past the right edge still ends at the maximum.
  await page.mouse.move(box.x + box.width * 0.25, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 1.4, box.y + box.height / 2, {
    steps: 8,
  });
  await page.mouse.up();
  await page.waitForTimeout(80);
  check(
    "a drag past the right edge saturates at the maximum",
    (await value()) === 100,
    String(await value()),
  );

  await page.mouse.move(box.x + box.width * 0.5, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x - box.width * 0.4, box.y + box.height / 2, {
    steps: 8,
  });
  await page.mouse.up();
  await page.waitForTimeout(80);
  check(
    "a drag past the left edge saturates at the minimum",
    (await value()) === 0,
    String(await value()),
  );

  // Keyboard: the handle owns the keys, and the click moved focus onto it.
  const handle = page.locator("[data-beui-slider-handle]").first();
  const handleIsFocused = await page.evaluate(
    () => document.activeElement?.hasAttribute("data-beui-slider-handle") ?? false,
  );
  check("a click on the track focuses the handle", handleIsFocused);
  await page.keyboard.press("ArrowRight");
  check("ArrowRight steps by one", (await value()) === 1, String(await value()));
  await page.keyboard.press("PageUp");
  check("PageUp steps by ten", (await value()) === 11, String(await value()));
  await page.keyboard.press("End");
  check("End goes to the maximum", (await value()) === 100);
  await page.keyboard.press("Home");
  check("Home goes to the minimum", (await value()) === 0);
  check(
    "aria-valuenow follows the value",
    (await handle.getAttribute("aria-valuenow")) === "0",
  );
  check(
    "role and the accessible name are set",
    (await handle.getAttribute("role")) === "slider" &&
      (await handle.getAttribute("aria-valuetext")) === "0%",
    String(await handle.getAttribute("aria-valuetext")),
  );

  // The controlled instance reads its value from outside.
  const controlled = page.locator("[data-beui-slider]").nth(1);
  await controlled.scrollIntoViewIfNeeded();
  const controlledBox = await controlled.boundingBox();
  await page.mouse.click(
    controlledBox.x + controlledBox.width * 0.5,
    controlledBox.y + controlledBox.height / 2,
  );
  await page.waitForTimeout(80);
  const controlledHandle = page.locator("[data-beui-slider-handle]").nth(1);
  const controlledScope = await page
    .locator("[data-beui-slider-handle]")
    .nth(1)
    .getAttribute("data-beui-slider-handle");
  check(
    "a click on the controlled track focuses its handle",
    (await page.evaluate(
      () => document.activeElement?.getAttribute("data-beui-slider-handle") ?? "none",
    )) === controlledScope,
    String(await page.evaluate(() => document.activeElement?.getAttribute("data-beui-slider-handle"))),
  );
  await page.keyboard.press("ArrowRight");
  const controlledValue = Number(await controlledHandle.getAttribute("aria-valuenow"));
  check(
    "the controlled slider steps on the 0.5 grid",
    Math.abs(controlledValue - 5.5) < 1e-9,
    String(controlledValue),
  );
  await page.screenshot({ path: join(OUT, "slider.png") });

  // --- useModalScope --------------------------------------------------------
  const sibling = page.locator("[data-modal-sibling]");
  check("the page behind is not inert before opening", !(await sibling.evaluate((el) => el.inert)));

  await page.click("[data-modal-open]");
  await page.waitForTimeout(120);
  check("the modal opens", (await probe(page, "modal-open")) === "true");
  check("the page behind becomes inert", await sibling.evaluate((el) => el.inert));
  check(
    "the body scroll is locked",
    await page.evaluate(() => document.body.style.overflow === "hidden"),
  );
  check(
    "focus lands on the first action",
    await page.evaluate(
      () => document.activeElement?.dataset.modalAction ?? "none",
    ).then((v) => v === "first"),
    String(await page.evaluate(() => document.activeElement?.dataset.modalAction)),
  );

  // Tab cycles between the two actions and never leaves the panel.
  await page.keyboard.press("Tab");
  check(
    "Tab moves to the second action",
    (await page.evaluate(() => document.activeElement?.dataset.modalAction)) === "second",
  );
  await page.keyboard.press("Tab");
  check(
    "Tab wraps back to the first",
    (await page.evaluate(() => document.activeElement?.dataset.modalAction)) === "first",
  );
  await page.keyboard.press("Shift+Tab");
  check(
    "Shift+Tab wraps the other way",
    (await page.evaluate(() => document.activeElement?.dataset.modalAction)) === "second",
  );
  await page.screenshot({ path: join(OUT, "modal-open.png") });

  // Focus forced outside is pulled back. Everything outside is `inert`, so a
  // real browser cannot put focus there — the escape has to be simulated with a
  // node that was added after the scope collected its siblings.
  // `focusin` fires during `focus()`, so the pull-back is synchronous: the
  // outsider never gets to hold focus at all.
  await page.evaluate(() => {
    const outsider = document.createElement("button");
    outsider.id = "escaped-focus";
    document.body.appendChild(outsider);
    outsider.focus();
  });
  await page.waitForTimeout(60);
  const afterEscape = await page.evaluate(
    () => document.activeElement?.dataset.modalAction ?? "none",
  );
  check(
    "focus that escapes is pulled back into the panel",
    afterEscape === "first" || afterEscape === "second",
    afterEscape,
  );
  await page.evaluate(() => document.querySelector("#escaped-focus")?.remove());

  // Escape closes and the key never reaches the page behind.
  await page.keyboard.press("Escape");
  await page.waitForTimeout(120);
  check("Escape closes the modal", (await probe(page, "modal-open")) === "false");
  check("the page is interactive again", !(await sibling.evaluate((el) => el.inert)));
  check(
    "the body scroll is released",
    await page.evaluate(() => document.body.style.overflow === ""),
  );
  check(
    "focus returns to the trigger",
    await page.evaluate(
      () => document.activeElement?.hasAttribute("data-modal-open") ?? false,
    ),
  );

  // Clicking outside dismisses, through the dismiss scope rather than the modal
  // scope (Escape is the modal's; outside clicks are the dismiss scope's).
  await page.click("[data-modal-open]");
  await page.waitForTimeout(120);
  // A click on the backdrop: the panel is the scope, so the backdrop is outside.
  await page.mouse.click(20, 780);
  await page.waitForTimeout(150);
  check("a click on the backdrop dismisses", (await probe(page, "modal-open")) === "false");

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

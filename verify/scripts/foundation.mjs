// Batch-0 verification: do the tokens, palettes, fonts and capability hooks
// actually reach the browser?
//
// There is no beUI page to diff this against — beUI's foundation is
// `app/globals.css` — so the checks here are the ones a pixel diff could not
// make anyway: that a Tailwind utility resolves to the *token* rather than to a
// hard-coded colour, that a palette switch moves `--primary` to the value
// `lib/themes.ts` declares, that `dark` lands on `<html>`, that Geist is the
// font actually in use, and that `prefers-reduced-motion` reaches both the CSS
// rule and the JS hook.
//
// Usage: node scripts/foundation.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(
  process.argv[2] ?? "../target/dx/gallery/debug/web/public",
);
const OUT = resolve("out/foundation");

const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".json": "application/json",
};

// `lib/themes.ts`: the brand tokens a palette swaps in.
const PALETTES = {
  default: { light: "#0285f7", dark: "#0285f7" },
  violet: { light: "oklch(55% 0.2 290)", dark: "oklch(72% 0.16 290)" },
  "blood-orange": { light: "oklch(60% 0.19 40)", dark: "oklch(72% 0.17 42)" },
};

const TOKENS = [
  "background",
  "card",
  "primary",
  "accent",
  "violet",
  "neon",
  "success",
  "warning",
  "destructive",
];

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

/** Resolve `var(--x)` the way the browser does: read it off a live element. */
const resolveVar = (page, name) =>
  page.evaluate(
    (name) =>
      getComputedStyle(document.documentElement).getPropertyValue(name).trim(),
    name,
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

  // A `dx build` bundle still opens the devtools hot-reload socket, which this
  // plain static server does not implement. That 404 is the harness, not the app.
  const ignored = /_dioxus/;
  const consoleErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error" && !ignored.test(message.text())) {
      consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => consoleErrors.push(String(error)));

  await page.goto(origin, { waitUntil: "networkidle" });
  await page.waitForSelector("[data-token-grid]");
  await page.evaluate(() => document.fonts.ready);
  // The capability hooks answer over the eval channel a tick after mount.
  await page.waitForTimeout(300);

  // --- fonts ----------------------------------------------------------------
  const fontFamily = await page.evaluate(
    () => getComputedStyle(document.body).fontFamily,
  );
  check("body uses Geist", /Geist/.test(fontFamily), fontFamily);
  const geistLoaded = await page.evaluate(() =>
    document.fonts.check("400 16px Geist"),
  );
  check("Geist face is loaded", geistLoaded);
  const faces = await page.evaluate(() =>
    [...document.fonts].map((f) => `${f.family}|${f.unicodeRange}`),
  );
  const declared = faces.filter((face) => /Geist/.test(face)).length;
  check(
    "every declared face is registered",
    declared === 11,
    `${declared} of 11`,
  );

  // --- tokens resolve through the utility layer -----------------------------
  for (const token of TOKENS) {
    const [utility, variable] = await page.evaluate((token) => {
      const swatch = document.querySelector(`[data-token-swatch="${token}"]`);
      return [
        getComputedStyle(swatch).backgroundColor,
        getComputedStyle(document.documentElement)
          .getPropertyValue(`--${token}`)
          .trim(),
      ];
    }, token);
    // The utility must produce a real colour, and it must be the token's
    // value rather than a lookalike literal: comparing against a throwaway
    // element that carries the raw variable.
    const expected = await page.evaluate((variable) => {
      const probe = document.createElement("div");
      probe.style.backgroundColor = `var(${variable})`;
      document.body.appendChild(probe);
      const value = getComputedStyle(probe).backgroundColor;
      probe.remove();
      return value;
    }, `--${token}`);
    check(
      `bg-${token} is var(${variable})`,
      utility === expected && utility !== "rgba(0, 0, 0, 0)",
      `${utility} vs ${expected}`,
    );
  }

  // --- palettes and dark mode ----------------------------------------------
  await page.screenshot({ path: join(OUT, "default-dark.png") });
  await page.click("[data-theme-toggle]");
  await page.waitForTimeout(50);
  check(
    "toggle removes the dark class",
    !(await page.evaluate(() =>
      document.documentElement.classList.contains("dark"),
    )),
  );
  await page.screenshot({ path: join(OUT, "default-light.png") });

  for (const [id, value] of Object.entries(PALETTES)) {
    await page.click(`[data-palette="${id}"]`);
    await page.waitForTimeout(50);
    const expectedTheme = id === "default" ? null : id;
    const attr = await page.evaluate(() =>
      document.documentElement.getAttribute("data-theme"),
    );
    check(
      `data-theme=${expectedTheme ?? "<unset>"} after selecting ${id}`,
      attr === expectedTheme,
      `got ${attr}`,
    );
    const primary = await resolveVar(page, "--primary");
    check(
      `${id} light --primary`,
      primary === value.light,
      `${primary} vs ${value.light}`,
    );
    await page.screenshot({ path: join(OUT, `theme-${id}-light.png`) });

    await page.click("[data-theme-toggle]");
    await page.waitForTimeout(50);
    const primaryDark = await resolveVar(page, "--primary");
    check(
      `${id} dark --primary`,
      primaryDark === value.dark,
      `${primaryDark} vs ${value.dark}`,
    );
    const background = await resolveVar(page, "--background");
    check(`${id} dark --background is #151515`, background === "#151515", background);
    await page.screenshot({ path: join(OUT, `theme-${id}-dark.png`) });
    await page.click("[data-theme-toggle]");
  }

  // --- capability hooks and the reduced-motion CSS rule ---------------------
  const probe = (name) =>
    page.getAttribute(`[data-probe="${name}"]`, "data-probe-value");
  check("hover probe is true on a headless desktop", (await probe("hover")) === "true");
  check("touch probe is false without a coarse pointer", (await probe("touch")) === "false");
  check("reduced-motion probe starts false", (await probe("reduce")) === "false");
  const transition = await page.evaluate(
    () => getComputedStyle(document.querySelector(".press")).transitionDuration,
  );
  check("press transition is live", transition === "0.12s", transition);

  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.waitForTimeout(200);
  check(
    "reduced-motion probe follows the media query",
    (await probe("reduce")) === "true",
    String(await probe("reduce")),
  );
  const reducedTransition = await page.evaluate(
    () => getComputedStyle(document.querySelector(".press")).transitionDuration,
  );
  check(
    "the global CSS rule collapses transitions to 0.01ms",
    reducedTransition === "1e-05s" || reducedTransition === "0.01ms",
    reducedTransition,
  );

  // --- motion tokens are readable from Rust-land via the page --------------
  const easeOut = await page.evaluate(() => {
    const probe = document.createElement("div");
    probe.style.transitionTimingFunction = "var(--ease-out)";
    document.body.appendChild(probe);
    const value = getComputedStyle(probe).transitionTimingFunction;
    probe.remove();
    return value;
  });
  check(
    "--ease-out is the beUI curve",
    easeOut === "cubic-bezier(0.16, 1, 0.3, 1)",
    easeOut,
  );

  check("no console errors", consoleErrors.length === 0, consoleErrors.join(" | "));

  await browser.close();
  server.close();

  console.log(`\nscreenshots: ${OUT}`);
  if (failures.length) {
    console.log(`\n${failures.length} check(s) failed`);
    process.exit(1);
  }
  console.log("\nall checks passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

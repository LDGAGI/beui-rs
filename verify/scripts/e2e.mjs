// End-to-end smoke test: every gallery page boots, mounts and paints.
//
// The web build only proves the wasm compiles. It does not catch a page that
// panics on first render, an `eval` script that never sends, or a component
// whose markup is empty. This walks the nav, waits for each page's marker text,
// and fails on any console error or page error.
//
// Usage: node scripts/e2e.mjs [dist-dir]

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, resolve } from "node:path";

const DIST = resolve(process.argv[2] ?? "../target/dx/gallery/debug/web/public");
// `ONLY=<slug>` walks just that page and stops — a cheap probe while
// bisecting a page that hangs the renderer.
const ONLY = process.env.ONLY ?? "";

const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".json": "application/json",
  ".png": "image/png",
};

// `(slug, marker text that must appear once the page has mounted)`.
const PAGES = [
  ["text-shimmer", "Loading projects…"],
  ["marquee", "Vercel"],
  ["magnetic", "Hover me"],
  ["tilt-card", "Tilt me"],
  ["scroll-reveal", "Spring slide"],
  ["scroll-progress", "Section 1"],
  ["parallax", "Parallax"],
  ["animated-number", "Monthly recurring revenue"],
  ["text-reveal", "Replay"],
  ["text-scramble", "Inspecting the repository"],
  ["digit-swap", "Card number"],
  ["number-ticker", "Active users"],
  ["text-cascade", "Install skills"],
  ["action-swap", "Copy link"],
  ["theme-toggle", "Rectangle"],
  ["shared-layout-bg", "Inbox"],
  ["chromatic-text-reveal", "Motion that feels"],
  ["preview-rail", "PreviewRail"],
  ["aspect-ratio", "16:9"],
  ["alert", "A little heads up"],
  ["notification-stack", "Orders import failed"],
  ["otp-input", "Verification code"],
  ["dock", "Dock"],
  ["bouncy-accordion", "Release Brief"],
  ["bounce-sidebar", "Overview"],
  ["bloom-menu", "Create"],
  ["expanding-arrow-button", "Book a demo"],
  ["slide-action-button", "Slide to continue"],
  ["hold-action-button", "Hold for vertical fill"],
  ["expandable-control", "ExpandableControl"],
  ["overflow-actions", "OverflowActions"],
  ["smooth-scroll", "ScrollTo / SmoothScroll"],
  ["card-folder", "CardFolder"],
  ["dynamic-island", "DynamicIsland"],
  ["center-morph-modal", "Center morph modal"],
  ["file-tree", "FileTree"],
  ["expandable-action-bar", "ExpandableActionBar"],
  ["swipeable-list", "Launch brief"],
  ["project-folder", "Brand direction"],
  ["pull-to-refresh", "PullToRefresh"],
  ["animated-badge", "Syncing"],
  ["color-selector", "Accent"],
  ["bottom-sheet", "Open bottom sheet"],
  ["breadcrumb", "Workspace"],
  ["infinite-masonry", "Visual index"],
  ["not-found-stacked", "NotFound"],
  ["not-found-terminal", "NotFound"],
  ["not-found-glitch", "NotFound"],
  ["not-found-spotlight", "NotFound"],
  ["not-found-magnetic", "NotFound"],
  ["range-slider", "Drag the handle"],
  ["range-slider-wave", "Gain"],
  ["prediction-market-card", "Boston Celtics"],
  ["sortable-list", "In the studio"],
  ["sortable-stack", "In the studio"],
  ["liquid", "Overview"],
  ["cylinder-carousel", "CylinderCarousel"],
  ["feedback-widget", "FeedbackWidget"],
  ["signup-form", "Create your account"],
  ["morphing-search-chips", "MorphingSearchChips"],
  ["prediction-market", "PredictionMarket"],
  ["morphing-search", "MorphingSearch"],
  ["file-upload", "Drop files here"],
  ["knockout-bracket", "KnockoutBracket"],
  ["wallet-card", "WalletCard"],
  ["swap", "MultiChainSwap"],
  ["knockout-wheel", "KnockoutWheel"],
  ["attachment-upload", "Attachments:"],
  ["table", "10,000 rows"],
  ["table-editable", "Click a cell to edit."],
  ["table-async", "loaded"],
  ["availability-scheduler", "Unavailable"],
  ["morphing-lightbox", "Look a little closer."],
  ["arc-picker", "Arc Picker"],
];

// Navigate the gallery: the nav is a tall sticky scroll container, so a plain
// click's actionability checks (visible + stable) can loop on it.
const nav = (slug) =>
  page.$eval(`[data-page-link="${slug}"]`, (element) => element.click());

const failures = [];
function check(name, ok, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
  if (!ok) failures.push(name);
}

async function serve(dir) {
  const server = createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const relative = decodeURIComponent(url.pathname);
    // The app routes on the path (`/site-home`, `/component-alert`, …), so a
    // path that is not a file is a page: serve index.html the way a static host
    // does with an SPA rewrite. Requests that name an extension are real assets
    // and still 404, so a missing bundle is not masked.
    const candidates =
      relative === "/" || relative.endsWith("/")
        ? [join(dir, relative, "index.html")]
        : extname(relative) === ""
          ? [join(dir, relative), join(dir, "index.html")]
          : [join(dir, relative)];
    for (const path of candidates) {
      try {
        const body = await readFile(path);
        response.writeHead(200, {
          "content-type": MIME[extname(path)] ?? "application/octet-stream",
        });
        response.end(body);
        return;
      } catch {
        // try the next candidate
      }
    }
    response.writeHead(404).end("not found");
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  return { server, origin: `http://127.0.0.1:${server.address().port}` };
}

const { server, origin } = await serve(DIST);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

const consoleErrors = [];
page.on("console", (message) => {
  if (message.type() !== "error") return;
  const text = message.text();
  // `dx build` ships the devtools websocket; the static server has no
  // `/_dioxus` endpoint, so its handshake 404s. Not a component error.
  if (text.includes("/_dioxus")) return;
  consoleErrors.push(text);
});
page.on("pageerror", (error) => consoleErrors.push(String(error)));

await page.goto(origin, { waitUntil: "networkidle" });
await page.waitForSelector('[data-page="foundation"]');

for (const [slug, marker] of PAGES.filter(([slug]) => !ONLY || slug === ONLY)) {
  const before = consoleErrors.length;
  // A click through the actionability checks is unreliable for the tail of a
  // 50-item sticky nav; dispatch it on the element directly.
  await nav(slug);
  try {
    await page.waitForSelector(`[data-page="${slug}"]`, { timeout: 5000 });
    await page.waitForFunction(
      (text) => document.body.innerText.includes(text),
      marker,
      { timeout: 5000 },
    );
    check(`page ${slug}`, true);
  } catch (error) {
    check(`page ${slug}`, false, String(error).split("\n")[0]);
  }
  const fresh = consoleErrors.slice(before);
  if (fresh.length > 0) {
    check(`page ${slug} console`, false, fresh[0]);
  }
}

if (ONLY) {
  await nav("knockout-wheel");
await page.waitForSelector('[data-page="knockout-wheel"]');
const wheel = '[data-page="knockout-wheel"]';
const wheelState = () =>
  page.evaluate((selector) => {
    const root = document.querySelector(selector);
    const anchors = [...root.querySelectorAll("button[aria-label]")];
    const dimmed = [...root.querySelectorAll("svg g[style] > image, svg g[style] > text")]
      .filter((node) => Number(node.getAttribute("opacity")) < 0.5).length;
    return {
      discs: root.querySelectorAll("svg circle.fill-card").length,
      anchors: anchors.length,
      trophy: root.querySelectorAll("svg path.fill-amber-400").length,
      dimmed,
      focused: document.activeElement?.id ?? "",
    };
  }, wheel);

const wheelFirst = await wheelState();
check(
  "knockout-wheel draws the rings",
  wheelFirst.discs === 15 && wheelFirst.anchors === 15 && wheelFirst.trophy === 1,
  `${wheelFirst.discs} discs, ${wheelFirst.anchors} anchors, ${wheelFirst.trophy} trophy`,
);
// Hovering a node lights its path and dims the rest.
await page.$eval(`${wheel} button[aria-label]`, (element) =>
  element.dispatchEvent(new PointerEvent("pointerenter", { bubbles: true, pointerType: "mouse" })),
);
await page.waitForTimeout(400);
check(
  "knockout-wheel dims what is not on the lit path",
  (await wheelState()).dimmed > 0,
  `${(await wheelState()).dimmed} dimmed`,
);
// The arrow keys walk the tree: Down goes from the hub to a feeder. (Right
// would walk a ring, and the hub is a ring of one.)
await page.$eval(`${wheel} button[tabindex="0"]`, (element) => element.focus());
const before = (await wheelState()).focused;
await page.keyboard.press("ArrowDown");
await page.waitForTimeout(300);
const after = (await wheelState()).focused;
check(
  "knockout-wheel walks the tree with the arrow keys",
  before !== "" && after !== before,
  `${before} -> ${after}`,
);

await browser.close();
  server.close();
  console.log(
    failures.length > 0
      ? `\n${failures.length} failure(s)`
      : `\n${ONLY}: booted`,
  );
  process.exit(failures.length > 0 ? 1 : 0);
}

// The two eval-driven primitives are the novel part of this batch: prove the
// scroll listener and the IntersectionObserver actually stream back.
await nav("scroll-progress");
await page.waitForSelector('[data-page="scroll-progress"]');
await page.evaluate(() => {
  const box = document.querySelector("#scroll-progress-box");
  box.scrollTop = box.scrollHeight;
  box.dispatchEvent(new Event("scroll"));
});
await page.waitForTimeout(300);
const scaleX = await page.evaluate(() => {
  const bar = document.querySelector('[data-page="scroll-progress"] [aria-hidden="true"]');
  return bar ? getComputedStyle(bar).transform : "none";
});
check("scroll-progress reads the container", scaleX !== "none" && scaleX !== "matrix(1, 0, 0, 1, 0, 0)", scaleX);

await nav("scroll-reveal");
await page.waitForSelector('[data-page="scroll-reveal"]');
await page.evaluate(() => {
  const root = document.querySelector("#scroll-reveal-root");
  root.scrollTop = root.scrollHeight;
  root.dispatchEvent(new Event("scroll"));
});
await page.waitForTimeout(800);
const revealed = await page.evaluate(() => {
  const cards = document.querySelectorAll('[data-page="scroll-reveal"] [data-beui-inview]');
  const last = cards[cards.length - 1];
  return last ? Number(getComputedStyle(last).opacity) : 0;
});
check("scroll-reveal observes the root", revealed > 0.5, `opacity ${revealed}`);

await nav("animated-number");
await page.waitForSelector('[data-page="animated-number"]');
await page.waitForFunction(
  () => document.body.innerText.includes("$129,480"),
  undefined,
  { timeout: 4000 },
);
check("animated-number counts up", true);

await nav("text-scramble");
await page.waitForSelector('[data-page="text-scramble"]');
await page.click('[data-page="text-scramble"] button');
await page.waitForFunction(
  () => document.body.innerText.includes("Running the checks"),
  undefined,
  { timeout: 4000 },
);
check("text-scramble resolves the next phrase", true);

await nav("digit-swap");
await page.waitForSelector('[data-page="digit-swap"]');
await page.click('[data-page="digit-swap"] button');
await page.waitForFunction(
  () => document.body.innerText.includes("4242 4242 4242 0806"),
  undefined,
  { timeout: 4000 },
);
check("digit-swap rolls to the revealed number", true);

await nav("action-swap");
await page.waitForSelector('[data-page="action-swap"]');
await page.click('[data-page="action-swap"] button');
await page.waitForFunction(
  () => document.body.innerText.includes("Copied"),
  undefined,
  { timeout: 4000 },
);
check("action-swap cycles its items", true);

await nav("theme-toggle");
await page.waitForSelector('[data-page="theme-toggle"]');
const beforeTheme = await page.evaluate(() => document.documentElement.classList.contains("dark"));
await page.click('[data-page="theme-toggle"] button');
await page.waitForFunction(
  (before) => document.documentElement.classList.contains("dark") !== before,
  beforeTheme,
  { timeout: 4000 },
);
check("theme-toggle flips the dark class", true);

await nav("shared-layout-bg");
await page.waitForSelector('[data-page="shared-layout-bg"]');
await page.hover('[data-page="shared-layout-bg"] .relative >> nth=1');
await page.waitForTimeout(400);
const pill = await page.evaluate(() => {
  const node = document.querySelector('[data-page="shared-layout-bg"] [data-beui-flip]');
  return node ? getComputedStyle(node).borderRadius : "none";
});
check("shared-layout-bg shows the pill on hover", pill !== "none", pill);

await nav("chromatic-text-reveal");
await page.waitForSelector('[data-page="chromatic-text-reveal"]');
await page.waitForTimeout(400);
const sweep = await page.evaluate(() => {
  const node = document.querySelector('[data-page="chromatic-text-reveal"] [aria-hidden="true"][style*="chromatic-sweep"]');
  return node ? node.getAttribute("style") : "";
});
check("chromatic-text-reveal sweeps", sweep.includes("chromatic-sweep"), sweep.slice(0, 60));

await nav("preview-rail");
await page.waitForSelector('[data-page="preview-rail"]');
await page.hover('[data-page="preview-rail"] [data-slot="preview-rail-item"] >> nth=0');
await page.waitForFunction(
  () => document.body.innerText.includes("Dashboard"),
  undefined,
  { timeout: 4000 },
);
check("preview-rail shows the card on hover", true);

await nav("alert");
await page.waitForSelector('[data-page="alert"]');
await page.click('[data-slot="alert-close"]');
await page.waitForFunction(
  () => !document.querySelector('[data-slot="alert"]'),
  undefined,
  { timeout: 4000 },
);
check("alert unmounts after its exit", true);

await nav("notification-stack");
await page.waitForSelector('[data-page="notification-stack"]');
await page.hover('[data-page="notification-stack"] button');
await page.waitForFunction(
  () => document.querySelector('[data-page="notification-stack"] button')?.getAttribute("aria-expanded") === "true",
  undefined,
  { timeout: 4000 },
);
check("notification-stack expands on hover", true);

await nav("otp-input");
await page.waitForSelector('[data-page="otp-input"]');
await page.click('[data-page="otp-input"] fieldset');
await page.keyboard.type("123456");
await page.waitForFunction(
  () => document.body.innerText.includes("Verified."),
  undefined,
  { timeout: 4000 },
);
check("otp-input verifies a typed code", true);

await nav("dock");
await page.waitForSelector('[data-page="dock"]');
await page.click('[data-page="dock"] [aria-label="Mail"]');
await page.waitForFunction(
  () => document.querySelector('[data-page="dock"] [aria-label="Mail"]')?.getAttribute("aria-pressed") === "true",
  undefined,
  { timeout: 4000 },
);
check("dock hands the pill to the clicked item", true);

await nav("bouncy-accordion");
await page.waitForSelector('[data-page="bouncy-accordion"]');
await page.click('[data-page="bouncy-accordion"] button[aria-controls$="brief-content"]');
await page.waitForFunction(
  () => document.querySelector('[data-page="bouncy-accordion"] button[aria-controls$="brief-content"]')?.getAttribute("aria-expanded") === "true",
  undefined,
  { timeout: 4000 },
);
await page.waitForTimeout(500);
const accordionHeight = await page.evaluate(() => {
  const node = document.querySelector('[data-page="bouncy-accordion"] [id$="brief-content"]');
  return node ? parseFloat(getComputedStyle(node).height) : 0;
});
check("bouncy-accordion opens its row", accordionHeight > 0, `height ${accordionHeight}px`);

await nav("bounce-sidebar");
await page.waitForSelector('[data-page="bounce-sidebar"]');
await page.click('[data-page="bounce-sidebar"] button >> nth=4');
await page.waitForFunction(
  () => document.querySelector('[data-page="bounce-sidebar"] button[aria-current="page"]')?.textContent.includes("Changelog"),
  undefined,
  { timeout: 4000 },
);
await page.waitForTimeout(400);
const dotMoved = await page.evaluate(() => {
  const dot = document.querySelector('[data-page="bounce-sidebar"] li[role="presentation"] span');
  return dot ? getComputedStyle(dot).transform : "none";
});
check("bounce-sidebar moves the dot", dotMoved !== "none", dotMoved);

await nav("bloom-menu");
await page.waitForSelector('[data-page="bloom-menu"]');
await page.click('[data-page="bloom-menu"] button[aria-haspopup="menu"]');
await page.waitForFunction(
  () => document.querySelector('[data-page="bloom-menu"] [data-beui-flip]')?.getBoundingClientRect().width > 300,
  undefined,
  { timeout: 4000 },
);
check("bloom-menu blooms into the panel", true);

await nav("expanding-arrow-button");
await page.waitForSelector('[data-page="expanding-arrow-button"]');
const capsuleBefore = await page.evaluate(() => {
  const node = document.querySelector('[data-page="expanding-arrow-button"] button > span[aria-hidden="true"]');
  return node ? node.getBoundingClientRect().width : 0;
});
await page.hover('[data-page="expanding-arrow-button"] button');
await page.waitForTimeout(700);
const capsuleAfter = await page.evaluate(() => {
  const node = document.querySelector('[data-page="expanding-arrow-button"] button > span[aria-hidden="true"]');
  return node ? node.getBoundingClientRect().width : 0;
});
check("expanding-arrow-button opens its capsule", capsuleAfter > capsuleBefore + 40, `${capsuleBefore} -> ${capsuleAfter}`);

await nav("slide-action-button");
await page.waitForSelector('[data-page="slide-action-button"]');
const thumbBox = await page.locator('[data-page="slide-action-button"] button').boundingBox();
await page.mouse.move(thumbBox.x + thumbBox.width / 2, thumbBox.y + thumbBox.height / 2);
await page.mouse.down();
await page.mouse.move(
  thumbBox.x + thumbBox.width / 2 + 250,
  thumbBox.y + thumbBox.height / 2,
  { steps: 12 },
);
await page.mouse.up();
await page.waitForFunction(
  () => document.body.innerText.includes("Action completed"),
  undefined,
  { timeout: 4000 },
);
check("slide-action-button completes on drag", true);
await page.waitForTimeout(2000);
await page.focus('[data-page="slide-action-button"] button');
await page.keyboard.press("Enter");
await page.waitForFunction(
  () => document.body.innerText.includes("Action completed"),
  undefined,
  { timeout: 4000 },
);
check("slide-action-button completes on Enter", true);

await nav("hold-action-button");
await page.waitForSelector('[data-page="hold-action-button"]');
const holdBox = await page.locator('[data-page="hold-action-button"] button').first().boundingBox();
await page.mouse.move(holdBox.x + holdBox.width / 2, holdBox.y + holdBox.height / 2);
await page.mouse.down();
await page.waitForTimeout(2000);
await page.mouse.up();
await page.waitForFunction(
  () => document.body.innerText.includes("Action confirmed"),
  undefined,
  { timeout: 4000 },
);
check("hold-action-button completes after the hold", true);

await nav("expandable-control");
await page.waitForSelector('[data-page="expandable-control"]');
await page.click('[data-page="expandable-control"] button[aria-expanded]');
await page.waitForFunction(
  () => document.querySelector('[data-page="expandable-control"] button[aria-expanded]')?.getAttribute("aria-expanded") === "true",
  undefined,
  { timeout: 4000 },
);
await page.waitForTimeout(500);
const expandWidth = await page.evaluate(() => {
  const node = document.querySelector('[data-page="expandable-control"] button[aria-expanded] > span:nth-child(2)');
  return node ? node.getBoundingClientRect().width : 0;
});
check("expandable-control expands its label", expandWidth > 40, `width ${expandWidth}`);

await nav("overflow-actions");
await page.waitForSelector('[data-page="overflow-actions"]');
await page.click('[data-page="overflow-actions"] button[aria-expanded]');
await page.waitForFunction(
  () => document.querySelector('[data-page="overflow-actions"] button[aria-expanded]')?.getAttribute("aria-expanded") === "true",
  undefined,
  { timeout: 4000 },
);
await page.waitForTimeout(500);
const overflowWidth = await page.evaluate(() => {
  const node = document.querySelector('[data-page="overflow-actions"] div[aria-hidden]');
  return node ? node.getBoundingClientRect().width : 0;
});
check("overflow-actions reveals the group", overflowWidth > 80, `width ${overflowWidth}`);

await nav("smooth-scroll");
await page.waitForSelector('[data-page="smooth-scroll"]');
await page.click('[data-page="smooth-scroll"] nav button >> nth=2');
await page.waitForTimeout(900);
const scrolled = await page.evaluate(() => {
  const node = document.querySelector('[data-page="smooth-scroll"] [data-beui-smooth-scroll]');
  return node ? node.scrollTop : 0;
});
check("scroll-to glides the contained box", scrolled > 50, `scrollTop ${scrolled}`);

await nav("card-folder");
await page.waitForSelector('[data-page="card-folder"]');
await page.click('[data-page="card-folder"] button[aria-pressed]');
await page.waitForFunction(
  () => document.body.innerText.includes("3745 6987 4096 0806"),
  undefined,
  { timeout: 4000 },
);
check("card-folder reveals the number", true);
await page.click('[data-page="card-folder"] button[aria-expanded]');
await page.waitForFunction(
  () => document.querySelector('[data-page="card-folder"] [data-open]')?.getAttribute("data-open") === "true",
  undefined,
  { timeout: 4000 },
);
check("card-folder opens", true);

await nav("dynamic-island");
await page.waitForSelector('[data-page="dynamic-island"]');
const islandBefore = await page.evaluate(() => {
  const node = document.querySelector('[data-page="dynamic-island"] [role="status"]');
  return node ? node.getBoundingClientRect().width : 0;
});
await page.click('[data-page="dynamic-island"] button >> nth=0');
await page.waitForFunction(
  () => document.body.innerText.toLowerCase().includes("incoming call"),
  undefined,
  { timeout: 4000 },
);
await page.waitForTimeout(1000);
const islandAfter = await page.evaluate(() => {
  const node = document.querySelector('[data-page="dynamic-island"] [role="status"]');
  return node ? node.getBoundingClientRect().width : 0;
});
check("dynamic-island expands to the view", islandAfter > islandBefore + 20, `${islandBefore} -> ${islandAfter}`);

await nav("center-morph-modal");
await page.waitForSelector('[data-page="center-morph-modal"]');
await page.click('[data-page="center-morph-modal"] button');
await page.waitForSelector('[role="dialog"]');
check("center-morph-modal opens", true);
await page.keyboard.press("Escape");
await page.waitForFunction(
  () => !document.querySelector('[role="dialog"]'),
  undefined,
  { timeout: 4000 },
);
check("center-morph-modal closes on Escape", true);

await nav("file-tree");
await page.waitForSelector('[data-page="file-tree"]');
await page.waitForFunction(
  () => document.body.innerText.includes("file-tree.tsx"),
  undefined,
  { timeout: 4000 },
);
const treeSelected = await page.evaluate(
  () => document.querySelector('[data-page="file-tree"] [role="treeitem"][aria-selected="true"]')?.getAttribute("data-beui-tree-row"),
);
check("file-tree selects the default row", treeSelected === "file-tree", String(treeSelected));
await page.focus('[data-page="file-tree"] [data-beui-tree-row="file-tree"]');
await page.keyboard.press("ArrowDown");
await page.waitForFunction(
  () => document.activeElement?.getAttribute("data-beui-tree-row") === "button",
  undefined,
  { timeout: 4000 },
);
check("file-tree moves focus with ArrowDown", true);

await nav("expandable-action-bar");
await page.waitForSelector('[data-page="expandable-action-bar"]');
const barBefore = await page.evaluate(() => {
  const node = document.querySelector('[data-page="expandable-action-bar"] div[class*="scrollbar-hide"]');
  return node ? node.getBoundingClientRect().width : 0;
});
await page.hover('[data-page="expandable-action-bar"] button[title="Send"]');
await page.waitForTimeout(800);
const barAfter = await page.evaluate(() => {
  const node = document.querySelector('[data-page="expandable-action-bar"] div[class*="scrollbar-hide"]');
  return node ? node.getBoundingClientRect().width : 0;
});
check("expandable-action-bar unfolds its labels", barAfter > barBefore + 40, `${barBefore} -> ${barAfter}`);

await nav("swipeable-list");
await page.waitForSelector('[data-page="swipeable-list"]');
const swipeRow = await page.locator('[data-page="swipeable-list"] [class*="cursor-grab"]').first().boundingBox();
await page.mouse.move(swipeRow.x + swipeRow.width / 2, swipeRow.y + swipeRow.height / 2);
await page.mouse.down();
await page.mouse.move(
  swipeRow.x + swipeRow.width / 2 + 120,
  swipeRow.y + swipeRow.height / 2,
  { steps: 12 },
);
await page.mouse.up();
await page.waitForTimeout(700);
const swipeTransform = await page.evaluate(() => {
  const node = document.querySelector('[data-page="swipeable-list"] [class*="cursor-grab"]');
  return node ? getComputedStyle(node).transform : "none";
});
check(
  "swipeable-list reveals the left rail",
  swipeTransform.startsWith("matrix(") && swipeTransform !== "matrix(1, 0, 0, 1, 0, 0)",
  swipeTransform,
);

await nav("project-folder");
await page.waitForSelector('[data-page="project-folder"]');
await page.click('[data-page="project-folder"] [aria-haspopup="dialog"]');
await page.waitForSelector('[role="dialog"]');
const folderCards = await page.evaluate(() => document.querySelectorAll('[role="dialog"] [class*="aspect-"]').length);
check("project-folder opens the overlay with its previews", folderCards === 5, String(folderCards));
await page.keyboard.press("Escape");
await page
  .waitForSelector('[data-page="project-folder"] [role="dialog"]', { state: "detached" })
  .then(() => check("project-folder closes on Escape", true, "detached"))
  .catch(async () => check("project-folder closes on Escape", false, await page.getAttribute('[data-page="project-folder"] [role="dialog"]', "class")));

await nav("pull-to-refresh");
await page.waitForSelector('[data-page="pull-to-refresh"]');
const feed = '[data-page="pull-to-refresh"] section[aria-label="Activity feed"]';
const feedBox = await page.locator(feed).boundingBox();
await page.mouse.move(feedBox.x + feedBox.width / 2, feedBox.y + 40);
await page.mouse.down();
for (let i = 1; i <= 10; i += 1) {
  await page.mouse.move(feedBox.x + feedBox.width / 2, feedBox.y + 40 + i * 14);
}
const pullState = await page.getAttribute(feed, "data-state");
check("pull-to-refresh reaches ready", pullState === "ready", pullState);
await page.mouse.up();
// The refresh is a 900ms timer inside a spawned task and the full walk leaves
// the page busy, so poll for the badge instead of guessing a wait.
const feedBadge = await page
  .waitForFunction(
    () =>
      document
        .querySelector('[data-page="pull-to-refresh"] header span')
        ?.textContent?.includes("1 refreshed"),
    undefined,
    { timeout: 40000 },
  )
  .then(() => "1 refreshed")
  .catch(async () => page.textContent('[data-page="pull-to-refresh"] header span'));
check("pull-to-refresh completes and counts", feedBadge.includes("1 refreshed"), feedBadge);

await nav("color-selector");
await page.waitForSelector('[data-page="color-selector"]');
const ringState = () =>
  page.evaluate(() => {
    const labels = [...document.querySelectorAll('[data-page="color-selector"] label')];
    const ring = document.querySelector('[data-page="color-selector"] [data-beui-flip]');
    return {
      index: labels.findIndex((label) => label.contains(ring)),
      transform: ring?.style.transform ?? "",
    };
  });
const ringBefore = await ringState();
await page.click('[data-page="color-selector"] label:nth-of-type(8)');
await page.waitForTimeout(90);
const ringMid = await ringState();
await page.waitForTimeout(700);
const ringAfter = await ringState();
check(
  "color-selector hops the ring",
  ringBefore.index === 0 &&
    ringMid.transform !== "none" &&
    ringAfter.index === 7 &&
    ringAfter.transform === "none",
  `${ringBefore.index} -> ${ringAfter.index} (mid: ${ringMid.transform})`,
);

await nav("animated-badge");
await page.waitForSelector('[data-page="animated-badge"]');
const badgeFirst = await page.textContent('[data-page="animated-badge"] [aria-live]');
// The roll keeps the leaving layer mounted for its 140ms exit, so the label
// slot holds two layers for a moment after every swap.
let badgeRolled = false;
for (let attempt = 0; attempt < 130 && !badgeRolled; attempt += 1) {
  badgeRolled = await page.evaluate(() => {
    const slot = document.querySelector(
      '[data-page="animated-badge"] [aria-live] > span:last-child',
    );
    return (slot?.children.length ?? 0) >= 2;
  });
  if (!badgeRolled) await page.waitForTimeout(16);
}
const badgeAfter = await page.textContent('[data-page="animated-badge"] [aria-live]');
check(
  "animated-badge rolls its label",
  badgeRolled && badgeFirst !== badgeAfter,
  `${badgeFirst} -> ${badgeAfter}`,
);
await nav("bottom-sheet");
await page.waitForSelector('[data-page="bottom-sheet"]');
await page.click('[data-page="bottom-sheet"] button:has-text("Open bottom sheet")');
await page.waitForSelector('[role="dialog"]');
await page.waitForTimeout(700);
const sheetBox = await page.locator('[role="dialog"]').boundingBox();
const sheetHeightRatio = sheetBox.height / (await page.evaluate(() => window.innerHeight));
check(
  "bottom-sheet opens at its first snap point",
  Math.abs(sheetHeightRatio - 0.4) < 0.02,
  sheetHeightRatio.toFixed(3),
);

// Fling the handle upwards: 0.4 -> 0.85.
const handleAt = async () =>
  page.locator('[role="dialog"] [class*="cursor-grab"]').first().boundingBox();
const handle = await handleAt();
await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
await page.mouse.down();
for (let i = 1; i <= 6; i += 1) {
  await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2 - i * 30);
}
await page.mouse.up();
await page.waitForTimeout(200);
await page.waitForTimeout(900);
const expandedRatio =
  (await page.locator('[role="dialog"]').boundingBox()).height /
  (await page.evaluate(() => window.innerHeight));
check("bottom-sheet flings to the next snap", expandedRatio > 0.8, expandedRatio.toFixed(3));

// A slow pull well past the dismiss threshold closes it.
const handle2 = await handleAt();
await page.mouse.move(handle2.x + handle2.width / 2, handle2.y + handle2.height / 2);
await page.mouse.down();
for (let i = 1; i <= 8; i += 1) {
  await page.mouse.move(handle2.x + handle2.width / 2, handle2.y + handle2.height / 2 + i * 40);
}
await page.mouse.up();
await page
  .waitForSelector('[role="dialog"]', { state: "detached", timeout: 3000 })
  .then(() => check("bottom-sheet dismisses on a long drag", true, "detached"))
  .catch(() => check("bottom-sheet dismisses on a long drag", false, "still open"));

// This smoke only checks the page mounts; the component's interaction is
// covered by its page's own assertions.
await nav("breadcrumb");
await page.waitForSelector('[data-page="breadcrumb"]');
const crumbs = await page.locator('[data-page="breadcrumb"] [aria-current="page"]').count();
check("breadcrumb page renders", crumbs >= 1, String(crumbs));

await nav("infinite-masonry");
await page.waitForSelector('[data-page="infinite-masonry"]');
const masonryCount = () =>
  page.evaluate(
    () => document.querySelectorAll('[data-page="infinite-masonry"] [data-beui-rect]').length,
  );
const masonryLanes = () =>
  page.evaluate(
    () =>
      new Set(
        [...document.querySelectorAll('[data-page="infinite-masonry"] [data-beui-rect]')].map(
          (node) => Math.round(node.getBoundingClientRect().x),
        ),
      ).size,
  );
const firstPage = await masonryCount();
const lanes = await masonryLanes();
check("infinite-masonry lays out the first page", firstPage === 12, String(firstPage));
check("infinite-masonry spreads items across columns", lanes >= 2, `${lanes} lanes`);
const masonrySnapped = await page.evaluate(() =>
  [...document.querySelectorAll('[data-page="infinite-masonry"] [data-beui-rect]')].every(
    (node) =>
      [node.style.left, node.style.top, node.style.width].every(
        (value) => value.length > 0 && !value.includes("."),
      ) && getComputedStyle(node).willChange === "auto",
  ),
);
check(
  "infinite-masonry snaps cells to whole pixels",
  masonrySnapped,
  "integer left/top/width, no permanent layer promotion",
);
await page.evaluate(() => {
  const scroller = document.querySelector('[data-page="infinite-masonry"] section');
  scroller.scrollTop = scroller.scrollHeight;
});
await page.waitForTimeout(1600);
const secondPage = await masonryCount();
check("infinite-masonry loads the next page", secondPage === 18, String(secondPage));

await nav("not-found-stacked");
await page.waitForSelector('[data-page="not-found-stacked"]');
const stackCard = () =>
  page.evaluate(
    () =>
      document.querySelector('[data-page="not-found-stacked"] div[aria-hidden="true"]')?.style
        .transform ?? "",
  );
const stackBefore = await stackCard();
await page.hover('[data-page="not-found-stacked"] [class*="h-44"]');
await page.waitForTimeout(700);
const stackAfter = await stackCard();
check(
  "not-found stacks fan out on hover",
  stackBefore !== stackAfter && stackAfter.includes("rotate"),
  `${stackBefore} -> ${stackAfter}`,
);

await nav("not-found-terminal");
await page.waitForSelector('[data-page="not-found-terminal"]');
await page.waitForFunction(
  () => document.body.innerText.includes("no such file or directory"),
  undefined,
  { timeout: 4000 },
);
await page.waitForFunction(
  () => document.body.innerText.includes("$ status 404"),
  undefined,
  { timeout: 4000 },
);
check("not-found terminal types its lines", true);

await nav("not-found-glitch");
await page.waitForSelector('[data-page="not-found-glitch"]');
// TextScramble keeps an `sr-only` copy for screen readers beside the animated
// one, so the settled state is the animated span, not the element's text.
await page.waitForFunction(
  () =>
    document.querySelector(
      '[data-page="not-found-glitch"] [data-not-found-code] span[aria-hidden="true"]',
    )?.textContent === "404",
  undefined,
  { timeout: 4000 },
);
// Park the pointer: `group-hover` would light the ghost up if the previous
// hover landed over this stage.
await page.mouse.move(4, 4);
await page.waitForTimeout(250);
const ghostHidden = await page.evaluate(
  () => getComputedStyle(document.querySelector('[data-page="not-found-glitch"] [aria-hidden="true"]')).opacity,
);
check("not-found glitch settles the scrambled code", ghostHidden === "0", ghostHidden);

await nav("not-found-spotlight");
await page.waitForSelector('[data-page="not-found-spotlight"]');
const spotlightMask = () =>
  page.evaluate(
    () =>
      document.querySelector('[data-page="not-found-spotlight"] [data-not-found-code]')?.style.getPropertyValue("mask-image") ??
      "",
  );
const spotlightBefore = await spotlightMask();
const stageBox = await page.locator('[data-page="not-found-spotlight"] [class*="aspect-"]').boundingBox();
await page.mouse.move(stageBox.x + 30, stageBox.y + 20);
await page.waitForTimeout(120);
const spotlightAfter = await spotlightMask();
check(
  "not-found spotlight follows the pointer",
  spotlightBefore !== spotlightAfter && spotlightAfter.includes("radial-gradient"),
  spotlightAfter.slice(0, 60),
);

await nav("not-found-magnetic");
await page.waitForSelector('[data-page="not-found-magnetic"]');
const glyph = '[data-page="not-found-magnetic"] [data-not-found-code] > div:first-child';
const glyphBefore = await page.evaluate(
  (selector) => getComputedStyle(document.querySelector(selector)).transform,
  glyph,
);
await page.hover(glyph);
await page.waitForTimeout(300);
const glyphAfter = await page.evaluate(
  (selector) => getComputedStyle(document.querySelector(selector)).transform,
  glyph,
);
check(
  "not-found magnetic leans its glyphs",
  glyphBefore !== glyphAfter,
  `${glyphBefore} -> ${glyphAfter}`,
);

await nav("range-slider");
await page.waitForSelector('[data-page="range-slider"]');
const sliderValue = () =>
  page.textContent('[data-page="range-slider"] .tabular-nums');
const sliderHandle = () =>
  page.evaluate(
    () =>
      document.querySelector('[data-page="range-slider"] [data-beui-slider-handle]')?.style
        .transform ?? "",
  );
const track = await page.locator('[data-page="range-slider"] [data-beui-slider]').first().boundingBox();
const handleBefore = await sliderHandle();
await page.mouse.move(track.x + track.width * 0.8, track.y + track.height / 2);
await page.mouse.down();
await page.mouse.move(track.x + track.width * 0.8, track.y + track.height / 2, { steps: 4 });
await page.mouse.up();
await page.waitForTimeout(500);
const draggedValue = await sliderValue();
const handleAfter = await sliderHandle();
check(
  "range-slider commits the pointer position",
  draggedValue === "80" && handleBefore !== handleAfter,
  `${handleBefore} -> ${handleAfter} at ${draggedValue}`,
);
await page.focus('[data-page="range-slider"] [data-beui-slider-handle]');
await page.keyboard.press("ArrowRight");
await page.waitForTimeout(200);
check(
  "range-slider steps from the keyboard",
  (await sliderValue()) === "85",
  await sliderValue(),
);
await page.keyboard.press("Home");
await page.waitForTimeout(200);
check("range-slider jumps to the minimum", (await sliderValue()) === "0", await sliderValue());

await nav("range-slider-wave");
await page.waitForSelector('[data-page="range-slider-wave"]');
const crestIndex = () =>
  page.evaluate(() => {
    const bars = [...document.querySelectorAll('[data-page="range-slider-wave"] [data-wave-bar]')];
    let best = -1;
    let bestScale = -1;
    bars.forEach((bar, index) => {
      const matrix = getComputedStyle(bar).transform.match(/matrix\(([^)]+)\)/);
      const scaleY = matrix ? Number(matrix[1].split(",")[3]) : 1;
      if (scaleY > bestScale) {
        bestScale = scaleY;
        best = index;
      }
    });
    return { best, bestScale, count: bars.length };
  });
const crestBefore = await crestIndex();
check(
  "wave-slider draws one bar per step",
  crestBefore.count === 32 && crestBefore.best >= 10 && crestBefore.best <= 15,
  `crest at ${crestBefore.best} of ${crestBefore.count}`,
);
const waveTrack = await page.locator('[data-page="range-slider-wave"] [data-beui-slider]').boundingBox();
await page.mouse.move(waveTrack.x + waveTrack.width * 0.8, waveTrack.y + waveTrack.height / 2);
await page.mouse.down();
await page.mouse.move(waveTrack.x + waveTrack.width * 0.8, waveTrack.y + waveTrack.height / 2);
await page.waitForTimeout(400);
const crestDuring = await crestIndex();
await page.mouse.up();
check(
  "wave-slider moves the crest with the handle",
  crestDuring.best > crestBefore.best + 5,
  `${crestBefore.best} -> ${crestDuring.best} (peak ${crestDuring.bestScale.toFixed(2)})`,
);

await nav("prediction-market-card");
await page.waitForSelector('[data-page="prediction-market-card"]');
const oddsBefore = await page.textContent('[data-beui="action-swap-text"] >> nth=0').catch(() => "");
const oddsButton = page.locator('[data-page="prediction-market-card"] button[aria-label^="Trade"]').first();
const oddsText = await oddsButton.textContent();
check("prediction-market-card shows the price", oddsText.includes("51%"), oddsText);
await oddsButton.hover();
await page.waitForTimeout(400);
const oddsHover = await oddsButton.textContent();
check("prediction-market-card rolls the odds to the side", oddsHover.includes("Yes"), oddsHover);
const bookmark = page.locator('[data-page="prediction-market-card"] button[aria-label^="Bookmark"]').first();
await bookmark.click();
await page.waitForTimeout(300);
check(
  "prediction-market-card bookmarks independently",
  (await bookmark.getAttribute("aria-pressed")) === "true",
  String(await bookmark.getAttribute("aria-pressed")),
);
void oddsBefore;
await oddsButton.click();
await page.waitForTimeout(200);
const picked = await page.textContent('[data-page="prediction-market-card"] .font-mono');
check("prediction-market-card reports the selection", picked.includes("boston yes"), picked);

await nav("sortable-list");
await page.waitForSelector('[data-page="sortable-list"]');
const sortableOrder = () =>
  page.getAttribute('[data-page="sortable-list"] [data-order]', "data-order");
const firstRow = () =>
  page.textContent('[data-page="sortable-list"] [data-sortable-row]:first-child');
const orderBefore = await sortableOrder();
check(
  "sortable-list renders the tasks in order",
  orderBefore === "research,design,build,ship" &&
    (await firstRow()).includes("Collect references"),
  orderBefore,
);
// Keyboard: focus the first handle and move it down one place.
await page.focus('[data-page="sortable-list"] [data-sortable-row]:first-child [data-sortable-id]');
await page.keyboard.press("ArrowDown");
await page.waitForTimeout(600);
const orderAfterKey = await sortableOrder();
check(
  "sortable-list moves a row with the keyboard",
  orderAfterKey === "design,research,build,ship",
  orderAfterKey,
);
await page.click('[data-page="sortable-list"] button:has-text("Undo reorder")');
await page.waitForTimeout(600);
check(
  "sortable-list undoes the move",
  (await sortableOrder()) === orderBefore,
  await sortableOrder(),
);
// Drag: pull the first row past the second one's midpoint.
const handleBox = await page
  .locator('[data-page="sortable-list"] [data-sortable-row]:first-child [data-sortable-id]')
  .boundingBox();
const rows = await page.locator('[data-page="sortable-list"] [data-sortable-row]').all();
const secondBox = await rows[1].boundingBox();
const startY = handleBox.y + handleBox.height / 2;
// Land inside the second row's box (its midpoint plus a nudge), not a whole
// row past it: the target index is picked by the box the pointer is inside.
const targetY = secondBox.y + secondBox.height * 0.6;
await page.mouse.move(handleBox.x + handleBox.width / 2, startY);
await page.mouse.down();
for (let i = 1; i <= 6; i += 1) {
  await page.mouse.move(handleBox.x + handleBox.width / 2, startY + ((targetY - startY) * i) / 6);
}
await page.mouse.up();
await page.waitForTimeout(700);
const orderAfterDrag = await sortableOrder();
check(
  "sortable-list reorders live while dragging",
  orderAfterDrag === "design,research,build,ship",
  orderAfterDrag,
);

await nav("liquid");
await page.waitForSelector('[data-page="liquid"]');
const blobBox = () =>
  page.evaluate(() => {
    const rects = [...document.querySelectorAll('[data-page="liquid"] g[filter] rect')];
    const item = document.querySelector('[data-page="liquid"] [class*="pointer-events-none"][class*="z-10"]');
    return {
      count: rects.length,
      x: rects[0] ? Number(rects[0].getAttribute("x")) : -1,
      width: rects[0] ? Number(rects[0].getAttribute("width")) : -1,
      itemTransform: item?.style.transform ?? "",
    };
  });
const blobBefore = await blobBox();
check(
  "liquid fuses one blob per item",
  blobBefore.count === 1 && blobBefore.x === 0 && blobBefore.width > 60,
  `${blobBefore.count} blob(s) at x=${blobBefore.x} w=${blobBefore.width}`,
);
await page.click('[data-page="liquid"] button:has-text("Settings")');
await page.waitForTimeout(120);
const blobDuring = await blobBox();
await page.waitForTimeout(500);
const blobAfter = await blobBox();
// The wrapper and the filtered blob read the same box, so they must agree
// once the morph settles.
const itemX = Number(
  (blobAfter.itemTransform.match(/translate\(([-\d.]+)px/) ?? [])[1] ?? NaN,
);
check(
  "liquid morphs the blob to the new segment",
  blobAfter.x > blobBefore.x + 100 && itemX === blobAfter.x,
  `${blobBefore.x} -> ${blobAfter.x} (mid ${blobDuring.x}, item ${itemX})`,
);

await nav("cylinder-carousel");
await page.waitForSelector('[data-page="cylinder-carousel"]');
const ball = (index) =>
  page.evaluate((wanted) => {
    const node = document.querySelector(
      `[data-page="cylinder-carousel"] [data-carousel-ball="${wanted}"]`,
    );
    const match = node?.style.transform.match(/translate\(([-\d.]+)px, ([-\d.]+)px\) scale\(([-\d.]+)\)/);
    return {
      x: match ? Number(match[1]) : NaN,
      y: match ? Number(match[2]) : NaN,
      scale: match ? Number(match[3]) : NaN,
    };
  }, index);
const centre = await ball(0);
const edge = await ball(4);
check(
  "cylinder-carousel projects the concave wall",
  centre.scale < 0.6 && edge.scale > 0.9 && centre.y > 0 && Math.abs(centre.x) < 1,
  `centre scale ${centre.scale} y ${centre.y}, edge scale ${edge.scale}`,
);
await page.focus('[data-page="cylinder-carousel"] [role="application"]');
await page.keyboard.press("ArrowRight");
await page.waitForTimeout(900);
const rolledCentre = await ball(0);
check(
  "cylinder-carousel rolls with the arrow keys",
  rolledCentre.x < -50 && Math.abs(rolledCentre.scale - centre.scale) > 0.05,
  `x ${rolledCentre.x} scale ${rolledCentre.scale}`,
);
const stage = await page.locator('[data-page="cylinder-carousel"] [role="application"]').boundingBox();
await page.mouse.move(stage.x + stage.width / 2, stage.y + stage.height / 2);
await page.mouse.down();
await page.mouse.move(stage.x + stage.width / 2 - 200, stage.y + stage.height / 2, { steps: 10 });
// Measure while the pointer is still down: releasing starts a velocity glide,
// so a sample after `mouse.up` depends on the synthetic drag's speed and can
// land anywhere. Held, the position is exactly what the drag moved.
await page.waitForTimeout(120);
const draggedCentre = await ball(0);
await page.mouse.up();
check(
  "cylinder-carousel rolls with a drag",
  Math.abs(draggedCentre.x - rolledCentre.x) > 40,
  `${rolledCentre.x} -> ${draggedCentre.x}`,
);

await nav("date-range-picker");
await page.waitForSelector('[data-page="date-range-picker"]');
const drpTrigger = (await page.textContent("[data-date-range-trigger]"))?.trim();
check(
  "date-range-picker summarizes the range",
  drpTrigger === "Jan 9 – Jan 15, 2026",
  drpTrigger,
);
await page.click("[data-date-range-trigger]");
await page.waitForSelector('[data-date="2026-01-14"]');
await page.waitForTimeout(500);
const drpPanelWidth = () =>
  page.evaluate(() => {
    const table = document.querySelector(
      '[data-page="date-range-picker"] table[role="grid"]',
    );
    return table?.closest("div.w-80")?.getBoundingClientRect().width ?? 0;
  });
const drpBefore = await drpPanelWidth();
check("date-range-picker keeps its panel width", drpBefore >= 300, `${drpBefore.toFixed(1)}px`);
await page.click('[data-date="2026-01-14"]');
await page.waitForTimeout(300);
const drpAfter = await drpPanelWidth();
const drpFooter = (await page.textContent('[data-page="date-range-picker"] [aria-live]'))?.trim();
check(
  "date-range-picker does not collapse after picking a start",
  drpAfter >= 300,
  `${drpBefore.toFixed(1)} -> ${drpAfter.toFixed(1)}px`,
);
check(
  "date-range-picker asks for the end date",
  drpFooter === "Choose an end date",
  drpFooter,
);
await page.keyboard.press("Escape");
await page.waitForTimeout(900);
check(
  "date-range-picker closes on Escape",
  (await page.locator('[data-page="date-range-picker"] [data-date]').count()) === 0,
);

await nav("feedback-widget");
await page.waitForSelector('[data-page="feedback-widget"]');
const widget = '[data-page="feedback-widget"]';
// The trigger's layer animates in; wait for it to settle, then dispatch the
// clicks directly (Playwright's stability check can keep retrying on a
// transform that is still tweening).
const tap = (selector) => page.$eval(selector, (element) => element.click());
const type = (selector, value) =>
  page.$eval(
    selector,
    (element, text) => {
      const setter = Object.getOwnPropertyDescriptor(
        HTMLTextAreaElement.prototype,
        "value",
      ).set;
      setter.call(element, text);
      element.dispatchEvent(new Event("input", { bubbles: true }));
    },
    value,
  );

await page.waitForTimeout(600);
await tap(`${widget} button[aria-haspopup="dialog"]`);
await page.waitForSelector(`${widget} textarea`);
await page.waitForTimeout(600);
const shellBox = await page.locator(`${widget} [class*="w-[min(86vw,320px)]"]`).boundingBox();
check(
  "feedback-widget morphs the trigger into the panel",
  shellBox.width > 280 && (await page.locator(`${widget} textarea`).count()) === 1,
  `${Math.round(shellBox.width)}px wide`,
);
await type(`${widget} textarea`, "The roll feels great");
await page.waitForTimeout(200);
await tap(`${widget} button:has-text("Submit")`);
await page.waitForTimeout(1500);
const failedText = await page.textContent(widget);
check(
  "feedback-widget reports a failed send",
  failedText.includes("Something went wrong"),
  failedText.includes("Something went wrong") ? "error view" : failedText.slice(0, 40),
);
await tap(`${widget} button:has-text("Try again")`);
await page.waitForTimeout(1500);
const sentText = await page.textContent(widget);
check(
  "feedback-widget celebrates the retry",
  sentText.includes("Thanks!"),
  sentText.includes("Thanks!") ? "sent view" : sentText.slice(0, 40),
);
await page.waitForTimeout(2200);
check(
  "feedback-widget closes after success",
  (await page.locator(`${widget} textarea`).count()) === 0,
  String(await page.locator(`${widget} textarea`).count()),
);

await nav("signup-form");
await page.waitForSelector('[data-page="signup-form"]');
const form = '[data-page="signup-form"] form';
const fieldValue = (index, value) =>
  page.$eval(
    `${form} input >> nth=${index}`,
    (element, text) => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
      setter.call(element, text);
      element.dispatchEvent(new Event("input", { bubbles: true }));
      element.dispatchEvent(new Event("blur", { bubbles: true }));
    },
    value,
  );

// Submitting empty reveals every error.
await page.$eval(`${form} button[type="submit"]`, (element) => element.click());
await page.waitForTimeout(300);
const emptyErrors = await page.textContent(form);
check(
  "signup-form validates an empty submit",
  emptyErrors.includes("Enter your name.") && emptyErrors.includes("Accept the terms"),
  emptyErrors.includes("Enter your name.") ? "all errors" : emptyErrors.slice(0, 40),
);

// A short password trips the minimum and shows the strength meter.
await fieldValue(0, "Ada Lovelace");
await fieldValue(1, "ada@example.com");
await fieldValue(2, "short");
await page.waitForTimeout(300);
const shortText = await page.textContent(form);
check(
  "signup-form measures the password",
  shortText.includes("Use at least 8 characters.") &&
    shortText.includes("Password strength: Too short"),
  shortText.includes("Password strength: Too short") ? "strength meter" : shortText.slice(0, 40),
);

// A strong password scores higher and clears the error.
await fieldValue(2, "Correct-Horse-Battery-99!");
await page.waitForTimeout(400);
const strongText = await page.textContent(form);
check(
  "signup-form clears a fixed field",
  !strongText.includes("Use at least 8 characters.") &&
    strongText.includes("Password strength: Strong"),
  strongText.includes("Password strength: Strong") ? "Strong" : strongText.slice(0, 40),
);

// Terms on + matching confirmation submits.
await fieldValue(3, "Correct-Horse-Battery-99!");
await page.$eval(`${form} button[role="checkbox"]`, (element) => element.click());
await page.waitForTimeout(120);
await page.$eval(`${form} button[type="submit"]`, (element) => element.click());
await page.waitForTimeout(1600);
const doneText = await page.textContent(form);
check(
  "signup-form submits when valid",
  doneText.includes("Account created"),
  doneText.includes("Account created") ? "success" : doneText.slice(0, 40),
);

await nav("morphing-search-chips");
await page.waitForSelector('[data-page="morphing-search-chips"]');
const search = '[data-page="morphing-search-chips"]';
const field = `${search} input[role="combobox"]`;
const typeTerm = (value) =>
  page.$eval(
    field,
    (element, text) => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
      setter.call(element, text);
      element.dispatchEvent(new Event("input", { bubbles: true }));
    },
    value,
  );
const press = (key) => page.$eval(field, (element, k) => {
  element.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
}, key);

// The default chip renders inside the bar.
check(
  "search-chips renders its default chip",
  (await page.locator(`${search} [aria-label="Remove filter onboarding"]`).count()) === 1,
  String(await page.locator(`${search} [aria-label^="Remove filter"]`).count()),
);

// Typing opens the panel and lists matching rows.
await page.$eval(field, (element) => element.focus());
await page.waitForTimeout(600);
await typeTerm("dark");
await page.waitForTimeout(1400);
const listbox = await page.locator(`${search} [role="listbox"]`).count();
const options = await page.locator(`${search} [role="option"]`).count();
const glass = await page.evaluate(() => {
  const node = [...document.querySelectorAll('[data-page="morphing-search-chips"] div[aria-hidden="true"]')]
    .find((candidate) => candidate.style.width && candidate.style.height);
  return node ? { w: Math.round(parseFloat(node.style.width)), h: Math.round(parseFloat(node.style.height)) } : null;
});
check(
  "search-chips morphs the panel open",
  listbox === 1 && options >= 2 && glass !== null && glass.h > 40,
  `${options} options, glass ${glass ? `${glass.w}x${glass.h}` : "none"}`,
);

// Enter turns the highlighted row into a chip.
await press("Enter");
await page.waitForTimeout(500);
check(
  "search-chips adds a chip on Enter",
  (await page.locator(`${search} [aria-label="Remove filter dark"]`).count()) === 1 &&
    (await page.$eval(field, (element) => element.value)) === "",
  String(await page.locator(`${search} [aria-label^="Remove filter"]`).count()),
);

// Backspace marks the last chip, and again removes it.
await press("Backspace");
await page.waitForTimeout(200);
const marked = await page.evaluate(
  (selector) =>
    [...document.querySelectorAll(`${selector} span`)].some((node) =>
      String(node.className).includes("ring-inset ring-foreground/40"),
    ),
  search,
);
await press("Backspace");
await page.waitForTimeout(400);
check(
  "search-chips marks then removes the last chip",
  marked && (await page.locator(`${search} [aria-label="Remove filter dark"]`).count()) === 0,
  `marked ${marked}`,
);

await nav("prediction-market");
await page.waitForSelector('[data-page="prediction-market"]');
const ticket = '[data-page="prediction-market"]';
const ticketText = () => page.textContent(ticket);
const amountField = `${ticket} input[inputmode="decimal"]`;
const setAmount = (value) =>
  page.$eval(
    amountField,
    (element, text) => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
      setter.call(element, text);
      element.dispatchEvent(new Event("input", { bubbles: true }));
    },
    value,
  );

// The default order: Buy / Yes / 115 → 115 / 0.167 shares, priced as USD.
const initial = await ticketText();
check(
  "prediction-market quotes the default order",
  initial.includes("To win") && initial.includes("Avg. Price 16.7¢"),
  initial.includes("Avg. Price 16.7¢") ? "16.7¢" : initial.slice(0, 60),
);

// A quick amount adds to the value.
await page.$eval(`${ticket} button:has-text("+$100")`, (element) => element.click());
await page.waitForTimeout(300);
check(
  "prediction-market adds a quick amount",
  (await page.$eval(amountField, (element) => element.value)) === "215",
  await page.$eval(amountField, (element) => element.value),
);

// Sell switches the unit to shares and caps at the position (125).
await page.$eval(`${ticket} button:has-text("Sell")`, (element) => element.click());
await page.waitForTimeout(300);
const selling = await ticketText();
check(
  "prediction-market switches to shares",
  selling.includes("To receive") && selling.includes("Shares"),
  selling.includes("Shares") ? "Shares" : selling.slice(0, 60),
);
await setAmount("999");
await page.waitForTimeout(300);
check(
  "prediction-market rejects more shares than held",
  (await ticketText()).includes("Not enough shares"),
  ((await ticketText()).match(/Not enough shares/) ?? ["-"])[0],
);

// A valid order fills after 650ms.
await setAmount("40");
await page.waitForTimeout(300);
await page.$eval(`${ticket} button[data-trade-state]`, (element) => element.click());
await page.waitForTimeout(1200);
const filled = await page.$eval(`${ticket} button[data-trade-state]`, (element) => element.textContent);
check("prediction-market fills the order", filled.includes("Trade filled"), filled);

await nav("morphing-search");
await page.waitForSelector('[data-page="morphing-search"]');
const morph = '[data-page="morphing-search"]';
const firstTrigger = `${morph} button[aria-haspopup="dialog"]`;
const tapMorph = (selector) => page.$eval(selector, (element) => element.click());

await tapMorph(firstTrigger);
await page.waitForSelector(`${morph} [role="dialog"]`);
await page.waitForTimeout(700);
const panel = await page.evaluate((selector) => {
  const dialog = document.querySelector(`${selector} [role="dialog"]`);
  const box = dialog?.getBoundingClientRect();
  const shell = [...document.querySelectorAll(`${selector} div[aria-hidden="true"]`)]
    .map((node) => node.getBoundingClientRect())
    .find((rect) => rect.width > 200);
  return {
    width: box ? Math.round(box.width) : 0,
    clip: dialog ? getComputedStyle(dialog).clipPath : "",
    shellWidth: shell ? Math.round(shell.width) : 0,
    options: document.querySelectorAll(`${selector} [role="option"]`).length,
  };
}, morph);
check(
  "morphing-search unfolds the panel",
  panel.width > 200 && panel.shellWidth > 200 && panel.options === 5,
  `${panel.width}px dialog, ${panel.shellWidth}px shell, ${panel.options} options`,
);

// Typing filters to the matching rows.
await page.$eval(
  `${morph} input[role="combobox"]`,
  (element, text) => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
    setter.call(element, text);
    element.dispatchEvent(new Event("input", { bubbles: true }));
  },
  "tokens",
);
await page.waitForTimeout(300);
const filtered = await page.locator(`${morph} [role="option"]`).count();
check("morphing-search filters the rows", filtered === 1, String(filtered));

// Enter chooses the highlighted row.
await page.$eval(`${morph} input[role="combobox"]`, (element) => {
  element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
});
await page.waitForTimeout(300);
check(
  "morphing-search selects with Enter",
  (await page.textContent(morph)).includes("design-tokens"),
  ((await page.textContent(morph)).match(/Picked: (\S+)/) ?? ["-", "-"])[1],
);

// The panel closed on selection, and the `f` shortcut reopens it.
await page.waitForTimeout(400);
const afterSelect = await page.locator(`${morph} [role="dialog"]`).count();
await page.keyboard.press("f");
await page.waitForTimeout(700);
check(
  "morphing-search closes on select and reopens on the shortcut",
  afterSelect === 0 && (await page.locator(`${morph} [role="dialog"]`).count()) === 1,
  `after select ${afterSelect}, after shortcut ${await page.locator(`${morph} [role="dialog"]`).count()}`,
);

await nav("file-upload");
await page.waitForSelector('[data-page="file-upload"]');
const upload = '[data-page="file-upload"]';
const queueRows = () => page.locator(`${upload} ul li`).count();

check(
  "file-upload lists the initial queue",
  (await queueRows()) === 3 && (await page.textContent(upload)).includes("ZIP · 18 MB"),
  `${await queueRows()} rows`,
);
const uploadingProgress = await page.evaluate((selector) => {
  const bar = document.querySelector(`${selector} [role="progressbar"]`);
  const fill = bar?.firstElementChild;
  const matrix = fill ? getComputedStyle(fill).transform.match(/matrix\(([^)]+)\)/) : null;
  return {
    now: Number(bar?.getAttribute("aria-valuenow") ?? -1),
    scaleX: matrix ? Number(matrix[1].split(",")[0]) : -1,
  };
}, upload);
// The bar is drawn with a transform, so the fill's scaleX tracks the value.
check(
  "file-upload draws the progress as scaleX",
  uploadingProgress.now > 0 && Math.abs(uploadingProgress.scaleX - uploadingProgress.now / 100) < 0.05,
  `aria-valuenow ${uploadingProgress.now}, scaleX ${uploadingProgress.scaleX.toFixed(2)}`,
);
check(
  "file-upload reports the failure",
  (await page.textContent(upload)).includes("Connection lost") &&
    (await page.locator(`${upload} button[aria-label="Retry vendor-contract.pdf"]`).count()) === 1,
  "error row",
);
await page.$eval(`${upload} button[aria-label="Retry vendor-contract.pdf"]`, (element) => element.click());
await page.waitForTimeout(400);
check(
  "file-upload retries a failed row",
  (await page.locator(`${upload} button[aria-label="Remove vendor-contract.pdf"]`).count()) === 1 &&
    !(await page.textContent(upload)).includes("Connection lost"),
  "retried",
);
await page.$eval(`${upload} button[aria-label="Remove brand-assets.zip"]`, (element) => element.click());
await page.waitForTimeout(400);
check("file-upload removes a row", (await queueRows()) === 2, `${await queueRows()} rows`);
// The variant toggle switches the dropzone's shape.
await page.$eval(`${upload} button:has-text("Row")`, (element) => element.click());
await page.waitForTimeout(300);
const dropzoneClass = await page.getAttribute(`${upload} [data-dragging]`, "class");
check(
  "file-upload switches variant",
  dropzoneClass !== null && !dropzoneClass.includes("min-h-56"),
  dropzoneClass?.includes("min-h-56") ? "still centered" : "row",
);

await nav("knockout-bracket");
await page.waitForSelector('[data-page="knockout-bracket"]');
const bracket = '[data-page="knockout-bracket"]';
const bracketState = () =>
  page.evaluate((selector) => {
    const cards = [...document.querySelectorAll(`${selector} li[aria-label]`)];
    const headers = [...document.querySelectorAll(`${selector} section > div > div > div`)].map(
      (node) => ({ text: node.textContent, opacity: getComputedStyle(node).opacity }),
    );
    const connectors = document.querySelectorAll(`${selector} .rounded-r-xl`).length;
    const status = document.querySelector(`${selector} .sr-only`)?.textContent ?? "";
    return {
      cards: cards.length,
      connectors,
      status,
      labels: cards.map((card) => card.getAttribute("aria-label")),
      visibleHeaders: headers.filter((header) => header.opacity === "1").map((h) => h.text),
    };
  }, bracket);

const first = await bracketState();
check(
  "knockout-bracket lays out the draw",
  first.cards === 8 &&
    first.connectors === 3 &&
    first.visibleHeaders.join(",").includes("Round of 8"),
  `${first.cards} cards, ${first.connectors} connectors, headers ${first.visibleHeaders.join("/")}`,
);
check(
  "knockout-bracket announces the visible rounds",
  first.status.includes("Round of 8") && first.status.includes("Final"),
  first.status,
);
check(
  "knockout-bracket labels each tie for screen readers",
  first.labels.some((label) => label.includes("Morocco 1 (3 on penalties)") || label.includes("won")) &&
    first.labels.some((label) => label.includes("versus")),
  first.labels[0] ?? "none",
);
// Paging moves the window to the last two rounds.
await page.$eval(`${bracket} button[aria-label="Next round"]`, (element) => element.click());
await page.waitForTimeout(900);
const second = await bracketState();
check(
  "knockout-bracket pages to the last two rounds",
  second.status.includes("Semi-finals") && second.status.includes("Final") &&
    !second.status.includes("Round of 8"),
  second.status,
);
await page.$eval(`${bracket} button[aria-label="Previous round"]`, (element) => element.click());
await page.waitForTimeout(900);
check(
  "knockout-bracket pages back",
  (await bracketState()).status.includes("Round of 8"),
  (await bracketState()).status,
);
// The third place play-off lives under the stage, with its own label.
check(
  "knockout-bracket renders the third place play-off",
  (await page.textContent(bracket)).includes("Third place play-off") &&
    (await page.locator(`${bracket} ul[aria-label="Third place play-off"]`).count()) === 1,
  "third place",
);

await nav("wallet-card");
await page.waitForSelector('[data-page="wallet-card"]');
const wallet = '[data-page="wallet-card"]';
const tapWallet = (selector) => page.$eval(selector, (element) => element.click());

check(
  "wallet-card renders the balance and actions",
  (await page.textContent(wallet)).includes("$12,480.32") &&
    (await page.textContent(wallet)).includes("Main Wallet") &&
    (await page.textContent(wallet)).includes("Deposit"),
  "balance + actions",
);
check(
  "wallet-card shows the change pill",
  (await page.textContent(wallet)).includes("+$124.50"),
  ((await page.textContent(wallet)).match(/[-+]\$\d[\d,.]*/) ?? ["-"])[0],
);
await tapWallet(`${wallet} button[aria-haspopup="listbox"]`);
await page.waitForSelector(`${wallet} [role="listbox"]`);
await page.waitForTimeout(800);
const walletOptions = await page.locator(`${wallet} [role="option"]`).count();
await page.evaluate((selector) => {
  const options = document.querySelectorAll(`${selector} [role="option"]`);
  if (options[1]) options[1].click();
}, wallet);
await page.waitForTimeout(600);
check(
  "wallet-card morphs open and switches account",
  walletOptions === 3 && (await page.textContent(wallet)).includes("Trading"),
  `${walletOptions} options`,
);
await page.$eval(`${wallet} button[aria-label="Hide balance"]`, (element) => element.click());
await page.waitForTimeout(500);
check(
  "wallet-card masks the balance",
  (await page.textContent(wallet)).includes("*******"),
  ((await page.textContent(wallet)).match(/\*+/g) ?? ["-"])[0],
);
await tapWallet(`${wallet} button[aria-label="Search"]`);
await page.waitForSelector(`${wallet} input[placeholder="Search"]`);
const recents = await page.locator(`${wallet} ul li`).count();
await page.$eval(
  `${wallet} input[placeholder="Search"]`,
  (element) => {
    element.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  },
);
await page.waitForTimeout(400);
check(
  "wallet-card opens the search bar with recents",
  recents === 4 && (await page.locator(`${wallet} input[placeholder="Search"]`).count()) === 0,
  `${recents} recents, closed on Escape`,
);
await page.$eval(`${wallet} button:has-text("Send")`, (element) => element.click());
await page.waitForTimeout(300);
check(
  "wallet-card fires an action",
  (await page.textContent(wallet)).includes("Action fired: send"),
  "send",
);

await nav("swap");
await page.waitForSelector('[data-page="swap"]');
const swap = '[data-page="swap"]';
const swapText = () => page.textContent(swap);
const tapSwap = (selector) => page.$eval(selector, (element) => element.click());

check(
  "swap renders both fields and the quote",
  (await swapText()).includes("You pay") &&
    (await swapText()).includes("You get") &&
    (await swapText()).includes("Network fee"),
  "fields + quote",
);
// The rate follows the USD prices (3142 / 148 = 21.2297…), but the quote row
// shows a spinner for 450ms after any change, so poll for the text.
await page
  .waitForFunction(
    () =>
      /1 ETH ≈ 21\.2/.test(
        document.querySelector('[data-page="swap"]')?.textContent ?? "",
      ),
    undefined,
    { timeout: 8000 },
  )
  .catch(() => {});
const rate = ((await swapText()).match(/1 ETH ≈ ([\d.,]+) SOL/) ?? [])[1] ?? "";
check("swap derives the rate from USD prices", rate.startsWith("21.22"), rate || "no rate");
// The flip exchanges the two tokens.
await tapSwap(`${swap} button[aria-label="Reverse direction"]`);
await page.waitForTimeout(600);
const flipped = await swapText();
check(
  "swap flips the direction",
  flipped.includes("Swap SOL → ETH") || flipped.includes("1 SOL ≈"),
  (flipped.match(/Swap \w+ → \w+/) ?? ["-"])[0],
);
// The picker opens as a sheet with chain filters and picks a token.
await tapSwap(`${swap} button:has-text("SOL")`);
await page.waitForSelector(`${swap} [role="dialog"]`);
await page.waitForTimeout(600);
const chains = await page.locator(`${swap} [role="dialog"] button[aria-pressed]`).count();
await page.evaluate((selector) => {
  const rows = [...document.querySelectorAll(`${selector} [role="dialog"] li button`)];
  const usdc = rows.find((row) => row.textContent.includes("USD Coin"));
  if (usdc) usdc.click();
}, swap);
await page.waitForTimeout(700);
check(
  "swap picks a token from the sheet",
  chains >= 4 && (await page.locator(`${swap} [role="dialog"]`).count()) === 0 &&
    (await swapText()).includes("USDC"),
  `${chains} chain filters`,
);
// The action label reacts to the amount: clear it and it asks for one.
await page.$eval(`${swap} input[inputmode="decimal"]`, (element) => {
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
  setter.call(element, "");
  element.dispatchEvent(new Event("input", { bubbles: true }));
});
await page.waitForTimeout(400);
check(
  "swap asks for an amount when empty",
  (await swapText()).includes("Enter an amount"),
  ((await swapText()).match(/Enter an amount/) ?? ["-"])[0],
);

// --- attachment-upload -------------------------------------------------------

// The page's controls sit inside a list that re-renders on every row state
// change, so clicks go through `$eval` like the nav does.
const clickIn = (selector) => page.$eval(selector, (element) => element.click());
const rowIn = (id) => `[data-page="attachment-upload"] [data-beui-attachment-row="${id}"]`;

await nav("attachment-upload");
await page.waitForSelector('[data-page="attachment-upload"]');
check(
  "attachment-upload lists the three demo rows",
  (await page.locator('[data-page="attachment-upload"] [data-beui-attachment-row]').count()) === 3,
  String(await page.locator('[data-page="attachment-upload"] [data-beui-attachment-row]').count()),
);
check(
  "attachment-upload marks the failed row retryable",
  (await page.locator(`${rowIn("brief")} [data-beui-attachment-action="retry"]`).count()) === 1,
);

// The retry path is the source's timers: 900ms to complete, then 1000ms back.
await clickIn(`${rowIn("brief")} [data-beui-attachment-action="retry"]`);
await page.waitForSelector(`${rowIn("brief")} [data-beui-attachment-state="complete"]`, { timeout: 5000 });
check("attachment-upload retry reaches complete", true);
await page.waitForSelector(`${rowIn("brief")} [data-beui-attachment-action="remove"]`, { timeout: 5000 });
check("attachment-upload retry settles back to idle", true);

await clickIn('[data-page="attachment-upload"] [data-beui-attachment-thumbnail="flowers"]');
// The preview root is a zero-box positioning wrapper, so it is attached but
// never "visible" to Playwright.
await page.waitForSelector('[data-beui-attachment-preview="flowers"]', {
  state: "attached",
  timeout: 5000,
});
await page.waitForSelector('[data-beui-attachment-preview-close]', { timeout: 5000 });
check("attachment-upload opens the image preview", true);
await page.keyboard.press("Escape");
await page.waitForTimeout(300);
check(
  "attachment-upload closes the preview on Escape",
  (await page.locator('[data-beui-attachment-preview="flowers"]').count()) === 0,
);

await clickIn(`${rowIn("voice-note")} [data-beui-attachment-action="remove"]`);
await page.waitForSelector(`${rowIn("voice-note")} [data-beui-attachment-state="removing"]`, { timeout: 3000 });
check("attachment-upload shows the removing spinner", true);
await page.waitForTimeout(800);
check(
  "attachment-upload drops the removed row",
  (await page.locator(rowIn("voice-note")).count()) === 0,
);

// --- table -------------------------------------------------------------------

const pageOf = (slug) => `[data-page="${slug}"]`;
// A menu item, clicked without the actionability walk (the menu is a fixed layer
// above a fixed backdrop).
const clickText = (scope, text) =>
  page.$$eval(
    `${scope} button`,
    (buttons, label) => {
      const target = buttons.find((button) => button.textContent.includes(label));
      if (target) target.click();
      return Boolean(target);
    },
    text,
  );

await nav("table");
await page.waitForSelector(pageOf("table"));
const renderedRows = () =>
  page.locator(`${pageOf("table")} tbody tr[data-selected]`).count();
const windowRows = await renderedRows();
check(
  "table renders only the virtual window of 10k rows",
  windowRows > 0 && windowRows < 60,
  `${windowRows} rows in the DOM`,
);

// The default sort is MRR descending, and the header says so.
const mrrSort = await page.$eval(
  `${pageOf("table")} thead th:nth-child(6)`,
  (element) => element.getAttribute("aria-sort"),
);
check("table marks the default sort", mrrSort === "descending", mrrSort ?? "none");
const mrrValues = await page.$$eval(
  `${pageOf("table")} tbody tr[data-selected] td:nth-child(6) span`,
  (nodes) => nodes.map((node) => Number(node.textContent.replace(/[^0-9]/g, ""))),
);
check(
  "table sorts rows by MRR descending",
  mrrValues.length > 1 &&
    mrrValues.every((value, index) => index === 0 || mrrValues[index - 1] >= value),
  mrrValues.slice(0, 3).join(" >= "),
);

// Clicking a header cycles asc → desc → off.
const roleSort = () =>
  page.$eval(
    `${pageOf("table")} thead th:nth-child(4)`,
    (element) => element.getAttribute("aria-sort"),
  );
await clickIn(`${pageOf("table")} thead th:nth-child(4) button:not([aria-label])`);
await page.waitForTimeout(250);
check("table toggles a column ascending", (await roleSort()) === "ascending", await roleSort());
await clickIn(`${pageOf("table")} thead th:nth-child(4) button:not([aria-label])`);
await page.waitForTimeout(250);
check("table toggles a column descending", (await roleSort()) === "descending", await roleSort());
await clickIn(`${pageOf("table")} thead th:nth-child(4) button:not([aria-label])`);
await page.waitForTimeout(250);
check("table clears the sort on the third click", (await roleSort()) === null, String(await roleSort()));

// Row selection: select-all then clear.
await clickIn(`${pageOf("table")} thead button[role="checkbox"]`);
await page.waitForTimeout(300);
check(
  "table selects all rows from the header",
  await page.evaluate(() => document.body.innerText.includes("10,000 selected")),
);
await clickIn(`${pageOf("table")} thead button[role="checkbox"]`);
await page.waitForTimeout(300);
check(
  "table clears the selection",
  !(await page.evaluate(() => document.body.innerText.includes("10,000 selected"))),
);

// Column resize: drag the header's right edge.
const nameWidth = () =>
  page.$eval(
    `${pageOf("table")} thead th:nth-child(2)`,
    (element) => element.getBoundingClientRect().width,
  );
const widthBefore = await nameWidth();
const resizeHandle = await page.$(
  `${pageOf("table")} thead th:nth-child(2) button[aria-label="Resize name column"]`,
);
const resizeBox = await resizeHandle.boundingBox();
await page.mouse.move(resizeBox.x + resizeBox.width / 2, resizeBox.y + resizeBox.height / 2);
await page.mouse.down();
await page.waitForTimeout(250);
await page.mouse.move(
  resizeBox.x + resizeBox.width / 2 + 80,
  resizeBox.y + resizeBox.height / 2,
  { steps: 6 },
);
await page.waitForTimeout(150);
await page.mouse.up();
const widthAfter = await nameWidth();
check(
  "table resizes a column by dragging its edge",
  widthAfter > widthBefore + 40,
  `${Math.round(widthBefore)} -> ${Math.round(widthAfter)}`,
);

// Column reorder: drag the name grip past the role column.
const headerText = (n) =>
  page.$eval(`${pageOf("table")} thead th:nth-child(${n})`, (element) =>
    element.textContent.trim(),
  );
const reorderHandle = await page.$(
  `${pageOf("table")} thead th:nth-child(2) button[aria-label="Reorder name column"]`,
);
const reorderBox = await reorderHandle.boundingBox();
const roleBox = await page.$eval(
  `${pageOf("table")} thead th:nth-child(4)`,
  (element) => {
    const box = element.getBoundingClientRect();
    return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
  },
);
await page.mouse.move(reorderBox.x + reorderBox.width / 2, reorderBox.y + reorderBox.height / 2);
await page.mouse.down();
await page.waitForTimeout(250);
await page.mouse.move(roleBox.x, roleBox.y, { steps: 8 });
await page.waitForTimeout(150);
await page.mouse.up();
await page.waitForTimeout(250);
check(
  "table reorders columns by dragging the grip",
  (await headerText(2)) === "Email",
  await headerText(2),
);

// --- table-editable ----------------------------------------------------------

await nav("table-editable");
await page.waitForSelector(pageOf("table-editable"));
const editableRows = () =>
  page.locator(`${pageOf("table-editable")} tbody tr[data-selected]`).count();
check(
  "table-editable lists the four demo rows",
  (await editableRows()) === 4,
  String(await editableRows()),
);

const nameCell = `${pageOf("table-editable")} input[aria-label="name for row 1"]`;
await page.fill(nameCell, "Zed Vale");
await page.waitForTimeout(200);
check(
  "table-editable keeps an inline edit",
  (await page.inputValue(nameCell)) === "Zed Vale",
  await page.inputValue(nameCell),
);

// The row handle appears on hover; its menu inserts a row.
await page.hover(`${pageOf("table-editable")} tbody tr[data-selected]`);
await page.waitForSelector(
  `${pageOf("table-editable")} button[aria-label="Row 1 options"]`,
  { timeout: 3000 },
);
check("table-editable shows the row handle on hover", true);
await clickIn(`${pageOf("table-editable")} button[aria-label="Row 1 options"]`);
await page.waitForSelector('div[role="menu"]', { timeout: 3000 });
check(
  "table-editable opens the row menu",
  await clickText('div[role="menu"]', "Insert before"),
);
await page.waitForTimeout(300);
check(
  "table-editable inserts a row from the row handle",
  (await editableRows()) === 5,
  String(await editableRows()),
);

// The column handle appears on hover; its menu deletes a column.
await page.hover(`${pageOf("table-editable")} thead th:nth-child(1)`);
await page.waitForSelector(
  `${pageOf("table-editable")} button[aria-label="name column options"]`,
  { timeout: 3000 },
);
check("table-editable shows the column handle on hover", true);
await clickIn(`${pageOf("table-editable")} button[aria-label="name column options"]`);
await page.waitForSelector('div[role="menu"]', { timeout: 3000 });
check(
  "table-editable opens the column menu",
  await clickText('div[role="menu"]', "Delete column"),
);
await page.waitForTimeout(300);
const editableHeaders = () =>
  page.locator(`${pageOf("table-editable")} thead th`).count();
check(
  "table-editable deletes a column from the column handle",
  (await editableHeaders()) === 3,
  `${await editableHeaders()} header cells`,
);

// --- table-async -------------------------------------------------------------

await nav("table-async");
await page.waitForSelector(pageOf("table-async"));
await page.waitForFunction(() => document.body.innerText.includes("20 loaded"), undefined, {
  timeout: 5000,
});
check("table-async loads the first page", true);
const asyncScroll = `${pageOf("table-async")} div.overflow-auto`;
await page.$eval(asyncScroll, (box) => {
  box.scrollTop = box.scrollHeight;
  box.dispatchEvent(new Event("scroll"));
});
await page.waitForFunction(() => document.body.innerText.includes("40 loaded"), undefined, {
  timeout: 5000,
});
check("table-async appends the next page near the bottom", true);
check(
  "table-async renders the loaded rows",
  (await page.locator(`${pageOf("table-async")} tbody tr[data-selected]`).count()) > 0,
);

// --- availability-scheduler --------------------------------------------------

const sched = (day) =>
  `${pageOf("availability-scheduler")} [data-beui-scheduler-row="${day}"]`;
// Both action groups exist (a `sm:hidden` mobile copy and the desktop one), so
// click whichever is actually laid out.
const clickVisible = (selector) =>
  page.$$eval(selector, (nodes) => {
    const target = nodes.find((node) => node.offsetParent !== null) ?? nodes[0];
    if (!target) return false;
    target.click();
    return true;
  });
const clickVisibleLast = (selector) =>
  page.$$eval(selector, (nodes) => {
    const visible = nodes.filter((node) => node.offsetParent !== null);
    const target = visible[visible.length - 1] ?? nodes[nodes.length - 1];
    if (!target) return false;
    target.click();
    return true;
  });
const switchOn = (day) =>
  page.$eval(`${sched(day)} button[role="switch"]`, (element) =>
    element.getAttribute("aria-checked"),
  );
const rowText = (day) => page.$eval(sched(day), (element) => element.innerText);
const selectTriggers = (day) =>
  page.locator(`${sched(day)} button[aria-haspopup="listbox"]`).count();
const openPanels = () =>
  page.locator('[data-beui-scheduler] [role="listbox"]:not([aria-hidden])').count();
// A removed range keeps its row through the exit spring, so wait for the count
// rather than sleeping a guessed number of milliseconds.
const waitForTriggers = (day, count) =>
  page.waitForFunction(
    ([selector, expected]) => document.querySelectorAll(selector).length === expected,
    [`${sched(day)} button[aria-haspopup="listbox"]`, count],
    { timeout: 6000 },
  );

await nav("availability-scheduler");
await page.waitForSelector(pageOf("availability-scheduler"));
check(
  "availability-scheduler renders the seven days",
  (await page.locator(`${pageOf("availability-scheduler")} [data-beui-scheduler-row]`).count()) === 7,
);
check(
  "availability-scheduler opens on the default week",
  (await switchOn("mon")) === "true" &&
    (await switchOn("sat")) === "false" &&
    (await rowText("sat")).includes("Unavailable"),
  `mon ${await switchOn("mon")}, sat ${await switchOn("sat")}`,
);
check(
  "availability-scheduler gives each workday one range",
  (await selectTriggers("mon")) === 2,
  `${await selectTriggers("mon")} triggers`,
);

// The switch flips the day between a fresh 9–5 range and "Unavailable".
await clickIn(`${sched("sat")} button[role="switch"]`);
await waitForTriggers("sat", 2);
// The "Unavailable" marker is a presence child too: it leaves after the range
// has already entered, so wait for it rather than reading the row once.
await page.waitForFunction(
  (selector) =>
    !(document.querySelector(selector)?.innerText ?? "").includes("Unavailable") &&
    (document.querySelector(selector)?.innerText ?? "").includes("9:00 AM"),
  sched("sat"),
  { timeout: 6000 },
);
check(
  "availability-scheduler turns a day on with a 9–5 range",
  (await switchOn("sat")) === "true" && (await rowText("sat")).includes("9:00 AM"),
  `sat switch ${await switchOn("sat")}`,
);
await clickIn(`${sched("fri")} button[role="switch"]`);
await page.waitForFunction(
  (selector) =>
    document.querySelector(selector)?.innerText.includes("Unavailable") ?? false,
  sched("fri"),
  { timeout: 6000 },
);
check(
  "availability-scheduler turns a day off to Unavailable",
  (await switchOn("fri")) === "false",
  `fri switch ${await switchOn("fri")}`,
);

// Add / remove a range. The new range starts an hour after the last one ends.
await clickVisible('[data-beui-scheduler] button[aria-label="Add time range to Monday"]');
await waitForTriggers("mon", 4);
check(
  "availability-scheduler adds an hour after the last range",
  (await rowText("mon")).includes("6:00 PM") && (await rowText("mon")).includes("7:00 PM"),
  `${await selectTriggers("mon")} triggers`,
);
await clickVisibleLast(`${sched("mon")} button[aria-label="Remove time range"]`);
await waitForTriggers("mon", 2);
check(
  "availability-scheduler removes a range",
  (await selectTriggers("mon")) === 2,
  `${await selectTriggers("mon")} triggers`,
);

// The time dropdown: one panel week-wide, and picking a time rewrites the field.
await clickIn(`${sched("mon")} button[aria-haspopup="listbox"]`);
await page.waitForFunction(
  () =>
    document.querySelectorAll('[data-beui-scheduler] [role="listbox"]:not([aria-hidden])')
      .length === 1,
  undefined,
  { timeout: 6000 },
);
check("availability-scheduler opens exactly one time panel", (await openPanels()) === 1);
const pickedTime = await page.$$eval(
  '[data-beui-scheduler] [role="listbox"]:not([aria-hidden]) button[role="option"]',
  (nodes) => {
    const target = nodes.find((node) => node.textContent.trim() === "8:00 AM");
    if (!target) return false;
    target.click();
    return true;
  },
);
check("availability-scheduler lists the start options", pickedTime);
await page.waitForFunction(
  () =>
    document.querySelector('[data-beui-scheduler-row="mon"] button[aria-haspopup="listbox"]')
      ?.innerText.includes("8:00 AM") ?? false,
  undefined,
  { timeout: 6000 },
);
check(
  "availability-scheduler writes the picked time back",
  (await openPanels()) === 0,
  await page.$eval(`${sched("mon")} button[aria-haspopup="listbox"]`, (element) => element.innerText),
);

// The copy menu: Monday's hours onto Tuesday.
check(
  "availability-scheduler opens the copy menu",
  await clickVisible(`${sched("mon")} [data-beui-copy-trigger]`),
);
await page.waitForSelector("[data-beui-copy-panel]", { timeout: 4000 });
check(
  "availability-scheduler lists the other six days",
  (await page.locator("[data-beui-copy-panel] label").count()) === 6,
  `${await page.locator("[data-beui-copy-panel] label").count()} days`,
);
const toggled = await page.$$eval("[data-beui-copy-panel] label", (nodes) => {
  const target = nodes.find((node) => node.textContent.trim() === "Tuesday");
  if (!target) return false;
  target.querySelector('button[role="checkbox"]').click();
  return true;
});
await page.waitForTimeout(250);
check("availability-scheduler picks a copy target", toggled);
check("availability-scheduler applies the copy", await clickVisible("[data-beui-copy-apply]"));
await page.waitForFunction(
  () =>
    document.querySelector('[data-beui-scheduler-row="tue"] button[aria-haspopup="listbox"]')
      ?.innerText.includes("8:00 AM") ?? false,
  undefined,
  { timeout: 6000 },
);
// The panel leaves through its exit spring, so wait for it to detach rather
// than reading the DOM the instant the copy lands.
await page
  .waitForSelector("[data-beui-copy-panel]", { state: "detached", timeout: 6000 })
  .catch(() => {});
check(
  "availability-scheduler copies the hours and closes",
  (await page.locator("[data-beui-copy-panel]").count()) === 0,
  await page.$eval(`${sched("tue")} button[aria-haspopup="listbox"]`, (element) => element.innerText),
);

// --- morphing-lightbox -------------------------------------------------------

const lightbox = pageOf("morphing-lightbox");
const counter = () =>
  page.$eval("[data-beui-lightbox-counter]", (element) => element.textContent.trim());
const viewerOpen = () =>
  page.evaluate(() => document.querySelector("[data-beui-lightbox-counter]") !== null);

await nav("morphing-lightbox");
await page.waitForSelector(lightbox);
check(
  "morphing-lightbox renders the three thumbnails",
  (await page.locator(`${lightbox} [data-beui-lightbox-thumbnail]`).count()) === 3,
  String(await page.locator(`${lightbox} [data-beui-lightbox-thumbnail]`).count()),
);
check("morphing-lightbox starts with the viewer closed", !(await viewerOpen()));
// Open the first photograph. The viewer layers portal out of the page subtree
// (`PortalHost`), so the dialog is read from the document, not the page.
await clickIn(`${lightbox} [data-beui-lightbox-thumbnail="architecture"]`);
await page.waitForSelector("[data-beui-lightbox-counter]", { timeout: 5000 });
check(
  "morphing-lightbox opens the viewer on the first image",
  (await counter()) === "1 / 3",
  await counter(),
);
check(
  "morphing-lightbox captions the open image",
  (await page.textContent("[data-beui-lightbox-caption]")).includes("Form and light"),
  (await page.textContent("[data-beui-lightbox-caption]")).trim(),
);
check(
  "morphing-lightbox disables previous on the first image",
  await page.$eval('[aria-label="Previous image"]', (element) => element.disabled),
);
// Next pages forward; the counter is an `aria-live` readout.
await clickIn('[aria-label="Next image"]');
await page.waitForFunction(
  () => document.querySelector("[data-beui-lightbox-counter]")?.textContent.trim() === "2 / 3",
  undefined,
  { timeout: 5000 },
);
check("morphing-lightbox advances with the next control", true);
// The arrow keys page too (focus lands on a control inside the dialog).
await page.$eval('[role="dialog"]', (element) => element.focus());
await page.keyboard.press("ArrowRight");
await page.waitForFunction(
  () => document.querySelector("[data-beui-lightbox-counter]")?.textContent.trim() === "3 / 3",
  undefined,
  { timeout: 5000 },
);
check("morphing-lightbox pages with the arrow keys", true);
// Escape closes it, and the exit unmounts the layer.
await page.keyboard.press("Escape");
await page.waitForFunction(
  () => document.querySelector("[data-beui-lightbox-counter]") === null,
  undefined,
  { timeout: 5000 },
);
check("morphing-lightbox closes on Escape", true);

// --- arc-picker --------------------------------------------------------------

const arc = '[data-page="arc-picker"] [data-beui-arc][aria-label="Time of day"]';
const arcValue = () =>
  page.$eval('[data-page="arc-picker"] [data-beui-arc-value="controlled"]', (element) =>
    element.textContent.trim(),
  );
const arcChecked = () =>
  page.$eval(`${arc} [aria-checked="true"]`, (element) => element.getAttribute("data-value"));

await nav("arc-picker");
await page.waitForSelector(pageOf("arc-picker"));
await page.waitForTimeout(700);
check(
  "arc-picker renders every option",
  (await page.locator(`${arc} [role="radio"]`).count()) === 22,
  String(await page.locator(`${arc} [role="radio"]`).count()),
);
// The controlled value must survive mount without drifting to an end.
check(
  "arc-picker keeps the controlled value on mount",
  (await arcValue()) === "golden-hour" && (await arcChecked()) === "golden-hour",
  `${await arcValue()} / ${await arcChecked()}`,
);
// Keyboard: End / Home / arrows publish immediately.
await page.$eval(`${arc} [aria-checked="true"]`, (element) => element.focus());
await page.keyboard.press("End");
await page.waitForTimeout(400);
check("arc-picker jumps to the end with End", (await arcValue()) === "starlight", await arcValue());
await page.keyboard.press("Home");
await page.waitForTimeout(400);
check(
  "arc-picker jumps to the start with Home",
  (await arcValue()) === "first-light",
  await arcValue(),
);
await page.keyboard.press("ArrowDown");
await page.waitForTimeout(400);
check("arc-picker steps with the arrow keys", (await arcValue()) === "dawn", await arcValue());

// A click on a far label selects it.
await clickIn(`${arc} [data-value="midnight"]`);
await page.waitForTimeout(700);
check("arc-picker selects a clicked label", (await arcValue()) === "midnight", await arcValue());

// A downward drag rolls the arc back and publishes the detents it crosses.
const arcBox = await page.locator(arc).boundingBox();
const arcX = arcBox.x + arcBox.width / 2;
const arcY = arcBox.y + arcBox.height / 2;
const arcBefore = await arcValue();
await page.mouse.move(arcX, arcY);
await page.mouse.down();
for (let i = 1; i <= 10; i += 1) await page.mouse.move(arcX, arcY + i * 20);
await page.mouse.up();
await page.waitForTimeout(800);
const arcAfter = await arcValue();
check(
  "arc-picker publishes while dragging",
  arcAfter !== arcBefore && (await arcChecked()) === arcAfter,
  `${arcBefore} -> ${arcAfter}`,
);

// The uncontrolled instance owns its own state.
const uncontrolledArc =
  '[data-page="arc-picker"] [data-beui-arc][aria-label="Uncontrolled time of day"]';
await page.$eval(`${uncontrolledArc} [aria-checked="true"]`, (element) => element.focus());
await page.keyboard.press("End");
await page.waitForTimeout(500);
check(
  "arc-picker drives an uncontrolled instance",
  (await page.$eval('[data-page="arc-picker"] [data-beui-arc-value="uncontrolled"]', (element) =>
    element.textContent.trim(),
  )) === "starlight",
);

// --- switch ------------------------------------------------------------------
//
// The thumb's resting position must come from one place. The port animates the
// 20px travel instead of using motion's `layout`, so the track must not also
// align the thumb with `justify-end`: that placed it twice and pushed it 20px
// past the track, on top of the label (only the checked rows, because the
// unchecked translate is zero).

await nav("switch");
await page.waitForSelector(pageOf("switch"));
const switchRows = await page.evaluate(() => {
  const round = (value) => Math.round(value * 10) / 10;
  return [...document.querySelectorAll('[data-page="switch"] [role="switch"]')].map((track) => {
    const thumb = track.querySelector("div");
    const label = track.parentElement.querySelector("label");
    const trackBox = track.getBoundingClientRect();
    const thumbBox = thumb.getBoundingClientRect();
    return {
      state: track.dataset.state,
      inside: thumbBox.left >= trackBox.left - 0.5 && thumbBox.right <= trackBox.right + 0.5,
      gap: label ? round(label.getBoundingClientRect().left - thumbBox.right) : null,
    };
  });
});
check(
  "the switch thumb stays inside its track",
  switchRows.length === 3 && switchRows.every((row) => row.inside),
  switchRows.map((row) => `${row.state}:${row.inside ? "in" : "OUT"}`).join(" "),
);
check(
  "the switch thumb never reaches its label",
  switchRows.every((row) => row.gap === null || row.gap >= 8),
  switchRows.map((row) => `${row.state}:${row.gap}px`).join(" "),
);

// --- the left navigation and URL routing -------------------------------------
//
// The sidebar is the only navigation there is, and the slug it links to is the
// URL path segment, so every page is addressable and survives a refresh.
// `data-page` still carries the slug, which is what the rest of this walk (and
// `ONLY=<slug>`) keys on.

const slugOf = (url) => new URL(url).pathname.replace(/^\//, "");
const navLinks = () => page.locator("[data-nav-sidebar] [data-page-link]").count();

await nav("foundation");
await page.waitForSelector(pageOf("foundation"));

check(
  "the sidebar is the only navigation",
  (await page.locator("[data-nav-sidebar]").count()) === 1 &&
    (await page.locator("[data-site-header]").count()) === 0 &&
    (await navLinks()) > 100,
  `${await navLinks()} entries, site header gone`,
);
// Two of the source registry's four categories — `charts` and `agents` — have
// no gallery page at all, so a nav derived from the page list correctly shows
// neither. Assert the shape, not a fixed group count: every group present must
// have entries.
const groupCounts = await page.evaluate(() =>
  [...document.querySelectorAll("[data-nav-sidebar] [data-nav-group]")].map((node) =>
    Number(node.querySelector(":scope > div > span:last-child")?.textContent ?? "0"),
  ),
);
check(
  "the sidebar groups its entries and lists no empty group",
  groupCounts.length >= 3 && groupCounts.every((count) => count > 0),
  `${groupCounts.length} groups: ${groupCounts.join(", ")}`,
);
check(
  "the sidebar marks the current page",
  (await page.locator('[data-nav-sidebar] [aria-current="page"]').getAttribute("data-page-link")) ===
    "foundation",
);
// The nav is derived from the page list, so it must cover at least everything
// this walk boots.
check(
  "the sidebar exposes at least every page the walk tests",
  (await navLinks()) >= PAGES.length,
  `${await navLinks()} links, ${PAGES.length} walked`,
);

// The filter narrows the list; clearing it brings everything back.
const allLinks = await navLinks();
await page.fill("[data-nav-sidebar] [data-nav-filter]", "arc");
await page.waitForFunction(
  (count) => document.querySelectorAll("[data-nav-sidebar] [data-page-link]").length < count,
  allLinks,
  { timeout: 5000 },
);
const filteredLinks = await navLinks();
check(
  "the filter narrows the sidebar",
  filteredLinks > 0 && filteredLinks < allLinks,
  `${filteredLinks} of ${allLinks}`,
);

// …and a filtered entry opens a real page. `hooks` is deliberately one no other
// check in this file visits: it proves a nav entry resolves to a page rather
// than to `render`'s `Unknown page` notice.
await page.fill("[data-nav-sidebar] [data-nav-filter]", "hooks");
await page.waitForFunction(
  () => document.querySelectorAll("[data-nav-sidebar] [data-page-link]").length === 1,
  undefined,
  { timeout: 5000 },
);
await nav("hooks");
await page.waitForSelector(pageOf("hooks"), { timeout: 10000 });
const hooksText = await page.locator(pageOf("hooks")).innerText();
check(
  "a filtered nav entry opens a real page",
  hooksText.length > 40 && !hooksText.includes("Unknown page"),
  `${hooksText.length} chars`,
);

await page.fill("[data-nav-sidebar] [data-nav-filter]", "");
await page.waitForFunction(
  (count) => document.querySelectorAll("[data-nav-sidebar] [data-page-link]").length === count,
  allLinks,
  { timeout: 5000 },
);
check("clearing the filter restores the list", true);

// A click routes without a document load, and the URL follows.
await nav("arc-picker");
await page.waitForSelector(pageOf("arc-picker"));
const clickedUrl = await page.url();
check("clicking a nav entry changes the URL", slugOf(clickedUrl) === "arc-picker", slugOf(clickedUrl));

// A deep link (a fresh document load at the path, not a click) renders the page
// the path names — this is what a shared link or a refresh does.
await page.goto(`${origin}/morphing-lightbox`, { waitUntil: "networkidle" });
await page.waitForSelector(pageOf("morphing-lightbox"), { timeout: 15000 });
check("a deep link loads its page", (await page.locator(pageOf("morphing-lightbox")).count()) === 1);

await page.reload({ waitUntil: "networkidle" });
await page.waitForSelector(pageOf("morphing-lightbox"), { timeout: 15000 });
check("a refresh keeps the page", (await page.locator(pageOf("morphing-lightbox")).count()) === 1);

// `/` is the shell's default page.
await page.goto(origin, { waitUntil: "networkidle" });
await page.waitForSelector(pageOf("foundation"), { timeout: 15000 });
check("the root serves the default page", (await page.locator(pageOf("foundation")).count()) === 1);

// An unknown path lands on the same notice `render` gives an unknown slug.
await page.goto(`${origin}/no-such-page`, { waitUntil: "networkidle" });
await page.waitForFunction(() => document.body.innerText.includes("Unknown page"), undefined, {
  timeout: 15000,
});
check("an unknown path renders the unknown-page notice", true);

// The small-viewport drawer carries the same list.
await page.goto(origin, { waitUntil: "networkidle" });
await page.waitForSelector(pageOf("foundation"), { timeout: 15000 });
await page.setViewportSize({ width: 420, height: 800 });
await page.waitForTimeout(250);
check(
  "the sidebar hides behind a button at phone width",
  await page.locator("[data-nav-toggle]").isVisible(),
);
await clickIn("[data-nav-toggle]");
await page.waitForSelector("[data-nav-drawer]", { timeout: 5000 });
const drawerLinks = await page.locator("[data-nav-drawer] [data-page-link]").count();
check("the drawer carries the same list", drawerLinks > 100, String(drawerLinks));
await clickIn("[data-nav-drawer] [data-page-link]");
await page.waitForSelector("[data-nav-drawer]", { state: "detached", timeout: 5000 }).catch(() => {});
check(
  "picking a drawer entry closes it",
  (await page.locator("[data-nav-drawer]").count()) === 0,
);
await page.setViewportSize({ width: 1440, height: 900 });

await browser.close();

server.close();

if (failures.length > 0) {
  console.log(`\n${failures.length} failure(s)`);
  process.exit(1);
}
console.log("\nall gallery pages boot");

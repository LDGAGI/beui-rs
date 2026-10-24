#!/usr/bin/env node
/**
 * Generate `gallery/src/registry.rs` from `../ui-components/lib/registry.ts`.
 *
 * The gallery's left navigation is driven by that registry: four categories (`motion`, `charts`, `agents`, `blocks`), each with an ordered
 * list of component entries. The port keeps the same order, names and copy;
 * only the fields the site shell renders are carried over.
 *
 * The registry array is pure data apart from `guide: agentGuides["..."]`, which
 * only the docs pages use — it is stubbed out before evaluating.
 *
 * Usage: node scripts/gen-registry.mjs [path-to-ui-components]
 *        BEUI_SOURCE=/path/to/ui-components node scripts/gen-registry.mjs
 */

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const sourceRoot = process.argv[2] ?? process.env.BEUI_SOURCE;
const src = resolve(
  sourceRoot
    ? join(sourceRoot, "lib", "registry.ts")
    : join(root, "..", "ui-components", "lib", "registry.ts"),
);
if (!existsSync(src)) {
  console.error(`no registry at ${src}`);
  process.exit(1);
}

const source = readFileSync(src, "utf8");
const match = source.match(/export const registry: CategoryEntry\[\] = (\[[\s\S]*?\n\];)/);
if (!match) {
  console.error("could not find `export const registry`");
  process.exit(1);
}
const literal = match[1].replace(/;\s*$/, "");
// `agentGuides` is the only free identifier in the array; stub it.
const registry = eval(
  `(function(){ const agentGuides = new Proxy({}, { get: () => undefined }); return (${literal}); })()`,
);

const esc = (value) => String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"');

let out = `//! The website registry — categories and their components, in source order.
//!
//! Generated from \`../ui-components/lib/registry.ts\` by
//! \`scripts/gen-registry.mjs\`; regenerate rather than edit. Only the fields
//! the navigation needs are carried over — descriptions, file paths, keywords
//! and guides stay in the source.

/// One component, as the navigation names it.
#[derive(Clone, Copy, PartialEq)]
pub struct SiteComponent {
    pub slug: &'static str,
    pub name: &'static str,
}

/// One registry category.
pub struct SiteCategory {
    pub name: &'static str,
    pub components: &'static [SiteComponent],
}

/// \`registry\` from \`lib/registry.ts\`, verbatim order and copy.
pub static REGISTRY: &[SiteCategory] = &[
`;

for (const category of registry) {
  out += `    SiteCategory {\n`;
  out += `        name: "${esc(category.name)}",\n`;
  out += `        components: &[\n`;
  for (const component of category.components) {
    out += `            SiteComponent {\n`;
    out += `                slug: "${esc(component.slug)}",\n`;
    out += `                name: "${esc(component.name)}",\n`;
    out += `            },\n`;
  }
  out += `        ],\n`;
  out += `    },\n`;
}
out += `];\n`;

const dest = join(root, "gallery/src/registry.rs");
writeFileSync(dest, out);
const total = registry.reduce((sum, category) => sum + category.components.length, 0);
console.log(
  `wrote ${dest} — ${registry.length} categories, ${total} components (` +
    registry.map((category) => `${category.slug}=${category.components.length}`).join(" ") +
    ")",
);

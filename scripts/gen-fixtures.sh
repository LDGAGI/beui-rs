#!/usr/bin/env bash
# Regenerate the fidelity fixtures from the beUI source and the exact motion
# version it animates with. Run after bumping `BEUI_SOURCE` or changing a token;
# the golden tests read what this writes.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_dir="${BEUI_SOURCE:-$here/../ui-components}"
motion_dir="${MOTION_DIR:-}"

if [ ! -d "$source_dir" ]; then
  echo "beUI source not found at $source_dir (set BEUI_SOURCE=...)" >&2
  exit 1
fi

# --- lib/themes.ts ----------------------------------------------------------
bun -e "
const m = await import('$source_dir/lib/themes.ts');
const fs = await import('node:fs');
const dir = '$here/crates/beui-theme/tests/fixtures';
fs.mkdirSync(dir, { recursive: true });
fs.writeFileSync(dir + '/themes-stylesheet.txt', m.themesStylesheet() + '\n');
fs.writeFileSync(dir + '/theme-export-default.txt', m.themeExportCss('default') + '\n');
fs.writeFileSync(dir + '/theme-export-indigo.txt', m.themeExportCss('indigo') + '\n');
fs.writeFileSync(
  dir + '/theme-list.txt',
  m.THEME_LIST.map((t) => t.id + '\t' + t.name + '\t' + t.swatch).join('\n') + '\n',
);
console.log('themes fixtures written');
"

# --- motion's cubic-bezier --------------------------------------------------
# Needs `motion-utils` of the same version beUI depends on. `bun add
# motion-utils@<version>` inside a scratch directory and pass it as MOTION_DIR.
if [ -n "$motion_dir" ]; then
  bun -e "
const { cubicBezier } = await import('$motion_dir/node_modules/motion-utils/dist/es/easing/cubic-bezier.mjs');
const fs = await import('node:fs');
const curves = { ease_out: [0.16,1,0.3,1], ease_in_out: [0.77,0,0.175,1], ease_drawer: [0.32,0.72,0,1] };
let out = '';
for (const [name, c] of Object.entries(curves)) {
  const f = cubicBezier(...c);
  out += '[' + name + ']\n';
  for (let i = 0; i <= 100; i++) { const x = i/100; out += x.toFixed(2) + '\t' + f(x).toPrecision(17) + '\n'; }
  for (const x of [-0.5, -0.001, 1.001, 1.5]) out += x + '\t' + f(x).toPrecision(17) + '\n';
}
fs.writeFileSync('$here/crates/beui-core/tests/fixtures/ease-samples.txt', out);
console.log('ease fixture written');
"
else
  echo "MOTION_DIR not set — skipping the cubic-bezier fixture" >&2
fi

#!/usr/bin/env bash
# Regenerate `globals.tailwind.css`, the copy of beUI's `globals.css` that the
# Tailwind v4 build imports.
#
# The only edit is dropping the leading `@import "tailwindcss";`: `gallery/tailwind.css`
# already imports the framework, and importing the same module twice is an
# error. `crates/beui-theme/tests/derived_assets.rs` asserts that this file is
# exactly the source minus that one line, so a drift is a test failure rather
# than a silent style difference.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
assets="$here/crates/beui-theme/assets"
source_css="$assets/globals.css"

[ -f "$source_css" ] || { echo "missing $source_css" >&2; exit 1; }

python3 - "$source_css" "$assets/globals.tailwind.css" <<'PY'
import sys

source, target = sys.argv[1], sys.argv[2]
text = open(source).read()
prefix = '@import "tailwindcss";\n'
if not text.startswith(prefix):
    raise SystemExit(f"{source} no longer starts with {prefix!r}")
open(target, "w").write(text[len(prefix):])
print(f"wrote {target} ({len(text) - len(prefix)} bytes)")
PY

# The woff2 files are served as static assets, so they have to sit in the
# gallery's public directory. Kept byte-identical by the parity test in
# `crates/beui-theme/tests/derived_assets.rs`.
fonts_src="$assets/fonts"
fonts_out="$here/gallery/public/assets/fonts"
mkdir -p "$fonts_out"
cp "$fonts_src"/*.woff2 "$fonts_out/"
echo "synced $(ls "$fonts_src" | wc -l | tr -d ' ') font files to $fonts_out"

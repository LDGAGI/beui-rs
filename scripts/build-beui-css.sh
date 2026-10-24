#!/usr/bin/env bash
# Regenerate `crates/beui-theme/assets/beui.css`, the prebuilt stylesheet a
# consumer can inject instead of running Tailwind themselves.
#
# Why this exists: the components emit literal Tailwind class strings. In this
# repository the gallery's Tailwind build scans `crates/` through `@source`, so
# it sees them. A consumer who installs the crates from crates.io has no such
# path — the sources live under `~/.cargo/registry/src/<hash>/beui-x.y.z`, whose
# name changes with the registry hash — so "just use Tailwind" is not something
# we can document with a copy-pasteable `@source` line. Compiling the classes
# once, here, turns the library into a plain stylesheet they can serve.
#
# The output deliberately omits Tailwind's preflight: it is injected into an
# existing page, and a reset there would be the library clobbering its host.
# The two declarations the components actually rely on are appended below.
#
# Requires Bun (or any `bunx`-compatible runner). The versions are pinned so a
# regeneration is a no-op unless the class strings changed.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
assets="$here/crates/beui-theme/assets"
out="$assets/beui.css"
cli_version="4.3.3"

command -v bunx >/dev/null || { echo "bunx is required" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cat > "$work/input.css" <<EOF
@layer theme, base, components, utilities;
@import "tailwindcss/theme.css" layer(theme);
@import "tailwindcss/utilities.css" layer(utilities);
@import "$assets/globals.tailwind.css";
@source "$here/crates";

/* Not preflight: only what the components need, so injecting this into an
   existing app does not reset it. */
*, ::before, ::after {
    box-sizing: border-box;
}
body {
    margin: 0;
}
EOF

(cd "$work" && bun add --silent "tailwindcss@$cli_version" "@tailwindcss/cli@$cli_version" >/dev/null)
(cd "$work" && bunx "@tailwindcss/cli@$cli_version" -i input.css -o beui.css --minify)

# A sorted class check: Tailwind only emits classes it actually saw, so a
# truncated or empty run shows up as a missing utility rather than a diff.
for needle in ".bg-primary" ".rounded-xl" "@keyframes marquee"; do
    grep -q -- "$needle" "$work/beui.css" || {
        echo "generated stylesheet is missing $needle" >&2
        exit 1
    }
done

cp "$work/beui.css" "$out"
echo "wrote $out ($(wc -c < "$out" | tr -d ' ') bytes)"

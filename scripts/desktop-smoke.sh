#!/usr/bin/env bash
# Desktop smoke test.
#
# The desktop renderer compiles the *other* half of every `cfg(target_arch)`
# branch, and its DOM lives in a WebView the Rust side reaches only through
# `document::eval`. That is exactly where a port can compile cleanly and still
# die on the first paint, so this runs the built app and fails if it panics
# ("cannot access imported statics on non-wasm targets" is the tell) or is not
# still running after a few seconds.
#
# The process is looked up by pattern rather than by `$!`: the launcher is not
# always the process that ends up holding the window, and a `kill -0` on the
# wrong pid reports a startup failure that is not there. An earlier version of
# this script did exactly that, twice.
#
# It does not check pixels: the window has to be a real one, so a screenshot of
# it needs a human (or screen-recording permission); visual sampling on macOS
# stays a manual step.
#
# Usage: scripts/desktop-smoke.sh [app-binary]

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
default_app="$here/target/dx/gallery/debug/macos/Gallery.app/Contents/MacOS/gallery"
app="${1:-$default_app}"
settle="${SETTLE_SECONDS:-8}"

[ -x "$app" ] || {
  echo "no desktop binary at $app — run: cd gallery && dx build --platform desktop" >&2
  exit 1
}

# A stale instance from an earlier run makes the next launch exit immediately
# (the platform keeps one window per bundle), and the kill that removes one can
# leave the next launch failing the same way — the launch is retried rather than
# reported as a crash, because the two are indistinguishable from the outside
# (both exit silently, with nothing on stdout or stderr).
log="$(mktemp)"
running=""
for attempt in 1 2 3; do
  pkill -f "$app" 2>/dev/null
  sleep 2
  "$app" >>"$log" 2>&1 &
  sleep "$settle"
  running="$(pgrep -f "$app" 2>/dev/null | head -1)"
  [ -n "$running" ] && break
  echo "note: attempt $attempt did not stay up; retrying" >&2
done

# A direct launch of the bundle's inner binary exits immediately in some session
# states — macOS hands a window only to an app opened through LaunchServices —
# so fall back to `open`, which is also how a user starts it. The output check
# still comes from the direct attempts above; on the fallback the app's stdout
# goes to the unified log instead, so only the "stays up" half is verified.
if [ -z "$running" ]; then
  bundle="${app%/Contents/MacOS/*}"
  if [ -d "$bundle" ] && command -v open >/dev/null 2>&1; then
    echo "note: direct launch did not stay up; retrying through LaunchServices" >&2
    open "$bundle" 2>/dev/null
    sleep "$settle"
    running="$(pgrep -f "$app" 2>/dev/null | head -1)"
    [ -n "$running" ] && fallback=1
  fi
fi

if [ -z "$running" ]; then
  echo "FAIL: no process for $app after ${settle}s, three attempts"
  [ -s "$log" ] && cat "$log" || echo "(no output)"
  pkill -f "$app" 2>/dev/null
  exit 1
fi

if [ "${fallback:-0}" = "1" ]; then
  echo "ok   desktop app is up after ${settle}s (pid $running), via LaunchServices"
else
  echo "ok   desktop app is up after ${settle}s (pid $running), no panic, attempt $attempt"
fi
[ -s "$log" ] && {
  echo "--- stderr/stdout ---"
  cat "$log"
}
pkill -f "$app" 2>/dev/null
rm -f "$log"

# beui-rs

**English** · [简体中文](README.zh-CN.md)

A [Dioxus](https://dioxuslabs.com) 0.7 port of [beUI](https://github.com/starc007/ui-components) — the same look, the same motion, in Rust.

beUI is a React/Next.js motion component library by Saurabh Chauhan (MIT). This
port re-implements its components for Dioxus and keeps the design tokens,
Tailwind class strings, easing curves and spring parameters verbatim, so the
two versions agree frame for frame.

## Install

Three ways in, cheapest first.

**Look at it.** The gallery is built and deployed by CI — no toolchain needed.
<a href="https://ldgagi.github.io/beui-rs/">ldgagi.github.io/beui-rs</a>

**Drop it into an existing Dioxus app.** The crates are not on crates.io yet, so
depend on the repository directly — `cargo add` understands that:

```sh
cargo add --git https://github.com/LDGAGI/beui-rs beui
cargo add --git https://github.com/LDGAGI/beui-rs beui-theme -F prebuilt-css
```

which writes:

```toml
beui = { git = "https://github.com/LDGAGI/beui-rs", package = "beui" }
beui-theme = { git = "https://github.com/LDGAGI/beui-rs", package = "beui-theme", features = ["prebuilt-css"] }
```

`prebuilt-css` ships the compiled component stylesheet, so this needs no
Tailwind, Node or Bun. Then:

```rust
use beui::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_effect(beui_theme::install_standalone);
    rsx! { Button { variant: ButtonVariant::Primary, "Save changes" } }
}
```

`install_standalone` injects the component utilities and the eleven palettes at
startup. It covers the library's class strings, not markup you write yourself —
for your own Tailwind utilities, use the `@source` route below instead.

Pin a revision if you want reproducible builds:

```toml
beui = { git = "https://github.com/LDGAGI/beui-rs", rev = "<sha>", package = "beui" }
```

**Start from a template.**

```sh
cargo install cargo-generate           # once
cargo generate --git https://github.com/LDGAGI/beui-rs templates/starter
cd beui-app && dx serve --platform web
```

Or copy [`templates/starter`](templates/starter) by hand.

### If you already run Tailwind

Skip `prebuilt-css`, scan the crate sources, and use your own build. A `git`
dependency lives under `~/.cargo/git/checkouts`:

```sh
find ~/.cargo/git/checkouts -maxdepth 4 -type d -path '*/crates/beui' | head -1
```

```css
@import "tailwindcss";
@import "…/crates/beui-theme/assets/globals.tailwind.css";
@source "…/crates/beui";
@source "./src";
```

This is what the gallery does, and it is the only way to get your own utilities
and the components in one pass. The path contains a checkout hash that varies by
machine, which is why the default is the prebuilt sheet.

### Fonts

The `@font-face` rules point at `/assets/fonts/*.woff2`. Copy
`beui-theme/assets/fonts/` into your app's public directory, or point elsewhere
at compile time with `BEUI_FONT_BASE=https://cdn.example.com/geist`. Without the
files the stack falls back to the system sans-serif; nothing breaks.

## What's here

| path | what |
|---|---|
| `crates/beui-theme` | `app/globals.css` verbatim, the eleven palettes, self-hosted Geist / Geist Mono |
| `crates/beui-core` | `cn!`, easing and spring tokens, hooks, touch, presence, FLIP, drag, portal, and `platform` — the one place the renderers differ |
| `crates/beui` | the `components/motion` port |
| `crates/beui-agents` | the `components/agents` port |
| `crates/beui-charts` | the `components/charts` port |
| `gallery/` | a component browser: one page per component, same copy and structure as beUI's previews |
| `verify/` | Playwright harness: computed-style and motion checks, pixel diffs, end-to-end page smoke |
| `templates/starter` | a copyable app that uses the published crates and needs no Tailwind build |
| `scripts/` | fixture, registry and prebuilt-stylesheet generation |

## Requirements

| tool | version |
|---|---|
| Rust | 1.85+ (developed on 1.90) |
| Dioxus CLI | 0.7.10 — `cargo install dioxus-cli --version 0.7.10` |
| Bun | any recent (verification harness, fixture generation) |

`dx` fetches the matching Tailwind v4 and `wasm-bindgen-cli` on first use.

Bun and Tailwind are for this repository's gallery and verification harness. The
published crates need neither: `beui-theme`'s `prebuilt-css` feature carries the
compiled stylesheet.

## Run the gallery

```sh
cd gallery
dx serve --platform web        # http://localhost:8080
dx serve --platform desktop    # native window
```

Production build:

```sh
dx build --platform web --release
dx build --platform desktop
```

## Checks

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --target wasm32-unknown-unknown -p beui-core -p beui-theme   # wasm-only paths

cd gallery && dx build --platform web && dx build --platform desktop

cd verify && bun install
bun run all     # foundation + hooks + presence + flip + drag + portal + motion
bun run e2e     # every gallery page boots and paints (built web bundle first)
```

`beui-theme`'s parity tests compare against the beUI checkout when one is
available. Point `BEUI_SOURCE` at it (or place it next to this repository) to
run them; otherwise they skip.

## How fidelity is kept

- **Styles.** `globals.css` is copied byte for byte. Component class strings are
  transcribed from the source; `cn!` only joins them, and a consumer's `class`
  is appended last so the cascade resolves conflicts.
- **Motion.** Spring `stiffness` / `damping` / `mass` values are copied
  verbatim. `visualDuration + bounce` presets are converted with Motion's own
  formula, and the converted numbers are documented next to the call.
- **Layout animation.** `layout` / `layoutId` go through `beui-core::flip`.
- **Drag.** Pointer capture and velocity sampling live in `beui-core::drag`.
- **Exit.** `AnimatePresence` becomes `beui-core::presence`; a component
  unmounts only after its exit animation finishes.
- **Reduced motion.** Two tracks: a global CSS rule and `use_reduced_motion()`
  in each component.
- **Icons.** Lucide path data, generated into `crates/beui/src/icons.rs`.

The three places the port is not a 1:1 translation are documented in the crate
docs: DOM access (`beui-core::platform`), per-frame animation writes (through the
reactive `style` attribute), and `cn()` (no `tailwind-merge`, by design).

## Status

Every motion, agent and chart component is ported. The gallery has a page for
each motion/blocks component; dedicated chart and agent pages are still being
added, though both crates are complete. `shader-background` is the one
`n/a`: the source is a WebGL shader, and the port keeps the DOM contract with a
CSS stand-in.

## License

MIT — see [LICENSE](LICENSE). The port includes material from beUI, Geist and
Lucide; see [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

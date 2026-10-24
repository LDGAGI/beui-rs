# Contributing

This project is a fidelity port: the goal is not "a Dioxus component library
inspired by beUI", it is the same components, with the same DOM, class strings
and motion parameters, running on Dioxus. Changes are judged against that.

## Before you start

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd gallery && dx build --platform web && dx build --platform desktop
```

Both renderers have to pass. They compile different code: `web-sys` compiles for
the host target but panics when called off-wasm, and the desktop renderer's DOM
lives in a WebView that Rust cannot reach directly. A web-only change can break
desktop and vice versa.

## Porting rules

1. **Styles come from the source.** Copy class strings verbatim from the beUI
   component. `cn!` only joins with spaces; a consumer's `class` goes last.
2. **Springs are copied, not tuned.** Keep `stiffness` / `damping` / `mass`
   byte-identical. For `visualDuration + bounce` presets, convert with Motion's
   formula and leave the converted value in a comment.
3. **`layout` / `layoutId`** always go through `beui-core::flip`.
4. **`drag`** always goes through `beui-core::drag`; copy the thresholds.
5. **Exit animations** go through `beui-core::presence`; unmount only when the
   animation finishes.
6. **Reduced motion has two tracks:** the global CSS rule and
   `use_reduced_motion()` in the component.
7. **Pure-JS algorithms** (popover goo, wheel picking, tab inversion) are
   translated line by line, not approximated.
8. **Icons** use Lucide path data, generated into `crates/beui/src/icons.rs`.
9. **API conventions:** enums for variants, `Signal` for state, `EventHandler`
   for callbacks, `children` for content, theme variables for color.

Known platform differences (FLIP paths, spring conversion, `backdrop-filter`
rendering, WebKitGTK view transitions, JS-only dependencies) are documented in
the module docs that touch them. Record a measured difference rather than
hiding it.

## Adding a component

1. Read the source component in beUI. List its DOM, class strings, props, state,
   animation parameters, and which foundations it uses.
2. Add the missing foundation first if there is one.
3. Write the Rust component next to its peers.
4. Add a gallery page that reproduces the beUI preview.
5. Run the checks above; make sure the page mounts in `verify`'s e2e smoke.

## Generated files

- `crates/beui/src/icons.rs` — `node scripts/gen-icons.mjs` (needs a
  `lucide-react` checkout; pass its path or set `LUCIDE_REACT_DIR`).
- `gallery/src/registry.rs` — `node scripts/gen-registry.mjs` (needs the beUI
  checkout; pass its path or set `BEUI_SOURCE`).
- `crates/beui-theme/assets/globals.tailwind.css` and the gallery's font copy —
  `scripts/sync-tailwind-input.sh`.
- `crates/beui-theme/assets/beui.css` — `scripts/build-beui-css.sh` (needs Bun;
  pins the Tailwind CLI version). This is the stylesheet consumers get from the
  `prebuilt-css` feature, so it has to be regenerated whenever a class string
  changes. CI regenerates it and fails on a diff.
- `crates/beui-theme/tests/fixtures/*` — `scripts/gen-fixtures.sh` (needs Bun
  and the beUI checkout).

Do not hand-edit those files; the theme crate's tests fail when they drift.

## Releasing

Version lives once in the workspace manifest (`Cargo.toml` →
`[workspace.package] version`), and every crate inherits it. Path dependencies
also carry that version so `cargo package` can substitute the registry copy.
Bump it, regenerate the derived files above, then tag:

```sh
git tag v0.1.0 && git push origin v0.1.0
```

`.github/workflows/release.yml` publishes `beui-core` → `beui-theme` → `beui` →
`beui-agents` → `beui-charts`, sleeping between each so the index catches up.
It needs a `CARGO_REGISTRY_TOKEN` secret. Do not tag until CI's "Every crate is
publishable" step is green; a missing `version` on a path dependency only fails
at that point.

## Reporting fidelity bugs

Include the component, the renderer (web or desktop), the browser/OS, and — if
you can — the source value and the ported value (spring numbers, a class string,
a measured rect). Screenshots at the same viewport help more than descriptions.

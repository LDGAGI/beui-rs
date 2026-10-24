# beUI starter

A one-page Dioxus app using beUI components. No Tailwind, no Node, no build
step beyond `dx`.

## Run it

```sh
cargo install dioxus-cli --version 0.7.10   # once
rustup target add wasm32-unknown-unknown    # once, for the web build

dx serve --platform web      # http://localhost:8080
dx serve --platform desktop  # native window
```

Release build:

```sh
dx build --platform web --release   # → dist/
dx build --platform desktop --release
```

## What is wired up

| piece | why |
|---|---|
| `beui-theme = { features = ["prebuilt-css"] }` | compiles the component stylesheet into the binary |
| `install_standalone()` | injects that stylesheet plus the eleven palettes once, at startup |
| `dioxus-motion` with a platform feature | without it the wasm bundle imports `env.now` and never boots |

`install_standalone()` covers the library's own class strings. Markup you write
yourself is not in that sheet — use inline styles, your own CSS, or add Tailwind
(see below).

## Adding Tailwind

If you want Tailwind utilities in your own markup, drop `prebuilt-css` and
`install_standalone`, and let `dx` run Tailwind over both your markup and the
library sources. `dx` fetches the Tailwind v4 CLI itself, so there is nothing to
install.

`Dioxus.toml`:

```toml
[application]
tailwind_input = "tailwind.css"
tailwind_output = "public/assets/tailwind.css"

[web.resource]
style = ["/assets/tailwind.css"]
```

`tailwind.css` has to point `@source` at the installed crate. That path lives
under Cargo's cache and contains a hash that changes between machines — a
`git` dependency lands in `~/.cargo/git/checkouts`, a crates.io one in
`~/.cargo/registry/src`:

```css
@import "tailwindcss";
@source "./src";
@source "…/crates/beui";
```

Resolve it locally:

```sh
find ~/.cargo/git/checkouts -maxdepth 4 -type d -path '*/crates/beui' | head -1
find ~/.cargo/registry/src  -maxdepth 2 -type d -name 'beui-0.1.0'      | head -1
```

Then call `beui_theme::install_theme` instead of `install_standalone`, and
import the tokens so `bg-background` and friends exist (same `crates/beui-theme`
root):

```css
@import "…/crates/beui-theme/assets/globals.tailwind.css";
```

That is exactly the friction `prebuilt-css` removes. Use Tailwind only if you
need your own utilities in the same pass.

## Fonts

`install_theme` points `@font-face` at `/assets/fonts/*.woff2`. The files are in
`beui-theme/assets/fonts/`; copy them somewhere your app serves, or set
`BEUI_FONT_BASE` at compile time:

```sh
BEUI_FONT_BASE=https://cdn.example.com/geist dx build --platform web --release
```

Without them the stack falls through to the system sans-serif — the components
still render.

## Editing the template

This directory is excluded from the beui-rs workspace, so it resolves `beui`
from the git repository. To work against a local checkout, replace the two
dependencies with paths:

```toml
beui = { path = "../../crates/beui" }
beui-theme = { path = "../../crates/beui-theme", features = ["prebuilt-css"] }
```

If you later publish the crates, switch both to a version:

```toml
beui = "0.1.0"
beui-theme = { version = "0.1.0", features = ["prebuilt-css"] }
```

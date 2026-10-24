//! A beUI starter: one themed page, no Tailwind build.
//!
//! `install_standalone()` injects two stylesheets at runtime — the prebuilt
//! component utilities and the eleven color palettes. Because the utilities
//! were compiled from the library's own sources, they cover the components but
//! not markup you write here: for your own layout, either add Tailwind to this
//! app (see the README) or use plain styles as below.

use beui::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Dark mode + palette state, and the stylesheet injection.
    let _dark = beui_theme::use_dark_mode_provider();
    let _color = beui_theme::use_color_theme_provider();
    use_effect(beui_theme::install_standalone);

    rsx! {
        main {
            style: "max-width: 40rem; margin: 0 auto; padding: 4rem 1.5rem;",
            h1 { style: "font-size: 1.5rem; margin-bottom: 1.5rem;", "beUI starter" }
            div {
                style: "display: flex; gap: 0.75rem; align-items: center;",
                Button { variant: ButtonVariant::Primary, "Save changes" }
                Button { variant: ButtonVariant::Outline, "Cancel" }
            }
        }
    }
}

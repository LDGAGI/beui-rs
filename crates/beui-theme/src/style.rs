//! `app/globals.css`, verbatim, plus the runtime-injected styles.

use beui_core::platform;

/// A byte-for-byte copy of `app/globals.css` from the beUI source tree.
///
/// Loaded into the page by the Tailwind v4 build: `gallery/tailwind.css` imports
/// the `@import`-stripped twin in `assets/globals.tailwind.css`, which is what
/// makes `bg-card`, `text-muted-foreground`, `animate-marquee` and the custom
/// `--ease-*` variables exist at all. This constant is the reference copy —
/// `tests/globals_verbatim.rs` checks it against the source file, and
/// `tests/derived_assets.rs` checks the twin against it.
pub const GLOBALS_CSS: &str = include_str!("../assets/globals.css");

/// The verbatim `app/globals.css` text.
pub fn globals_css() -> &'static str {
    GLOBALS_CSS
}

/// The runtime stylesheet: the font faces, the `--font-geist-*` variables
/// `next/font` would have set, and all eleven palettes.
///
/// beUI injects `themesStylesheet()` through `PreferencesProvider`; the fonts are
/// a `next/font` side effect there and an explicit `@font-face` block here. Both
/// live in one stylesheet so switching a palette only rewrites an attribute.
pub fn theme_css() -> String {
    format!("{}\n{}", crate::fonts_css(), crate::themes_stylesheet())
}

/// The same stylesheet wrapped in a `<style data-beui-theme>` element.
pub fn theme_style_tag() -> String {
    format!("<style data-beui-theme>{}</style>", theme_css())
}

/// The script that installs [`theme_css`], written as JS so the desktop
/// and mobile renderers (which cannot touch the DOM from Rust) run the same
/// code path as the web renderer.
fn install_script() -> String {
    format!(
        "if (!document.querySelector('style[data-beui-theme]')) {{\n\
           const style = document.createElement('style');\n\
           style.setAttribute('data-beui-theme', '');\n\
           style.textContent = {};\n\
           document.head.appendChild(style);\n\
         }}",
        platform::js_string(&theme_css())
    )
}

/// Append the theme `<style>` tag to `document.head`, once.
///
/// Call from an effect at the app root. Idempotent — a second call finds the tag
/// already there and does nothing.
pub fn install_theme() {
    platform::fire(&install_script());
}

/// The component utilities, compiled ahead of time by
/// `scripts/build-beui-css.sh` and shipped in this crate.
///
/// This is the fallback for apps that do not run Tailwind. A Tailwind app
/// should scan the crate sources with `@source` and use its own build instead:
/// that produces a smaller sheet and lets the app's own utilities share one
/// pass. Available only with the `prebuilt-css` feature.
#[cfg(feature = "prebuilt-css")]
pub const COMPONENTS_CSS: &str = include_str!("../assets/beui.css");

/// [`COMPONENTS_CSS`], for callers that want to write it out themselves.
#[cfg(feature = "prebuilt-css")]
pub fn components_css() -> &'static str {
    COMPONENTS_CSS
}

/// The script that installs [`COMPONENTS_CSS`], once, as
/// `<style data-beui-styles>`.
#[cfg(feature = "prebuilt-css")]
fn install_styles_script() -> String {
    format!(
        "if (!document.querySelector('style[data-beui-styles]')) {{\n\
           const style = document.createElement('style');\n\
           style.setAttribute('data-beui-styles', '');\n\
           style.textContent = {};\n\
           document.head.appendChild(style);\n\
         }}",
        platform::js_string(COMPONENTS_CSS)
    )
}

/// Append the prebuilt component stylesheet to `document.head`, once.
///
/// Call this **instead of** running Tailwind over the crates. No preflight is
/// included, so it does not reset the host page; the components' two
/// dependencies (`box-sizing`, `body` margin) are part of the sheet.
#[cfg(feature = "prebuilt-css")]
pub fn install_styles() {
    platform::fire(&install_styles_script());
}

/// [`install_styles`] plus [`install_theme`] — everything an app that does not
/// use Tailwind needs, in one call.
///
/// ```ignore
/// use_effect(beui_theme::install_standalone);
/// ```
#[cfg(feature = "prebuilt-css")]
pub fn install_standalone() {
    install_styles();
    install_theme();
}

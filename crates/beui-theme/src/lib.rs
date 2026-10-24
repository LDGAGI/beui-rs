//! beUI design tokens, theme presets and self-hosted Geist fonts.
//!
//! Everything visual that beUI keeps in CSS lives here, unchanged:
//!
//! - [`globals_css`] — a byte-for-byte copy of `app/globals.css` (tokens,
//!   `@theme`, keyframes, glass utilities, reduced-motion). It is imported by
//!   the Tailwind v4 input (`theme/input.css`) so the utilities exist in the
//!   built stylesheet.
//! - [`themes_stylesheet`] — the eleven full token sets from `lib/themes.ts`,
//!   keyed by `data-theme`; inject once at startup.
//! - [`fonts_css`] — `@font-face` for the same Geist / Geist Mono woff2 files
//!   `next/font/google` serves in the source app.
//! - [`use_dark_mode`] / [`use_color_theme`] — the `next-themes`
//!   (`attribute="class"`, `defaultTheme="dark"`, `enableSystem`) and
//!   `PreferencesProvider` behaviour: class strategy on `<html>`, in-memory
//!   color theme, `data-theme` deleted for the default palette.

mod color_mode;
mod fonts;
mod style;
mod themes;

pub use color_mode::{
    use_color_theme, use_color_theme_provider, use_dark_mode, use_dark_mode_provider, ColorMode,
    ColorTheme, DarkMode,
};
pub use fonts::{font_base_path, fonts_css, FONT_BASE_PATH, FONT_FILES};
#[cfg(feature = "prebuilt-css")]
pub use style::{components_css, install_standalone, install_styles, COMPONENTS_CSS};
pub use style::{globals_css, install_theme, theme_css, theme_style_tag};
pub use themes::{
    theme_export_css, theme_list, themes_stylesheet, themes_stylesheet_cached, Theme, ThemeId,
    BASE_DARK, BASE_LIGHT, THEMES,
};

//! Geist / Geist Mono, self-hosted.
//!
//! `app/layout.tsx` pulls both through `next/font/google`, which downloads the
//! variable woff2 files and exposes them as `--font-geist-sans` /
//! `--font-geist-mono`. These are the same files.
//!
//! They are served as plain static assets (`/assets/fonts/<name>.woff2`, from
//! the app's public directory — `scripts/sync-tailwind-input.sh` keeps that copy
//! in step with `assets/fonts/`) rather than through `asset!()`: the macro
//! resolves at *compile* time, and the gallery's stylesheet is written by the
//! Tailwind step that `dx` runs before compiling, so a plain `cargo clippy` on a
//! fresh checkout would fail on a file that does not exist yet.

/// Every font file, in the order the `@font-face` rules are emitted: the
/// `unicode-range` list, so the browser can pick a subset.
pub const FONT_FILES: [&str; 11] = [
    "Geist-cyrillic-ext.woff2",
    "Geist-cyrillic.woff2",
    "Geist-vietnamese.woff2",
    "Geist-latin-ext.woff2",
    "Geist-latin.woff2",
    "GeistMono-cyrillic-ext.woff2",
    "GeistMono-cyrillic.woff2",
    "GeistMono-symbols2.woff2",
    "GeistMono-vietnamese.woff2",
    "GeistMono-latin-ext.woff2",
    "GeistMono-latin.woff2",
];

/// `unicode-range` per subset, copied from the Google Fonts CSS that
/// `next/font/google` resolves at build time. Order matters: the browser picks
/// the first matching face, so the list stays in the source order.
const UNICODE_RANGES: [(&str, &str); 11] = [
    ("Geist-cyrillic-ext.woff2", "U+0460-052F, U+1C80-1C8A, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F"),
    ("Geist-cyrillic.woff2", "U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116"),
    ("Geist-vietnamese.woff2", "U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+0300-0301, U+0303-0304, U+0308-0309, U+0323, U+0329, U+1EA0-1EF9, U+20AB"),
    ("Geist-latin-ext.woff2", "U+0100-02BA, U+02BD-02C5, U+02C7-02CC, U+02CE-02D7, U+02DD-02FF, U+0304, U+0308, U+0329, U+1D00-1DBF, U+1E00-1E9F, U+1EF2-1EFF, U+2020, U+20A0-20AB, U+20AD-20C4, U+2113, U+2C60-2C7F, U+A720-A7FF"),
    ("Geist-latin.woff2", "U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD"),
    ("GeistMono-cyrillic-ext.woff2", "U+0460-052F, U+1C80-1C8A, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F"),
    ("GeistMono-cyrillic.woff2", "U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116"),
    ("GeistMono-symbols2.woff2", "U+2000-2001, U+2004-2008, U+200A, U+23B8-23BD, U+2500-259F"),
    ("GeistMono-vietnamese.woff2", "U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+0300-0301, U+0303-0304, U+0308-0309, U+0323, U+0329, U+1EA0-1EF9, U+20AB"),
    ("GeistMono-latin-ext.woff2", "U+0100-02BA, U+02BD-02C5, U+02C7-02CC, U+02CE-02D7, U+02DD-02FF, U+0304, U+0308, U+0329, U+1D00-1DBF, U+1E00-1E9F, U+1EF2-1EFF, U+2020, U+20A0-20AB, U+20AD-20C4, U+2113, U+2C60-2C7F, U+A720-A7FF"),
    ("GeistMono-latin.woff2", "U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD"),
];

/// Where the woff2 files are served from.
///
/// The gallery copies them into `gallery/public/assets/fonts`. A consuming app
/// does the same, or points this somewhere else at compile time with
/// `BEUI_FONT_BASE=https://cdn.example.com/geist`; without either, the
/// `@font-face` rules 404 and the stack falls through to the system sans-serif
/// (the components still render, just not in Geist).
pub const FONT_BASE_PATH: &str = "/assets/fonts";

/// [`FONT_BASE_PATH`], unless the app overrode it at compile time.
pub fn font_base_path() -> &'static str {
    option_env!("BEUI_FONT_BASE").unwrap_or(FONT_BASE_PATH)
}

fn family(file: &str) -> &'static str {
    if file.starts_with("GeistMono") {
        "Geist Mono"
    } else {
        "Geist"
    }
}

/// `@font-face` rules for both families.
///
/// `font-weight: 100 900` and `font-display: swap` match what `next/font/google`
/// emits for a variable font, so metrics and the swap window are identical.
/// `globals.css` maps `--font-sans` / `--font-mono` / `--font-display` onto
/// `var(--font-geist-sans)` / `var(--font-geist-mono)`, so these are the two
/// family names that must exist.
pub fn fonts_css() -> String {
    let mut css = String::from(
        "/* Geist / Geist Mono — self-hosted, same files next/font/google serves in beUI. */\n",
    );
    // `next/font` exposes the loaded families as `--font-geist-sans` /
    // `--font-geist-mono` (it puts the generated class on `<html>`), and
    // `globals.css` declares `--font-sans: var(--font-geist-sans), …`. Without
    // these two declarations the `var()` is undefined, and CSS drops the whole
    // `font-family` in `@layer base` — the page silently falls back to the UA
    // default font, which is a pixel diff on every single component.
    css.push_str(":root {\n  --font-geist-sans: 'Geist';\n  --font-geist-mono: 'Geist Mono';\n}\n");
    let base = font_base_path();
    for file in FONT_FILES {
        let range = UNICODE_RANGES
            .iter()
            .find(|(name, _)| *name == file)
            .map(|(_, range)| *range)
            .unwrap_or_default();
        css.push_str(&format!(
            "@font-face {{\n  font-family: '{}';\n  font-style: normal;\n  font-weight: 100 900;\n  font-display: swap;\n  src: url('{base}/{file}') format('woff2');\n  unicode-range: {range};\n}}\n",
            family(file),
        ));
    }
    css
}

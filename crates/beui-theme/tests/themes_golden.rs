//! `lib/themes.ts` parity.
//!
//! Fixtures come from `scripts/gen-fixtures.sh`, which runs the actual
//! TypeScript through Bun. The stylesheet is compared byte for byte: it is
//! injected into the page verbatim, so any formatting difference would be a
//! difference in the shipped CSS.

use beui_theme::{theme_export_css, theme_list, themes_stylesheet, ThemeId};

#[test]
fn stylesheet_matches_themes_ts() {
    // The fixtures are newline-terminated (git-friendly); the generated text is
    // not, so the comparison adds the one trailing newline back.
    let expected = include_str!("fixtures/themes-stylesheet.txt");
    assert_eq!(themes_stylesheet() + "\n", expected);
}

#[test]
fn export_css_matches_themes_ts() {
    assert_eq!(
        theme_export_css(ThemeId::Default) + "\n",
        include_str!("fixtures/theme-export-default.txt")
    );
    assert_eq!(
        theme_export_css(ThemeId::Indigo) + "\n",
        include_str!("fixtures/theme-export-indigo.txt")
    );
}

#[test]
fn theme_list_matches_themes_ts() {
    let listing = theme_list()
        .into_iter()
        .map(|(id, name, swatch)| format!("{}\t{}\t{}", id.id(), name, swatch))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(listing, include_str!("fixtures/theme-list.txt"));
}

#[test]
fn default_is_not_re_emitted_as_a_theme_selector() {
    // `globals.css` already carries the default tokens; `themesStylesheet()`
    // filters it out.
    let css = themes_stylesheet();
    assert!(!css.contains("[data-theme=\"default\"]"));
    assert!(css.contains("[data-theme=\"violet\"]"));
    assert!(css.contains(".dark[data-theme=\"blood-orange\"]"));
}

#[test]
fn every_theme_id_round_trips() {
    for id in ThemeId::ALL {
        assert_eq!(ThemeId::from_id(id.id()), Some(id));
    }
    assert_eq!(ThemeId::from_id("nope"), None);
}

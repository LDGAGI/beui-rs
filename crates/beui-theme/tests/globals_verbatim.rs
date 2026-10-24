//! The copy of `app/globals.css` must stay byte-identical to the source.
//!
//! beUI's whole look is in this one file; the port keeps it verbatim and lets
//! Tailwind's own build do the work, so a stray edit here is a fidelity bug
//! rather than a refactor. The test compares against a beUI checkout when one
//! is available and skips (rather than fails) otherwise: set `BEUI_SOURCE` to
//! the checkout, or place it next to this repository. `cargo test` runs with
//! the crate root as its working directory, so the default walks up from
//! `beui-rs/crates/beui-theme` to the repository's parent.

use std::path::PathBuf;

fn source_path() -> Option<PathBuf> {
    let path = match std::env::var_os("BEUI_SOURCE") {
        Some(dir) => PathBuf::from(dir).join("app/globals.css"),
        None => PathBuf::from("../../../ui-components/app/globals.css"),
    };
    path.exists().then_some(path)
}

#[test]
fn globals_css_is_verbatim() {
    let copy = beui_theme::globals_css();
    let Some(source) = source_path() else {
        eprintln!("skipping: set BEUI_SOURCE to a beUI checkout to run this parity test");
        return;
    };
    let original = std::fs::read_to_string(&source).expect("read source globals.css");
    assert_eq!(
        copy,
        original,
        "crates/beui-theme/assets/globals.css has drifted from {}",
        source.display()
    );

    // The rules the port depends on, named so a regression is legible.
    for needle in [
        "@custom-variant dark (&:where(.dark, .dark *))",
        "--ease-out: cubic-bezier(0.16, 1, 0.3, 1);",
        "--ease-in-out: cubic-bezier(0.77, 0, 0.175, 1);",
        "--ease-drawer: cubic-bezier(0.32, 0.72, 0, 1);",
        "@keyframes marquee-vertical",
        "@keyframes action-settings",
        ".glass-strong",
        "@media (prefers-reduced-motion: reduce)",
        "animation-duration: 0.01ms !important;",
    ] {
        assert!(copy.contains(needle), "globals.css lost `{needle}`");
    }
}

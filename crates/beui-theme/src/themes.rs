//! Color theme presets — a byte-for-byte port of `lib/themes.ts` from beUI.
//!
//! `brand()` in TypeScript spreads `BASE_LIGHT` / `BASE_DARK` and then overrides
//! the five brand keys. JavaScript keeps an overridden key at its original
//! position in the object, so the emitted `--key: value;` order is the base
//! order with values swapped. [`Theme::light_vars`] reproduces that exactly.

use std::sync::OnceLock;

/// Theme ids in the same order as `THEMES` in `lib/themes.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeId {
    Default,
    Violet,
    Blue,
    Green,
    Amber,
    BloodOrange,
    Rose,
    Red,
    Teal,
    Indigo,
    Lime,
}

impl ThemeId {
    /// Every theme, in `THEMES` order.
    pub const ALL: [ThemeId; 11] = [
        ThemeId::Default,
        ThemeId::Violet,
        ThemeId::Blue,
        ThemeId::Green,
        ThemeId::Amber,
        ThemeId::BloodOrange,
        ThemeId::Rose,
        ThemeId::Red,
        ThemeId::Teal,
        ThemeId::Indigo,
        ThemeId::Lime,
    ];

    /// The `data-theme` value. `default` is not written to the DOM (beUI deletes
    /// the attribute instead), but it is what `themeExportCss` keys on.
    pub const fn id(self) -> &'static str {
        match self {
            ThemeId::Default => "default",
            ThemeId::Violet => "violet",
            ThemeId::Blue => "blue",
            ThemeId::Green => "green",
            ThemeId::Amber => "amber",
            ThemeId::BloodOrange => "blood-orange",
            ThemeId::Rose => "rose",
            ThemeId::Red => "red",
            ThemeId::Teal => "teal",
            ThemeId::Indigo => "indigo",
            ThemeId::Lime => "lime",
        }
    }

    pub fn from_id(id: &str) -> Option<ThemeId> {
        ThemeId::ALL.into_iter().find(|t| t.id() == id)
    }

    pub const fn theme(self) -> &'static Theme {
        &THEMES[match self {
            ThemeId::Default => 0,
            ThemeId::Violet => 1,
            ThemeId::Blue => 2,
            ThemeId::Green => 3,
            ThemeId::Amber => 4,
            ThemeId::BloodOrange => 5,
            ThemeId::Rose => 6,
            ThemeId::Red => 7,
            ThemeId::Teal => 8,
            ThemeId::Indigo => 9,
            ThemeId::Lime => 10,
        }]
    }
}

/// `name` / `swatch` for the theme picker.
pub struct Theme {
    pub name: &'static str,
    pub swatch: &'static str,
    /// Brand-token overrides applied on top of [`BASE_LIGHT`].
    pub light: &'static [(&'static str, &'static str)],
    /// Brand-token overrides applied on top of [`BASE_DARK`].
    pub dark: &'static [(&'static str, &'static str)],
}

impl Theme {
    /// Full light token set in CSS declaration order, exactly as `brand()` spreads it.
    pub fn light_vars(&self) -> Vec<(&'static str, &'static str)> {
        merge(BASE_LIGHT, self.light)
    }

    /// Full dark token set in CSS declaration order.
    pub fn dark_vars(&self) -> Vec<(&'static str, &'static str)> {
        merge(BASE_DARK, self.dark)
    }
}

fn merge(
    base: &'static [(&'static str, &'static str)],
    overrides: &'static [(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    base.iter()
        .map(|(k, v)| {
            let value = overrides
                .iter()
                .find(|(ok, _)| ok == k)
                .map(|(_, ov)| *ov)
                .unwrap_or(*v);
            (*k, value)
        })
        .collect()
}

/// `BASE_LIGHT` from `lib/themes.ts`, verbatim.
pub const BASE_LIGHT: &[(&str, &str)] = &[
    ("--background", "oklch(99% 0 0)"),
    ("--foreground", "oklch(15% 0 0)"),
    ("--card", "oklch(97% 0 0)"),
    ("--card-foreground", "oklch(15% 0 0)"),
    ("--popover", "oklch(97% 0 0)"),
    ("--popover-foreground", "oklch(15% 0 0)"),
    ("--primary", "#0285f7"),
    ("--primary-foreground", "#ffffff"),
    ("--secondary", "oklch(97% 0 0)"),
    ("--secondary-foreground", "oklch(15% 0 0)"),
    ("--muted", "oklch(97% 0 0)"),
    ("--muted-foreground", "oklch(50% 0 0)"),
    ("--accent", "oklch(72% 0.18 195)"),
    ("--accent-foreground", "oklch(15% 0 0)"),
    ("--destructive", "oklch(62% 0.22 25)"),
    ("--border", "oklch(15% 0 0 / 0.06)"),
    ("--input", "oklch(15% 0 0 / 0.06)"),
    ("--ring", "oklch(15% 0 0 / 0.12)"),
];

/// `BASE_DARK` from `lib/themes.ts`, verbatim.
pub const BASE_DARK: &[(&str, &str)] = &[
    ("--background", "#151515"),
    ("--foreground", "oklch(96% 0 0)"),
    ("--card", "#1c1c1c"),
    ("--card-foreground", "oklch(96% 0 0)"),
    ("--popover", "#1c1c1c"),
    ("--popover-foreground", "oklch(96% 0 0)"),
    ("--primary", "#0285f7"),
    ("--primary-foreground", "#ffffff"),
    ("--secondary", "#1c1c1c"),
    ("--secondary-foreground", "oklch(96% 0 0)"),
    ("--muted", "#1c1c1c"),
    ("--muted-foreground", "oklch(62% 0 0)"),
    ("--accent", "oklch(80% 0.18 195)"),
    ("--accent-foreground", "#151515"),
    ("--destructive", "oklch(62% 0.22 25)"),
    ("--border", "rgb(255 255 255 / 0.05)"),
    ("--input", "rgb(255 255 255 / 0.05)"),
    ("--ring", "rgb(255 255 255 / 0.1)"),
];

/// `THEMES` from `lib/themes.ts`, in order. `default` has no overrides — its
/// full set *is* the base.
pub const THEMES: [Theme; 11] = [
    Theme {
        name: "beUI",
        swatch: "#0285f7",
        light: &[],
        dark: &[],
    },
    Theme {
        name: "Violet",
        swatch: "oklch(55% 0.2 290)",
        light: &[
            ("--primary", "oklch(55% 0.2 290)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(55% 0.2 290)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(55% 0.2 290 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(72% 0.16 290)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(72% 0.16 290)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(72% 0.16 290 / 0.55)"),
        ],
    },
    Theme {
        name: "Blue",
        swatch: "oklch(55% 0.18 255)",
        light: &[
            ("--primary", "oklch(55% 0.18 255)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(55% 0.18 255)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(55% 0.18 255 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(70% 0.15 255)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(70% 0.15 255)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(70% 0.15 255 / 0.55)"),
        ],
    },
    Theme {
        name: "Green",
        swatch: "oklch(56% 0.14 150)",
        light: &[
            ("--primary", "oklch(56% 0.14 150)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(56% 0.14 150)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(56% 0.14 150 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(72% 0.15 150)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(72% 0.15 150)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(72% 0.15 150 / 0.55)"),
        ],
    },
    Theme {
        name: "Amber",
        swatch: "oklch(74% 0.15 70)",
        light: &[
            ("--primary", "oklch(74% 0.15 70)"),
            ("--primary-foreground", "oklch(20% 0.02 70)"),
            ("--accent", "oklch(74% 0.15 70)"),
            ("--accent-foreground", "oklch(20% 0.02 70)"),
            ("--ring", "oklch(74% 0.15 70 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(80% 0.15 75)"),
            ("--primary-foreground", "oklch(18% 0.02 75)"),
            ("--accent", "oklch(80% 0.15 75)"),
            ("--accent-foreground", "oklch(18% 0.02 75)"),
            ("--ring", "oklch(80% 0.15 75 / 0.55)"),
        ],
    },
    Theme {
        name: "Blood Orange",
        swatch: "oklch(60% 0.19 40)",
        light: &[
            ("--primary", "oklch(60% 0.19 40)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(60% 0.19 40)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(60% 0.19 40 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(72% 0.17 42)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(72% 0.17 42)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(72% 0.17 42 / 0.55)"),
        ],
    },
    Theme {
        name: "Rose",
        swatch: "oklch(58% 0.2 12)",
        light: &[
            ("--primary", "oklch(58% 0.2 12)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(58% 0.2 12)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(58% 0.2 12 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(70% 0.17 12)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(70% 0.17 12)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(70% 0.17 12 / 0.55)"),
        ],
    },
    Theme {
        name: "Red",
        swatch: "oklch(55% 0.22 25)",
        light: &[
            ("--primary", "oklch(55% 0.22 25)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(55% 0.22 25)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(55% 0.22 25 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(68% 0.19 25)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(68% 0.19 25)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(68% 0.19 25 / 0.55)"),
        ],
    },
    Theme {
        name: "Teal",
        swatch: "oklch(55% 0.12 185)",
        light: &[
            ("--primary", "oklch(55% 0.12 185)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(55% 0.12 185)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(55% 0.12 185 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(72% 0.13 185)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(72% 0.13 185)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(72% 0.13 185 / 0.55)"),
        ],
    },
    Theme {
        name: "Indigo",
        swatch: "oklch(50% 0.2 275)",
        light: &[
            ("--primary", "oklch(50% 0.2 275)"),
            ("--primary-foreground", "oklch(99% 0 0)"),
            ("--accent", "oklch(50% 0.2 275)"),
            ("--accent-foreground", "oklch(99% 0 0)"),
            ("--ring", "oklch(50% 0.2 275 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(70% 0.16 275)"),
            ("--primary-foreground", "oklch(15% 0 0)"),
            ("--accent", "oklch(70% 0.16 275)"),
            ("--accent-foreground", "oklch(15% 0 0)"),
            ("--ring", "oklch(70% 0.16 275 / 0.55)"),
        ],
    },
    Theme {
        name: "Lime",
        swatch: "oklch(72% 0.18 130)",
        light: &[
            ("--primary", "oklch(72% 0.18 130)"),
            ("--primary-foreground", "oklch(20% 0.04 130)"),
            ("--accent", "oklch(72% 0.18 130)"),
            ("--accent-foreground", "oklch(20% 0.04 130)"),
            ("--ring", "oklch(72% 0.18 130 / 0.5)"),
        ],
        dark: &[
            ("--primary", "oklch(80% 0.18 130)"),
            ("--primary-foreground", "oklch(18% 0.04 130)"),
            ("--accent", "oklch(80% 0.18 130)"),
            ("--accent-foreground", "oklch(18% 0.04 130)"),
            ("--ring", "oklch(80% 0.18 130 / 0.55)"),
        ],
    },
];

fn block(selector: &str, vars: &[(&str, &str)]) -> String {
    let body = vars
        .iter()
        .map(|(k, v)| format!("  {k}: {v};"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{selector} {{\n{body}\n}}")
}

/// `themesStylesheet()`: full token sets keyed by `data-theme`, `default` excluded
/// because it already lives in `globals.css`.
pub fn themes_stylesheet() -> String {
    THEMES
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 0)
        .map(|(i, t)| {
            let id = ThemeId::ALL[i].id();
            format!(
                "{}\n{}",
                block(&format!("[data-theme=\"{id}\"]"), &t.light_vars()),
                block(&format!(".dark[data-theme=\"{id}\"]"), &t.dark_vars()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// [`themes_stylesheet`], computed once. The stylesheet is a few KB of
/// constants, so the app shell can hold a `&'static str` rather than rebuilding
/// it per render.
pub fn themes_stylesheet_cached() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(themes_stylesheet)
}

/// `themeExportCss(id)`: copyable `:root` / `.dark` blocks.
pub fn theme_export_css(id: ThemeId) -> String {
    let t = id.theme();
    format!(
        "{}\n\n{}",
        block(":root", &t.light_vars()),
        block(".dark", &t.dark_vars()),
    )
}

/// `THEME_LIST` — id / name / swatch for the picker.
pub fn theme_list() -> Vec<(ThemeId, &'static str, &'static str)> {
    ThemeId::ALL
        .into_iter()
        .map(|id| (id, id.theme().name, id.theme().swatch))
        .collect()
}

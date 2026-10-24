//! Dark mode and the color-theme picker.
//!
//! Two independent pieces of state, exactly as beUI splits them:
//!
//! - **Dark mode** is `next-themes` with `attribute="class"`,
//!   `defaultTheme="dark"`, `enableSystem`, and `enableColorScheme={false}`
//!   (`components/app/chrome/theme-provider.tsx`). That means: a `dark` class on
//!   `<html>`, `localStorage["theme"]` holding `light` / `dark` / `system`, and
//!   `system` resolving through `prefers-color-scheme`.
//! - **Color theme** is `PreferencesProvider`'s `colorTheme`: a `data-theme`
//!   attribute on `<html>`, **not persisted** ("Intentionally NOT persisted —
//!   resets to defaults on refresh"), and *deleted* rather than set to
//!   `default`, because `globals.css` already carries the default tokens.

use beui_core::hooks::use_media_query;
use beui_core::platform;
use dioxus::prelude::*;

use crate::themes::ThemeId;

/// The stored theme preference, i.e. `next-themes`' three values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    Light,
    #[default]
    Dark,
    System,
}

impl ColorMode {
    pub const fn id(self) -> &'static str {
        match self {
            ColorMode::Light => "light",
            ColorMode::Dark => "dark",
            ColorMode::System => "system",
        }
    }

    pub fn from_id(id: &str) -> Option<ColorMode> {
        match id {
            "light" => Some(ColorMode::Light),
            "dark" => Some(ColorMode::Dark),
            "system" => Some(ColorMode::System),
            _ => None,
        }
    }
}

/// `next-themes`' storage key.
const STORAGE_KEY: &str = "theme";

/// Everything a theme toggle needs.
///
/// The fields are signals so a component re-renders when the value changes; the
/// methods of the same name read them. (`dark.is_dark` is the signal,
/// `dark.is_dark()` is the value — the same pairing the rest of the library
/// uses, minus the deref gymnastics the call needs.)
#[derive(Clone, Copy)]
pub struct DarkMode {
    mode: Signal<ColorMode>,
    is_dark: Signal<bool>,
    system_dark: Signal<bool>,
}

impl DarkMode {
    /// The stored preference.
    pub fn mode(&self) -> ColorMode {
        (self.mode)()
    }

    /// What the preference resolves to right now.
    pub fn is_dark(&self) -> bool {
        (self.is_dark)()
    }

    /// The system's preference, tracked while `mode` is `system`.
    pub fn system_dark(&self) -> bool {
        (self.system_dark)()
    }

    /// The signal behind [`DarkMode::mode`], for components that need to react
    /// to it without re-rendering on every read.
    pub fn mode_signal(&self) -> Signal<ColorMode> {
        self.mode
    }

    pub fn set(&self, mode: ColorMode) {
        let mut signal = self.mode;
        signal.set(mode);
    }

    pub fn toggle(&self) {
        let next = if self.is_dark() {
            ColorMode::Light
        } else {
            ColorMode::Dark
        };
        self.set(next);
    }
}

/// Install the dark-mode state: reads `localStorage`, follows the system
/// preference, and keeps the `dark` class on `<html>` in sync.
///
/// Call once at the root of the app; descendants read it with [`use_dark_mode`].
pub fn use_dark_mode_provider() -> DarkMode {
    let mode = use_signal(|| {
        platform::storage_get(STORAGE_KEY)
            .and_then(|value| ColorMode::from_id(&value))
            // `next-themes` fell back to `system` when `enableSystem` was on and
            // nothing was stored; beUI's `defaultTheme="dark"` wins outright.
            .unwrap_or_default()
    });
    let system_dark = use_media_query("(prefers-color-scheme: dark)");
    let mut is_dark = use_signal(|| false);

    use_effect(move || {
        let resolved = match mode() {
            ColorMode::Light => false,
            ColorMode::Dark => true,
            ColorMode::System => system_dark(),
        };
        is_dark.set(resolved);
        // `attribute="class"` adds the resolved class and removes the other, so
        // `:root` rules keep working in both directions.
        platform::html_class("dark", resolved);
        platform::html_class("light", !resolved);
        platform::storage_set(STORAGE_KEY, Some(mode().id()));
    });

    let dark_mode = DarkMode {
        mode,
        is_dark,
        system_dark,
    };
    use_context_provider(|| dark_mode);
    dark_mode
}

/// The dark-mode state installed by [`use_dark_mode_provider`].
pub fn use_dark_mode() -> DarkMode {
    use_context::<DarkMode>()
}

/// Everything the theme picker needs.
#[derive(Clone, Copy)]
pub struct ColorTheme {
    theme: Signal<ThemeId>,
    /// All eleven palettes, keyed by `data-theme`; inject once.
    pub stylesheet: &'static str,
}

impl ColorTheme {
    /// The selected palette.
    pub fn theme(&self) -> ThemeId {
        (self.theme)()
    }

    /// Whether `id` is the selected palette.
    pub fn is(&self, id: ThemeId) -> bool {
        self.theme() == id
    }

    /// Select a palette. The default palette removes the attribute, as the
    /// source does.
    pub fn set(&self, theme: ThemeId) {
        let mut signal = self.theme;
        signal.set(theme);
    }
}

/// Install the color-theme state. Call once at the root of the app.
pub fn use_color_theme_provider() -> ColorTheme {
    let theme = use_signal(|| ThemeId::Default);

    use_effect(move || {
        let id = theme();
        // The stylesheet is injected once by the app shell; the attribute is what
        // changes.
        platform::html_attr(
            "data-theme",
            if id == ThemeId::Default {
                None
            } else {
                Some(id.id())
            },
        );
    });

    let color_theme = ColorTheme {
        theme,
        stylesheet: crate::themes::themes_stylesheet_cached(),
    };
    use_context_provider(|| color_theme);
    color_theme
}

/// The color-theme state installed by [`use_color_theme_provider`].
pub fn use_color_theme() -> ColorTheme {
    use_context::<ColorTheme>()
}

//! `motion/theme-toggle.tsx` — dark/light switch with a View Transition reveal.
//!
//! Four reveal shapes (`rectangle`, `circle`, `circle-blur`, `blinds`). The CSS
//! travels with the component and is appended to `<head>` once; the toggle
//! itself sets `data-beui-vt` on `<html>`, calls `document.startViewTransition`
//! (whose callback flips the class so the "new" snapshot is the other theme)
//! and clears the attribute when the transition finishes. Reduced motion, or a
//! browser without View Transitions, switches straight away.

use crate::action_swap::{ActionSwapAnimation, ActionSwapIcon};
use crate::icons::{Moon, Sun};
use crate::prelude::*;
use beui_core::platform;
use beui_theme::{use_dark_mode, ColorMode};

/// The reveal shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeVariant {
    #[default]
    Rectangle,
    Circle,
    CircleBlur,
    Blinds,
}

/// Where the reveal starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RectStart {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    #[default]
    BottomUp,
}

const VT_STYLE_ID: &str = "beui-theme-toggle-vt";

/// `VT_CSS`, verbatim (including the `@property` slat registration).
pub const THEME_TOGGLE_VT_CSS: &str = r#"
html[data-beui-vt="rect"]::view-transition-old(root) {
  animation: none;
  mix-blend-mode: normal;
}
html[data-beui-vt="rect"]::view-transition-new(root) {
  mix-blend-mode: normal;
  animation: beui-rect-reveal 400ms ease-out;
}
html[data-beui-vt="circle"]::view-transition-old(root),
html[data-beui-vt="circle-blur"]::view-transition-old(root) {
  animation: none;
  mix-blend-mode: normal;
}
html[data-beui-vt="circle"]::view-transition-new(root) {
  mix-blend-mode: normal;
  animation: beui-circle-reveal 700ms cubic-bezier(0.4, 0, 0.2, 1);
}
html[data-beui-vt="circle-blur"]::view-transition-new(root) {
  mix-blend-mode: normal;
  animation: beui-circle-blur-reveal 700ms cubic-bezier(0.4, 0, 0.2, 1);
}
html[data-beui-vt="blinds"]::view-transition-old(root) {
  animation: none;
  mix-blend-mode: normal;
}
@property --beui-vt-slat {
  syntax: "<length>";
  inherits: false;
  initial-value: 72px;
}
html[data-beui-vt="blinds"]::view-transition-new(root) {
  mix-blend-mode: normal;
  mask-image: linear-gradient(
    90deg,
    #000 0 var(--beui-vt-slat),
    transparent calc(var(--beui-vt-slat) + 20px)
  );
  mask-size: 72px 100%;
  mask-repeat: repeat;
  animation: beui-blinds-reveal 700ms cubic-bezier(0.16, 1, 0.3, 1);
}
@keyframes beui-rect-reveal {
  from { clip-path: var(--beui-vt-from, inset(100% 0 0 0)); }
  to   { clip-path: inset(0 0 0 0); }
}
@keyframes beui-circle-reveal {
  from { clip-path: circle(0% at var(--beui-vt-origin, 50% 100%)); }
  to   { clip-path: circle(150% at var(--beui-vt-origin, 50% 100%)); }
}
@keyframes beui-circle-blur-reveal {
  from { clip-path: circle(0% at var(--beui-vt-origin, 50% 100%)); filter: blur(8px); }
  to   { clip-path: circle(150% at var(--beui-vt-origin, 50% 100%)); filter: blur(0px); }
}
@keyframes beui-blinds-reveal {
  from { --beui-vt-slat: -20px; }
  to   { --beui-vt-slat: 72px; }
}
"#;

fn rect_from(start: RectStart) -> &'static str {
    match start {
        RectStart::TopLeft => "inset(0 100% 100% 0)",
        RectStart::TopRight => "inset(0 0 100% 100%)",
        RectStart::BottomLeft => "inset(100% 100% 0 0)",
        RectStart::BottomRight => "inset(100% 0 0 100%)",
        RectStart::Center => "inset(50% 50% 50% 50%)",
        RectStart::BottomUp => "inset(100% 0 0 0)",
    }
}

fn circle_origin(start: RectStart) -> &'static str {
    match start {
        RectStart::TopLeft => "0% 0%",
        RectStart::TopRight => "100% 0%",
        RectStart::BottomLeft => "0% 100%",
        RectStart::BottomRight => "100% 100%",
        RectStart::Center => "50% 50%",
        RectStart::BottomUp => "50% 100%",
    }
}

/// Install the View Transition stylesheet once, in `<head>`.
fn install_vt_css() {
    platform::fire(&format!(
        "(function(){{ if(document.getElementById({id})) return; const el=document.createElement('style'); el.id={id}; el.textContent={css}; document.head.appendChild(el); }})()",
        id = platform::js_string(VT_STYLE_ID),
        css = platform::js_string(THEME_TOGGLE_VT_CSS),
    ));
}

/// `useThemeToggle`: the resolved theme and a `toggle` that runs the reveal.
#[derive(Clone, Copy)]
pub struct ThemeToggleHandle {
    dark: beui_theme::DarkMode,
    variant: ThemeVariant,
    start: RectStart,
    reduce: Signal<bool>,
}

impl ThemeToggleHandle {
    /// Whether the resolved theme is dark.
    pub fn is_dark(&self) -> bool {
        self.dark.is_dark()
    }

    /// Flip the theme with the configured reveal.
    pub fn toggle(&self) {
        let next = if self.dark.is_dark() {
            ColorMode::Light
        } else {
            ColorMode::Dark
        };
        if (self.reduce)() {
            self.dark.set(next);
            return;
        }
        let dark_next = next == ColorMode::Dark;
        let (variant, from, origin) = match self.variant {
            ThemeVariant::Rectangle => ("rect", rect_from(self.start), ""),
            ThemeVariant::Blinds => ("blinds", "", ""),
            ThemeVariant::Circle => ("circle", "", circle_origin(self.start)),
            ThemeVariant::CircleBlur => ("circle-blur", "", circle_origin(self.start)),
        };
        platform::fire(&format!(
            "(function(){{ const root=document.documentElement; const dark={dark}; if(!('startViewTransition' in document)){{ return; }} \
             if({variant}==='rect'){{ root.style.setProperty('--beui-vt-from', {from}); root.dataset.beuiVt='rect'; }} \
             else if({variant}==='blinds'){{ root.dataset.beuiVt='blinds'; }} \
             else {{ root.style.setProperty('--beui-vt-origin', {origin}); root.dataset.beuiVt={variant}; }} \
             const vt=document.startViewTransition(function(){{ root.classList.toggle('dark', dark); root.classList.toggle('light', !dark); try {{ localStorage.setItem('theme', dark ? 'dark' : 'light'); }} catch(e){{}} }}); \
             vt.finished.finally(function(){{ delete root.dataset.beuiVt; }}); }})()",
            dark = if dark_next { "true" } else { "false" },
            variant = platform::js_string(variant),
            from = platform::js_string(from),
            origin = platform::js_string(origin),
        ));
        self.dark.set(next);
    }
}

/// Props for [`ThemeToggle`].
#[derive(Props, Clone, PartialEq)]
pub struct ThemeToggleProps {
    #[props(default)]
    pub variant: ThemeVariant,
    #[props(default)]
    pub start: RectStart,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub icon_class: Option<String>,
    #[props(default)]
    pub aria_label: Option<String>,
}

/// The dark/light switch.
#[component]
pub fn ThemeToggle(props: ThemeToggleProps) -> Element {
    let ThemeToggleProps {
        variant,
        start,
        class,
        icon_class,
        aria_label,
    } = props;
    let dark = use_dark_mode();
    let reduce = use_reduced_motion();

    use_effect(install_vt_css);

    let is_dark = dark.is_dark();
    let class = cn!("flex items-center justify-center", class);
    let label = aria_label.unwrap_or_else(|| {
        if is_dark {
            "Switch to light mode".to_string()
        } else {
            "Switch to dark mode".to_string()
        }
    });

    rsx! {
        button {
            r#type: "button",
            aria_label: label,
            class,
            onclick: move |_| {
                ThemeToggleHandle {
                    dark,
                    variant,
                    start,
                    reduce,
                }
                .toggle();
            },
            ActionSwapIcon {
                value: if is_dark { "dark".to_string() } else { "light".to_string() },
                animation: ActionSwapAnimation::Blur,
                class: icon_class.clone(),
                if is_dark {
                    Sun { class: icon_class.clone() }
                } else {
                    Moon { class: icon_class.clone() }
                }
            }
        }
    }
}

//! `motion/animated-badge.tsx` — the status badge.
//!
//! The badge rolls its content when the status changes: the leaving glyphs slide
//! up and out while the next slide in from below, both through a light blur, and
//! the exit outruns the entry (`SPRING_SWAP` in, a 140 ms `EASE_OUT` out). The
//! icon and the label are separate `AnimatePresence` boundaries keyed by the
//! content, so a status change rolls both.
//!
//! Difference from the source: motion animates the pill's width with `layout`
//! and counter-scales the slots (`layout="position"`). That nested projection is
//! not implemented in `beui-core::flip` (see that module's FLIP notes), so the
//! pill's width changes without animating; the rolls,
//! the pulse and every colour transition are unchanged.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::ease::ease_in_out;
use beui_core::hooks::use_reduced_motion;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};

use crate::icons::{IconAlertTriangle, IconCheck, IconCircle, IconInfo, IconLoaderCircle, IconX};
use crate::motion::{number, spring, track, tween, use_loop_phase, Ease, MotionStyle};

/// `AnimatedBadgeStatus`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimatedBadgeStatus {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
    Loading,
}

/// `AnimatedBadgeSize`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimatedBadgeSize {
    Sm,
    #[default]
    Md,
}

impl AnimatedBadgeStatus {
    const fn class(self) -> &'static str {
        match self {
            Self::Neutral => "border-border bg-card text-muted-foreground",
            Self::Info => "border-primary/30 bg-muted text-primary",
            Self::Success => {
                "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
            }
            Self::Warning => {
                "border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400"
            }
            Self::Danger => "border-destructive/30 bg-destructive/10 text-destructive",
            Self::Loading => "border-primary/30 bg-muted text-primary",
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::Loading => "loading",
        }
    }
}

impl AnimatedBadgeSize {
    const fn class(self) -> &'static str {
        match self {
            Self::Sm => "h-6 gap-1.5 px-2 text-[11px]",
            Self::Md => "h-8 gap-2 px-3 text-xs",
        }
    }

    const fn icon_class(self) -> &'static str {
        match self {
            Self::Sm => "h-3 w-3",
            Self::Md => "h-3.5 w-3.5",
        }
    }
}

/// The default glyph for a status.
#[component]
fn StatusIcon(status: AnimatedBadgeStatus, size: AnimatedBadgeSize) -> Element {
    let class = size.icon_class();
    match status {
        AnimatedBadgeStatus::Neutral => rsx! { IconCircle { class } },
        AnimatedBadgeStatus::Info => rsx! { IconInfo { class } },
        AnimatedBadgeStatus::Success => rsx! { IconCheck { class } },
        AnimatedBadgeStatus::Warning => rsx! { IconAlertTriangle { class } },
        AnimatedBadgeStatus::Danger => rsx! { IconX { class } },
        AnimatedBadgeStatus::Loading => rsx! { IconLoaderCircle { class } },
    }
}

/// The loading glyph spins; every other glyph is still.
#[component]
fn BadgeIcon(status: AnimatedBadgeStatus, size: AnimatedBadgeSize) -> Element {
    let reduce = use_reduced_motion()();
    // The hook is unconditional: the component can outlive a reduced-motion
    // change, so the loop always exists and is simply not applied.
    let phase = use_loop_phase(1000);
    let spin = status == AnimatedBadgeStatus::Loading && !reduce;
    let rotate = if spin { 360.0 * number(&phase) } else { 0.0 };
    rsx! {
        span { class: "inline-flex", style: "transform:rotate({rotate}deg)",
            StatusIcon { status, size }
        }
    }
}

const ROLL_BLUR: f32 = 3.0;

fn icon_style(opacity: f32, y: f32, blur: f32) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        y,
        ..Default::default()
    };
    style.add_css_property("filter", format!("blur({blur}px)"));
    style
}

fn icon_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        );
    }
    PresenceConfig::with_transitions(
        icon_style(0.0, 12.0, ROLL_BLUR),
        icon_style(1.0, 0.0, 0.0),
        icon_style(0.0, -12.0, ROLL_BLUR),
        spring(beui_core::ease::SPRING_SWAP),
        tween(140, Ease::Out),
    )
}

fn text_style(opacity: f32, percent: f32, blur: f32) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        ..Default::default()
    };
    style.add_css_property("filter", format!("blur({blur}px)"));
    // The source rolls by percentage; `MotionStyle::y` is pixels, so the
    // translate goes through the property map where a percentage interpolates.
    style.add_css_property("transform", format!("translateY({percent}%)"));
    style
}

fn text_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        );
    }
    PresenceConfig::with_transitions(
        text_style(0.0, 90.0, ROLL_BLUR),
        text_style(1.0, 0.0, 0.0),
        text_style(0.0, -90.0, ROLL_BLUR),
        spring(beui_core::ease::SPRING_SWAP),
        tween(140, Ease::Out),
    )
}

#[component]
pub fn AnimatedBadge(
    #[props(default)] status: AnimatedBadgeStatus,
    #[props(default)] size: AnimatedBadgeSize,
    #[props(default)] icon: Option<Element>,
    #[props(default = true)] show_icon: bool,
    #[props(default)] pulse: Option<bool>,
    #[props(default)] content_key: Option<String>,
    #[props(default)] aria_live: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let pulse = pulse.unwrap_or(status == AnimatedBadgeStatus::Loading);
    let key = content_key.unwrap_or_else(|| status.key().to_string());
    let has_icon = show_icon;
    let icons: Vec<String> = if has_icon {
        vec![status.key().to_string()]
    } else {
        Vec::new()
    };
    let labels: Vec<String> = vec![key.clone()];

    rsx! {
        span {
            "aria-live": aria_live,
            class: cn!(
                "relative inline-flex shrink-0 items-center overflow-hidden whitespace-nowrap rounded-full border font-medium tabular-nums",
                "transition-colors duration-300",
                status.class(),
                size.class(),
                class,
            ),
            if pulse && !reduce {
                Pulse {}
            }
            if has_icon {
                span { class: "relative z-10 inline-grid shrink-0 place-items-center overflow-hidden",
                    AnimatePresence { mode: PresenceMode::PopLayout, initial: false,
                        for status_key in icons {
                            BadgeIconSlot { key: "{status_key}", status, size, icon: icon.clone() }
                        }
                    }
                }
            }
            span { class: "relative z-10 inline-flex", style: "clip-path:inset(0 -999px)",
                AnimatePresence { mode: PresenceMode::PopLayout, initial: false,
                    for label_key in labels {
                        BadgeLabelSlot { key: "{label_key}", children: children.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn Pulse() -> Element {
    let phase = use_loop_phase(1600);
    let opacity = track(&[0.06, 0.18, 0.06], number(&phase), ease_in_out);
    rsx! {
        span {
            "aria-hidden": "true",
            class: "absolute inset-0 rounded-full bg-current",
            style: "opacity:{opacity}",
        }
    }
}

#[component]
fn BadgeIconSlot(
    status: AnimatedBadgeStatus,
    size: AnimatedBadgeSize,
    icon: Option<Element>,
) -> Element {
    let reduce = use_reduced_motion()();
    let gate = use_gate();
    let style = use_presence_style(icon_config(reduce)).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        span {
            "aria-hidden": "true",
            "data-badge-icon": "true",
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "col-start-1 row-start-1 inline-flex items-center justify-center",
            if let Some(icon) = icon {
                {icon}
            } else {
                BadgeIcon { status, size }
            }
        }
    }
}

#[component]
fn BadgeLabelSlot(children: Element) -> Element {
    let reduce = use_reduced_motion()();
    let gate = use_gate();
    let style = use_presence_style(text_config(reduce)).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        span {
            "data-badge-label": "true",
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "inline-block",
            {children}
        }
    }
}

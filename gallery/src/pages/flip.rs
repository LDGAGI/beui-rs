//! `layout` and `layoutId`, on screen.
//!
//! Two panels, the two shapes beUI uses:
//!
//! - `SharedLayoutPanel` is the tabs indicator: the indicator is rendered
//!   *inside* the active trigger, so selecting another tab unmounts it in one
//!   button and mounts it in another. Nothing is animated in CSS — the new node
//!   is pinned to where the old one was last seen and springs to its own box,
//!   which is what `layoutId` means.
//! - `LayoutPanel` is plain `layout`: one element whose own box changes. Its
//!   layout box jumps immediately and its visual box follows.

use beui::prelude::*;
use beui_core::ease::tween_ease_out;
use beui_core::flip::{use_flip, FlipMode, FlipOptions};
use dioxus_motion::prelude::{AnimationConfig, Spring};

/// `tabs.tsx`'s indicator spring, copied verbatim. A per-component spring, not
/// one of the shared `lib/ease.ts` tokens.
const SPRING_INDICATOR: Spring = Spring {
    stiffness: 245.0,
    damping: 36.0,
    mass: 1.2,
};

const ENTER_MS: u64 = 300;

const TABS: [(&str, &str); 3] = [
    ("overview", "Overview"),
    ("activity", "Activity log"),
    ("settings", "Settings"),
];

#[component]
pub fn Flip() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Foundations"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Layout and shared layout"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A layout change is not a transition: the box jumps in the DOM and a transform that starts as the inverse of the jump decays to nothing, so the layout is already settled while the eye still sees the element travel."
                }
            }
            SharedLayoutPanel {}
            LayoutPanel {}
        }
    }
}

#[component]
fn Panel(title: String, note: String, children: Element) -> Element {
    rsx! {
        section { class: "flex flex-col gap-4",
            div { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold tracking-tight", "{title}" }
                p { class: "max-w-3xl text-sm text-muted-foreground", "{note}" }
            }
            {children}
        }
    }
}

#[component]
fn Probe(name: &'static str, value: String) -> Element {
    rsx! {
        span {
            "data-probe": "{name}",
            "data-probe-value": "{value}",
            class: "inline-flex items-center gap-1 rounded-full border border-border bg-card px-2.5 py-1 font-mono text-[11px] text-muted-foreground",
            "{name} = {value}"
        }
    }
}

// --- layoutId: one shape, handed between elements ---------------------------

#[component]
fn SharedLayoutPanel() -> Element {
    let selected = use_signal(|| "overview");
    let mut set_selected = selected;

    rsx! {
        Panel {
            title: "layoutId · the tabs indicator",
            note: "The indicator lives inside the active trigger, so switching tabs is an unmount in one button and a mount in another. The new node starts from the box the old one was last measured at — the layout box is already the new tab's, only the transform is behind.",
            div { class: "flex flex-col gap-3",
                div {
                    "data-flip-tabs": "true",
                    class: "relative inline-flex items-center rounded-full border border-border bg-card p-1",
                    for (value, label) in TABS {
                        button {
                            key: "{value}",
                            "data-flip-tab": "{value}",
                            r#type: "button",
                            role: "tab",
                            "aria-selected": if selected() == value { "true" } else { "false" },
                            class: cn!(
                                "relative isolate rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
                                if selected() == value { "text-primary-foreground" } else { "text-muted-foreground" },
                            ),
                            onclick: move |_| set_selected.set(value),
                            if selected() == value {
                                Indicator { value }
                            }
                            span { class: "relative z-10", "{label}" }
                        }
                    }
                }
                Probe { name: "tab", value: selected().to_string() }
            }
        }
    }
}

/// The indicator itself. A component so that each tab gets its *own* instance —
/// which is what makes this the shared-layout case rather than a plain `layout`
/// on a persistent element: the new instance has no memory of the old box, and
/// has to read it out of the registry.
#[component]
fn Indicator(value: &'static str) -> Element {
    let flip = use_flip(
        value,
        FlipOptions::default()
            .layout_id("flip-demo-indicator")
            .transition(AnimationConfig::spring(SPRING_INDICATOR))
            .border_radius(9999.0),
    );

    rsx! {
        span {
            "data-flip-indicator": "true",
            "data-beui-flip": flip.scope(),
            "aria-hidden": "true",
            class: "absolute inset-0 rounded-full bg-primary",
            style: flip.style_with(""),
        }
    }
}

// --- layout: one element whose own box changes ------------------------------

#[component]
fn LayoutPanel() -> Element {
    let expanded = use_signal(|| false);
    let mut toggle = expanded;
    let flip = use_flip(
        expanded(),
        FlipOptions::default()
            .transition(tween_ease_out(ENTER_MS))
            .mode(FlipMode::Both),
    );

    rsx! {
        Panel {
            title: "layout · a box that resizes",
            note: "The box's layout height is the new one from the first frame; the visual height starts where it was and follows over the declared 300 ms. Empty on purpose — a scaled container distorts its children, which motion fixes with nested projection and this port does not have yet.",
            div { class: "flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    button {
                        "data-flip-size-toggle": "true",
                        r#type: "button",
                        class: "press h-9 rounded-lg border border-border px-3 text-sm",
                        onclick: move |_| {
                            let next = !expanded();
                            toggle.set(next);
                        },
                        if expanded() { "Shrink" } else { "Grow" }
                    }
                    Probe { name: "expanded", value: expanded().to_string() }
                }
                div { class: "w-full", style: "height: 320px",
                    div {
                        "data-flip-box": "true",
                        "data-beui-flip": flip.scope(),
                        class: cn!(
                            "w-full rounded-xl bg-brand-accent",
                            if expanded() { "h-72" } else { "h-20" },
                        ),
                        style: flip.style_with(""),
                    }
                }
            }
        }
    }
}

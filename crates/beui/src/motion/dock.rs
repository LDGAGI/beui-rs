//! `components/motion/dock.tsx` — a row of app icons with one travelling pill.
//!
//! The active pill is a `layoutId` shape shared by every item, so activating
//! another item is an unmount in one button and a mount in another and the new
//! node starts from the box the old one was measured at — `beui-core::flip`.
//! There is no magnification or drag in the source; the dock is a static row.

use dioxus::prelude::*;
use dioxus_motion::prelude::AnimationConfig;

use beui_core::cn;
use beui_core::ease::SPRING_LAYOUT;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::platform;
use beui_core::use_reduced_motion;

#[derive(Clone)]
struct DockContext {
    size: f64,
    pill_layout_id: String,
}

/// `Dock`.
#[component]
pub fn Dock(
    #[props(default = None)] class: Option<String>,
    /// Size of each item in px.
    #[props(default = 44.0)]
    size: f64,
    children: Element,
) -> Element {
    let pill_layout_id = use_hook(|| format!("beui-dock-{}", platform::next_scope_id()));
    use_context_provider(|| DockContext {
        size,
        pill_layout_id,
    });

    rsx! {
        div {
            class: cn!(
                "inline-flex h-auto items-end gap-1.5 rounded-2xl border border-border bg-card/80 px-2 py-1 shadow-2xl backdrop-blur-xl",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `DockItem`.
#[component]
pub fn DockItem(
    #[props(default = None)] class: Option<String>,
    /// When set, the item renders as a `<button>`.
    #[props(default = None)]
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default = false)] active: bool,
    #[props(default = None)] aria_label: Option<String>,
    children: Element,
) -> Element {
    let dock = use_context::<DockContext>();
    let size = dock.size;
    let shared_style = format!("width:{size}px;height:{size}px");
    let shared_class = cn!(
        "relative flex shrink-0 items-center justify-center rounded-full text-foreground",
        class.as_deref().unwrap_or(""),
    );
    let pill = active.then(|| dock.pill_layout_id.clone());

    if let Some(handler) = onclick {
        return rsx! {
            button {
                r#type: "button",
                "aria-label": aria_label.as_deref().unwrap_or("Dock item"),
                "aria-pressed": active,
                style: shared_style,
                class: cn!(
                    shared_class,
                    "cursor-pointer border-0 bg-transparent p-0 outline-none",
                    "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                ),
                onclick: move |event| handler.call(event),
                if let Some(layout_id) = pill.clone() {
                    DockPill { layout_id, state: active }
                }
                {children}
            }
        };
    }

    rsx! {
        div {
            style: shared_style,
            class: shared_class,
            if let Some(layout_id) = pill {
                DockPill { layout_id, state: active }
            }
            {children}
        }
    }
}

/// The travelling pill. Rendered inside whichever item is active.
#[component]
fn DockPill(layout_id: String, state: bool) -> Element {
    let reduce = use_reduced_motion()();
    let flip = use_flip(
        state,
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(if reduce {
                beui_core::ease::tween_ease_out(1)
            } else {
                AnimationConfig::spring(SPRING_LAYOUT)
            })
            .border_radius(12.0),
    );

    rsx! {
        span {
            "data-beui-flip": flip.scope(),
            "aria-hidden": "true",
            style: flip.style_with(""),
            class: "absolute inset-0.5 -z-10 rounded-xl bg-muted/60",
        }
    }
}

/// `DockSeparator`.
#[component]
pub fn DockSeparator(#[props(default = None)] class: Option<String>) -> Element {
    rsx! {
        span {
            "aria-hidden": "true",
            class: cn!("mx-1 h-6 w-px self-center bg-border", class.as_deref().unwrap_or("")),
        }
    }
}

//! `components/motion/breadcrumb.tsx` — a path that folds its middle away.
//!
//! `BreadcrumbList` collapses everything between the root and the last
//! `maxItems - 2` slots into an ellipsis. React can slice `Children`; a Dioxus
//! `Element` is opaque, so the same slice is taken off the fragment's child
//! `VNode`s (they keep the caller's keys), and the overflow item is rebuilt
//! around them. Each item is a `popLayout` presence child, so entering and
//! leaving routes animate and the siblings move at once.
//!
//! Deviations from the source:
//!
//! - `BreadcrumbItem`'s width is measured with `client_rect` rather than a
//!   `ResizeObserver`, and the exit width is written directly as `min-width`;
//! - the ellipsis' hover-out is a fixed 160 ms close delay rather than a check
//!   for focus inside the panel;
//! - `BreadcrumbLink` has no `render` prop (no `cloneElement`).

use dioxus::prelude::*;
use dioxus_core::{DynamicNode, VNode};

use crate::icons::{ChevronRightIcon, EllipsisIcon};
use crate::motion::MorphPopoverSide;
use crate::motion::{MorphPopover, MorphPopoverAlign, MorphPopoverContent, MorphPopoverTrigger};
use beui_core::ease::tween_ease_out;
use beui_core::hooks::use_hover_capable;
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};
use beui_core::{cn, use_reduced_motion};
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

/// Read the keyed child nodes out of an `Element`.
///
/// A keyed `for` loop (or a run of keyed components) compiles to a one-root
/// fragment; anything else is a single node. Either way the caller's keys are
/// preserved, which is what `AnimatePresence` needs.
fn element_nodes(children: Element) -> Vec<VNode> {
    let Ok(vnode) = children else {
        return Vec::new();
    };
    if vnode.template.roots.len() == 1 {
        if let Some(DynamicNode::Fragment(nodes)) = vnode.dynamic_root(0) {
            return nodes.to_vec();
        }
    }
    vec![vnode]
}

/// The `Breadcrumb` navigation landmark.
#[component]
pub fn Breadcrumb(#[props(default = None)] class: Option<String>, children: Element) -> Element {
    rsx! {
        nav {
            "aria-label": "Breadcrumb",
            class: cn!("min-w-0", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `BreadcrumbList` — collapses the middle of a long path.
#[component]
pub fn BreadcrumbList(
    /// Maximum visible slots, including the ellipsis. Minimum 3; 0 disables.
    #[props(default = 4)]
    max_items: usize,
    #[props(default = "Show hidden paths")] overflow_label: &'static str,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let items = element_nodes(children);
    let collapse = max_items != 0 && items.len() > max_items.max(3);
    let visible: Vec<VNode> = if collapse {
        let limit = max_items.max(3);
        let tail = limit - 2;
        let mut visible = Vec::new();
        if let Some(first) = items.first() {
            visible.push(first.clone());
        }
        let hidden: Vec<VNode> = items[1..items.len() - tail].to_vec();
        visible.push(overflow_item(hidden, overflow_label));
        visible.extend(items[items.len() - tail..].iter().cloned());
        visible
    } else {
        items
    };

    // The for-loop body has to *be* the keyed component; a block that returns an
    // `Element` gets wrapped in a fresh unkeyed fragment and the key is lost.
    let keyed: Vec<(String, VNode)> = visible
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            let key = node
                .key
                .clone()
                .unwrap_or_else(|| format!("breadcrumb-item-{index}"));
            (key, node)
        })
        .collect();

    rsx! {
        ol {
            class: cn!("relative flex flex-wrap items-center gap-x-1 gap-y-1 text-sm", class.as_deref().unwrap_or("")),
            AnimatePresence {
                initial: false,
                mode: PresenceMode::PopLayout,
                for (key, node) in keyed {
                    BreadcrumbSlot { key: "{key}", node }
                }
            }
        }
    }
}

/// Renders one caller VNode as a presence child. The key lives on this
/// component, which is what keeps `AnimatePresence` able to retain it.
#[component]
fn BreadcrumbSlot(node: VNode) -> Element {
    rsx! {
        {node}
    }
}

fn overflow_item(hidden: Vec<VNode>, label: &'static str) -> VNode {
    let children: Vec<Element> = hidden.into_iter().map(Ok).collect();
    let key = "breadcrumb-overflow";
    rsx! {
        BreadcrumbItem { key: "{key}",
            BreadcrumbSeparator {}
            BreadcrumbEllipsis { label, for child in children { {child} } }
        }
    }
    .expect("breadcrumb overflow item renders")
}

fn item_config(reduce: bool) -> PresenceConfig {
    let shift = if reduce { 0.0 } else { 6.0 };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, y: shift },
        motion_style! { opacity: 1.0, y: 0.0 },
        motion_style! { opacity: 0.0, y: shift },
        tween_ease_out(200),
        tween_ease_out(200),
    )
}

/// `BreadcrumbItem` — one presence child, with its exit width pinned.
#[component]
pub fn BreadcrumbItem(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] style: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let gate = use_gate();
    let handle = use_presence_style(item_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    let node: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);
    let measured = use_signal(|| 0.0_f64);
    use_effect(move || {
        let Some(mounted) = node() else {
            return;
        };
        let mut width = measured;
        spawn(async move {
            if let Some(rect) = platform::client_rect(&mounted).await {
                width.set(rect.width);
            }
        });
    });

    let min_width = if gate.is_present() {
        String::new()
    } else {
        format!("min-width:{}px;", (measured)())
    };
    let base = format!("{min_width}{}", style.as_deref().unwrap_or(""));

    rsx! {
        li {
            onmounted: move |event| {
                let mut slot = node;
                slot.set(Some(event.data()));
            },
            inert: gate.inert_attr(),
            "aria-hidden": if gate.is_present() { None } else { Some("true") },
            style: gate.style_with(&format!("{base}{css}")),
            class: cn!("relative inline-flex min-w-0 max-w-full items-center gap-1", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `BreadcrumbLink`.
#[component]
pub fn BreadcrumbLink(
    #[props(default = None)] href: Option<String>,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    rsx! {
        a {
            href: href.as_deref().unwrap_or("#"),
            class: cn!(
                "inline-flex min-h-8 min-w-0 items-center gap-1.5 rounded-md px-2 font-medium text-muted-foreground transition-colors duration-150 hover:bg-muted/60 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 [&>svg]:size-3.5 [&>svg]:shrink-0",
                class.as_deref().unwrap_or(""),
            ),
            onclick: move |event| {
                if let Some(callback) = onclick {
                    callback.call(event);
                }
            },
            {children}
        }
    }
}

/// `BreadcrumbPage`.
#[component]
pub fn BreadcrumbPage(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] tabindex: Option<i32>,
    children: Element,
) -> Element {
    rsx! {
        span {
            "aria-current": "page",
            tabindex: tabindex,
            class: cn!(
                "relative isolate inline-flex min-h-8 min-w-0 items-center gap-1.5 rounded-md px-2 font-medium text-foreground [overflow-wrap:anywhere] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring [&>svg]:size-3.5 [&>svg]:shrink-0",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `BreadcrumbSeparator`.
#[component]
pub fn BreadcrumbSeparator(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        span {
            "aria-hidden": "true",
            "data-breadcrumb-separator": "",
            class: cn!(
                "inline-flex shrink-0 items-center text-muted-foreground/50 [&>svg]:size-3.5 rtl:rotate-180",
                class.as_deref().unwrap_or(""),
            ),
            crate::motion::Slot { default: rsx! { ChevronRightIcon {} }, {children} }
        }
    }
}

/// `BreadcrumbEllipsis` — the hover/click disclosure of the hidden ancestors.
#[component]
pub fn BreadcrumbEllipsis(
    #[props(default = "Show hidden paths")] label: &'static str,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let open = use_signal(|| false);
    let placement = use_signal(|| {
        (
            MorphPopoverAlign::Start,
            MorphPopoverSide::Bottom,
            224.0_f64,
        )
    });
    let can_hover = use_hover_capable()();
    let mut set_open = open;
    let mut set_placement = placement;
    let close_token = use_signal(|| 0u32);
    let mut open_signal = open;

    // Placement: prefer the requested side, flip when there is more room, and
    // clamp the width to the space beside the trigger.
    use_effect(move || {
        if !open() {
            return;
        }
        let script = "(() => { const t = document.querySelector('[data-breadcrumb-ellipsis] [data-beui-morph-trigger]'); \
             if (!t) { dioxus.send('start|bottom|224'); return; } const r = t.getBoundingClientRect(); \
             const right = window.innerWidth - r.left - 8; const left = r.right - 8; \
             const align = (right < 224 && left > right) ? 'end' : 'start'; \
             const below = window.innerHeight - r.bottom; \
             const side = (below < 280 && r.top > below) ? 'top' : 'bottom'; \
             const width = Math.max(32, Math.min(224, align === 'start' ? right : left)); \
             dioxus.send(align + '|' + side + '|' + width); })()";
        let Some(mut eval) = platform::eval(script) else {
            return;
        };
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                let mut parts = message.split('|');
                let align = match parts.next() {
                    Some("end") => MorphPopoverAlign::End,
                    _ => MorphPopoverAlign::Start,
                };
                let side = match parts.next() {
                    Some("top") => MorphPopoverSide::Top,
                    _ => MorphPopoverSide::Bottom,
                };
                let width = parts
                    .next()
                    .and_then(|w| w.parse::<f64>().ok())
                    .unwrap_or(224.0);
                if *set_placement.peek() != (align, side, width) {
                    set_placement.set((align, side, width));
                }
            }
        });
    });

    let cancel_close = move |_| {
        let mut token = close_token;
        token += 1;
    };
    let leave = move |_| {
        let mut token = close_token;
        token += 1;
        let current = *token.peek();
        spawn(async move {
            if Time::delay(Duration::from_millis(160)).await.is_ok() && *token.peek() == current {
                set_open.set(false);
            }
        });
    };
    let enter = move |event: Event<PointerData>| {
        cancel_close(());
        if can_hover && event.pointer_type() == "mouse" {
            set_open.set(true);
        }
    };

    let (align, side, width) = (placement)();

    rsx! {
        span {
            "data-breadcrumb-ellipsis": "",
            onpointerenter: enter,
            onpointerleave: leave,
            MorphPopover { open: Some(open()), on_open_change: move |next| set_open.set(next),
                MorphPopoverTrigger {
                    class: "inline-flex size-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    aria_label: label,
                    onkeydown: move |event: KeyboardEvent| {
                        if event.key() == Key::ArrowDown {
                            event.prevent_default();
                            open_signal.set(true);
                        }
                    },
                    EllipsisIcon { class: "size-4" }
                }
                MorphPopoverContent { align, side, radius: 10.0, side_offset: 6.0, class: "p-1.5",
                    ol {
                        style: format!("width:{}px;", (width - 14.0).max(32.0)),
                        class: "flex max-h-64 flex-col gap-0.5 overflow-y-auto [&>li]:w-full [&_a]:w-full [&_a]:py-1 [&_a]:[overflow-wrap:anywhere] [&_[data-breadcrumb-separator]]:hidden",
                        onpointerenter: move |_| cancel_close(()),
                        onpointerleave: leave,
                        {children}
                    }
                }
            }
        }
    }
}

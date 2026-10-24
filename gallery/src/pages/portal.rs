//! Portal, on screen: the three things an overlay has to escape.
//!
//! - `EscapePanel` puts the same element in two places — one rendered inline
//!   inside a stage that has a `transform` and a narrow `overflow: hidden`, one
//!   portaled to `<body>`. The inline one is positioned against the stage (a
//!   transform makes it a containing block for `position: fixed`) and clipped by
//!   it; the portaled one is positioned against the viewport and whole. That
//!   difference is the whole reason beUI portals.
//! - `ZOrderPanel` renders two portaled layers that overlap, swaps their order in
//!   the tree, and leaves the stacking to their `z-index` classes.
//! - `ModalPortalPanel` runs `useModalScope` over a portaled panel, which is the
//!   case the source's rule — "every child of `body` except the root is inert" —
//!   was written for: the app's own root goes inert too.

use beui::prelude::*;
use beui_core::hooks::{use_dismiss, use_modal_scope, DismissOptions};
use beui_core::portal::{use_portal, Portal as PortalHandle};

#[component]
pub fn Portal() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Foundations"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Portal and z-order"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "An overlay has to escape its ancestors: a transform above it becomes its containing block for position: fixed, overflow above it clips it, and a z-index above it caps how high it can paint. beUI portals every overlay to body; so does this."
                }
            }
            EscapePanel {}
            ZOrderPanel {}
            ModalPortalPanel {}
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

const INLINE_STYLE: &str = "position: fixed; left: 40px; top: 40px; width: 220px; z-index: 50";

#[component]
fn EscapePanel() -> Element {
    let portal = use_portal();

    rsx! {
        Panel {
            title: "escaping an ancestor",
            note: "Both boxes carry `position: fixed; left: 40px; top: 40px`. The stage has a transform and a 160px width, so the inline box is positioned against the stage and clipped by it; the portaled box is a child of body, positioned against the viewport and whole.",
            div {
                "data-portal-stage": "true",
                style: "transform: translateY(0); overflow: hidden; width: 160px",
                class: "relative rounded-xl border border-border bg-card p-3",
                p { class: "text-xs text-muted-foreground", "The stage" }
                // The control: the same element, no portal.
                div {
                    "data-portal-inline": "true",
                    style: INLINE_STYLE,
                    class: "rounded-lg border border-warning bg-warning/20 p-2 text-xs",
                    "inline"
                }
                // The portaled one.
                div {
                    "data-portal-fixed": "true",
                    "data-beui-portal": portal.scope(),
                    style: INLINE_STYLE,
                    class: "rounded-lg border border-primary bg-card p-2 text-xs shadow-2xl",
                    onmounted: move |_| portal.attach(),
                    "portaled to body"
                }
            }
        }
    }
}

// --- z-order: two portaled layers, rendered in either order -------------------

#[component]
fn ZOrderPanel() -> Element {
    let flipped = use_signal(|| false);

    rsx! {
        Panel {
            title: "stacking without a shared scale",
            note: "Two portaled layers at the same place, z-50 and z-[9999]. Whichever order they are rendered in, a hit test at their centre lands on the higher one — which is what makes the source's per-component z classes safe to copy as they are.",
            div { class: "flex flex-col gap-3",
                button {
                    "data-portal-z-flip": "true",
                    r#type: "button",
                    class: "press h-9 w-fit rounded-lg border border-border px-3 text-sm",
                    onclick: move |_| {
                        let next = !flipped();
                        let mut signal = flipped;
                        signal.set(next);
                    },
                    if flipped() { "Render the higher one first" } else { "Render the lower one first" }
                }
                if flipped() {
                    ZLayer { high: true, first: true }
                    ZLayer { high: false, first: false }
                } else {
                    ZLayer { high: false, first: true }
                    ZLayer { high: true, first: false }
                }
            }
        }
    }
}

/// One layer of the z-order demo. Each owns its own portal, which is how a
/// component uses this: the layer renders itself, then moves itself.
#[component]
fn ZLayer(high: bool, first: bool) -> Element {
    let portal = use_portal();
    let style = "position: fixed; left: 320px; top: 40px; width: 160px; height: 80px";
    let (z, class, label) = if high {
        ("9999", "border-warning bg-warning shadow-2xl", "z-[9999]")
    } else {
        (
            "50",
            "border-primary bg-primary text-primary-foreground",
            "z-50",
        )
    };

    rsx! {
        div {
            "data-portal-z": if high { "high" } else { "low" },
            "data-beui-portal": portal.scope(),
            "data-portal-z-order": if first { "first" } else { "second" },
            style: "{style}; z-index: {z}",
            class: cn!("grid place-items-center rounded-lg border text-sm", class),
            onmounted: move |_| portal.attach(),
            "{label}"
        }
    }
}

// --- a modal scope over a portaled panel -------------------------------------

#[component]
fn ModalPortalPanel() -> Element {
    let open = use_signal(|| false);
    let on_close = EventHandler::new(move |_| {
        let mut signal = open;
        signal.set(false);
    });
    let scope = use_dismiss(open, on_close, DismissOptions::default());
    use_modal_scope(open, on_close, scope);
    let portal = use_portal();

    rsx! {
        Panel {
            title: "useModalScope over a portal",
            note: "With the panel portaled, every child of body other than the panel is inert — including the app's own root, which the in-place walk-up cannot reach. That is the rule the source wrote its modal scope against.",
            div { class: "flex flex-wrap items-center gap-3",
                button {
                    "data-portal-modal-open": "true",
                    r#type: "button",
                    class: "press h-9 rounded-lg bg-primary px-3 text-sm text-primary-foreground",
                    onclick: move |_| {
                        let mut signal = open;
                        signal.set(true);
                    },
                    "Open"
                }
                PortalProbe { open }
            }
            if open() {
                div {
                    "data-portal-modal": "true",
                    "data-beui-portal": portal.scope(),
                    "data-beui-scope": scope.attr(),
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-label": "Portaled modal",
                    class: "fixed inset-0 z-50 grid place-items-center bg-black/40",
                    onmounted: move |_| portal.attach(),
                    div { class: "glass flex w-72 flex-col gap-3 rounded-2xl p-4",
                        p { class: "text-sm", "The app root behind this is inert." }
                        button {
                            "data-portal-modal-close": "true",
                            r#type: "button",
                            class: "press h-9 rounded-lg border border-border px-3 text-sm",
                            onclick: move |_| on_close.call(()),
                            "Close"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PortalProbe(open: Signal<bool>) -> Element {
    rsx! {
        span {
            "data-probe": "portal-modal-open",
            "data-probe-value": if open() { "true" } else { "false" },
            class: "inline-flex items-center gap-1 rounded-full border border-border bg-card px-2.5 py-1 font-mono text-[11px] text-muted-foreground",
            "portal-modal-open = {open()}"
        }
    }
}

/// Keeps the `Portal` handle type imported for the doc reference.
#[allow(dead_code)]
fn _handle(_: PortalHandle) {}

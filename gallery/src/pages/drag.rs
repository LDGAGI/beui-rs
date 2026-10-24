//! Drag, on screen: the three shapes beUI drags with.
//!
//! - `SheetPanel` is `bottom-sheet`: `drag="y"` with `{top: 0, bottom: 0}` and an
//!   asymmetric `{top: 0.02, bottom: 0.4}` — pulled down it keeps 40% of the
//!   finger's travel, pulled up 2%, and it springs back unless the release was a
//!   fling or a long pull.
//! - `ToastPanel` is `animated-toast-stack`: `drag="x"` with `{left: 0, right: 0}`
//!   and a symmetric `0.18`, dismissed by `|offset.x| > 72` or `|velocity.x| > 520`
//!   — read from the *raw* pointer offset, not from the damped one the element
//!   shows.
//! - `RowPanel` is `swipeable-list`: `drag="x"` with `{left: -leftWidth, right:
//!   rightWidth}` and `0.04`, so it moves freely inside the action widths and
//!   almost not at all beyond them, then snaps to whichever side the release
//!   went.

use beui::prelude::*;
use beui_core::drag::{use_drag, DragAxis, DragEnd, DragOptions, DragRange};
use beui_core::ease::{tween_ease_out, SPRING_LAYOUT};

const DISMISS_THRESHOLD: f64 = 120.0;
const FLING_VELOCITY: f64 = 600.0;
const TOAST_DISMISS: f64 = 72.0;
const TOAST_FLING: f64 = 520.0;
const LEFT_WIDTH: f64 = 96.0;
const RIGHT_WIDTH: f64 = 72.0;

#[component]
pub fn Drag() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Foundations"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Drag"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Pointer capture, frame-to-frame velocity sampling, and the constraint-plus-elasticity pair motion uses. Inside the range the element follows the finger; beyond it, only the elastic share of the excess moves."
                }
            }
            SheetPanel {}
            ToastPanel {}
            RowPanel {}
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

fn round1(value: f64) -> String {
    format!("{value:.1}")
}

// --- bottom sheet: elastic down, barely elastic up ---------------------------

#[component]
fn SheetPanel() -> Element {
    let open = use_signal(|| true);
    let offset = use_signal(|| 0.0_f64);
    let outcome = use_signal(|| "—".to_string());

    let mut set_open = open;
    let mut set_outcome = outcome;

    let drag = use_drag(
        DragOptions::default()
            .axis(DragAxis::Y)
            .range(DragRange::FREE, DragRange::pinned(0.0))
            .elastic_y(0.02, 0.4)
            .without_momentum(),
        Some(EventHandler::new(move |end: DragEnd| {
            let value = if end.velocity.y > FLING_VELOCITY || end.offset.y > DISMISS_THRESHOLD {
                set_open.set(false);
                "dismissed"
            } else {
                "sprang back"
            };
            set_outcome.set(value.to_string());
        })),
    );

    let current = drag.offset().y;
    let mut set_offset = offset;
    use_effect(move || set_offset.set(drag.offset().y));

    rsx! {
        Panel {
            title: "drag=\"y\" · bottom sheet",
            note: "dragConstraints {{top: 0, bottom: 0}} and dragElastic {{top: 0.02, bottom: 0.4}}: down 40%, up 2%. Released, it springs back unless the pull passed 120px or the fling passed 600px/s.",
            div { class: "flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    button {
                        "data-sheet-reset": "true",
                        r#type: "button",
                        class: "press h-9 rounded-lg border border-border px-3 text-sm",
                        onclick: move |_| {
                            set_open.set(true);
                            set_outcome.set("—".to_string());
                        },
                        "Reopen"
                    }
                    Probe { name: "sheet-open", value: open().to_string() }
                    Probe { name: "sheet-offset", value: round1(current) }
                    Probe { name: "sheet-outcome", value: outcome() }
                }
                div { class: "relative h-64 overflow-hidden rounded-xl border border-border bg-card",
                    if open() {
                        div {
                            "data-sheet": "true",
                            "data-beui-drag": drag.scope(),
                            class: "absolute inset-x-0 bottom-0 flex h-40 flex-col gap-2 rounded-t-2xl border-t border-border bg-background p-4 shadow-2xl",
                            style: "transform: translateY({current}px)",
                            onpointerdown: move |event| drag.on_pointer_down(&event),
                            onpointermove: move |event| drag.on_pointer_move(&event),
                            onpointerup: move |event| drag.on_pointer_up(&event),
                            onpointercancel: move |event| drag.on_pointer_cancel(&event),
                            onlostpointercapture: move |event| drag.on_pointer_up(&event),
                            div { class: "mx-auto h-1 w-10 rounded-full bg-border" }
                            p { class: "text-sm text-muted-foreground",
                                "Drag me down. The handle takes the gesture, so the page below does not scroll."
                            }
                        }
                    } else {
                        p { class: "grid h-full place-items-center text-sm text-muted-foreground",
                            "Dismissed — reopen it."
                        }
                    }
                }
            }
        }
    }
}

// --- toast: raw offset decides, damped offset shows --------------------------

#[component]
fn ToastPanel() -> Element {
    let visible = use_signal(|| true);
    let outcome = use_signal(|| "—".to_string());

    let mut set_visible = visible;
    let mut set_outcome = outcome;

    let drag = use_drag(
        DragOptions::default()
            .axis(DragAxis::X)
            .range(DragRange::pinned(0.0), DragRange::pinned(0.0))
            .elastic(beui_core::DragElastic::uniform(0.18)),
        Some(EventHandler::new(move |end: DragEnd| {
            if end.offset.x.abs() > TOAST_DISMISS || end.velocity.x.abs() > TOAST_FLING {
                set_visible.set(false);
                set_outcome.set(format!(
                    "dismissed at {:.0}px / {:.0}px·s⁻¹",
                    end.offset.x, end.velocity.x
                ));
            } else {
                set_outcome.set(format!(
                    "stayed at {:.0}px / {:.0}px·s⁻¹",
                    end.offset.x, end.velocity.x
                ));
            }
        })),
    );

    let current = drag.offset().x;

    rsx! {
        Panel {
            title: "drag=\"x\" · toast",
            note: "dragConstraints {{left: 0, right: 0}} with dragElastic 0.18 — the card only ever shows 18% of the finger's travel, so it looks pinned while the gesture is live. The threshold reads the raw offset, so 72px of finger decides, not 13px of card.",
            div { class: "flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    button {
                        "data-toast-reset": "true",
                        r#type: "button",
                        class: "press h-9 rounded-lg border border-border px-3 text-sm",
                        onclick: move |_| {
                            set_visible.set(true);
                            set_outcome.set("—".to_string());
                        },
                        "Bring it back"
                    }
                    Probe { name: "toast-visible", value: visible().to_string() }
                    Probe { name: "toast-x", value: round1(current) }
                    Probe { name: "toast-outcome", value: outcome() }
                }
                div { class: "relative grid h-32 place-items-center overflow-hidden rounded-xl border border-border bg-card",
                    if visible() {
                        div {
                            "data-toast": "true",
                            "data-beui-drag": drag.scope(),
                            class: "flex w-72 cursor-grab touch-none items-center gap-3 rounded-2xl border border-border bg-background p-3 shadow-2xl select-none [-webkit-touch-callout:none]",
                            style: "transform: translateX({current}px)",
                            onpointerdown: move |event| drag.on_pointer_down(&event),
                            onpointermove: move |event| drag.on_pointer_move(&event),
                            onpointerup: move |event| drag.on_pointer_up(&event),
                            onpointercancel: move |event| drag.on_pointer_cancel(&event),
                            onlostpointercapture: move |event| drag.on_pointer_up(&event),
                            span { class: "size-8 shrink-0 rounded-full bg-brand-accent" }
                            div { class: "flex flex-col",
                                span { class: "text-sm font-medium", "Saved to your library" }
                                span { class: "text-xs text-muted-foreground", "Swipe to dismiss" }
                            }
                        }
                    } else {
                        p { class: "text-sm text-muted-foreground", "Dismissed." }
                    }
                }
            }
        }
    }
}

// --- swipeable row: free inside, 4% beyond ----------------------------------

#[component]
fn RowPanel() -> Element {
    let snapped = use_signal(|| 0.0_f64);
    let last = use_signal(|| "—".to_string());

    let mut set_last = last;

    let drag = use_drag(
        DragOptions::default()
            .axis(DragAxis::X)
            .range(
                DragRange::new(Some(-RIGHT_WIDTH), Some(LEFT_WIDTH)),
                DragRange::FREE,
            )
            .elastic(beui_core::DragElastic::uniform(0.04))
            .without_momentum(),
        None,
    );

    // The row owns where it rests between gestures — motion leaves that to the
    // component. Past half of the revealed width it opens, otherwise it closes.
    let release = EventHandler::new(move |_| {
        let current = drag.offset().x;
        let target = if current < -RIGHT_WIDTH * 0.5 {
            -RIGHT_WIDTH
        } else if current > LEFT_WIDTH * 0.5 {
            LEFT_WIDTH
        } else {
            0.0
        };
        let mut set_snapped = snapped;
        set_snapped.set(target);
        set_last.set(format!("released at {:.0}px → {:.0}px", current, target));
        drag.settle_to(beui_core::Axes { x: target, y: 0.0 }, tween_ease_out(220));
    });

    let current = drag.offset().x;

    rsx! {
        Panel {
            title: "drag=\"x\" · swipeable row",
            note: "dragConstraints {{left: -72, right: 96}} with dragElastic 0.04: full travel inside the action widths, 4% past them. The release decides where it rests — the hook only animates to the target, which is what the source's component does with its own motion value.",
            div { class: "flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    Probe { name: "row-x", value: round1(current) }
                    Probe { name: "row-snapped", value: round1(snapped()) }
                    Probe { name: "row-last", value: last() }
                }
                div {
                    "data-row-stage": "true",
                    class: "relative h-24 overflow-hidden rounded-xl border border-border bg-card",
                    div { class: "absolute inset-y-0 left-0 flex w-24 items-center justify-center bg-warning text-sm font-medium",
                        "Archive"
                    }
                    div { class: "absolute inset-y-0 right-0 flex w-[72px] items-center justify-center bg-danger text-sm font-medium text-primary-foreground",
                        "Delete"
                    }
                    div {
                        "data-row": "true",
                        "data-beui-drag": drag.scope(),
                        class: "absolute inset-0 flex cursor-grab touch-none items-center gap-3 bg-background px-4 select-none [-webkit-touch-callout:none]",
                        style: "transform: translateX({current}px)",
                        onpointerdown: move |event| drag.on_pointer_down(&event),
                        onpointermove: move |event| drag.on_pointer_move(&event),
                        onpointerup: move |event| {
                            drag.on_pointer_up(&event);
                            release.call(());
                        },
                        onpointercancel: move |event| drag.on_pointer_cancel(&event),
                        onlostpointercapture: move |event| drag.on_pointer_up(&event),
                        span { class: "size-8 shrink-0 rounded-full bg-muted" }
                        span { class: "text-sm", "Swipe me either way" }
                    }
                }
            }
        }
    }
}

/// Unused import guard: the spring is referenced so the constant stays imported
/// for the row's settle, which will move to `SPRING_LAYOUT` when the row's snap
/// follows beUI's spring rather than a tween.
#[allow(dead_code)]
fn settle_spring() -> dioxus_motion::prelude::AnimationConfig {
    dioxus_motion::prelude::AnimationConfig::spring(SPRING_LAYOUT)
}

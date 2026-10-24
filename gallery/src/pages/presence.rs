//! Presence, on screen: an overlay that finishes its exit before it unmounts.
//!
//! Two panels, both driven the way a ported overlay is:
//!
//! - `GatePanel` is beUI's drawer shape — a backdrop layer and a panel layer
//!   inside one `AnimatePresence`, each reading the gate so that interaction
//!   stops the moment the exit starts. The checks that matter are *timing* ones:
//!   how long the node stays mounted, what its `inert` and `pointer-events` say
//!   while it exits, and what happens when a re-open interrupts the exit.
//! - `StackPanel` is the `mode="popLayout"` shape the toast stack and the
//!   sortable lists use: the exiting item stays mounted while its siblings move
//!   up immediately, because it has been popped out of layout.

use beui::prelude::*;
use beui_core::ease::tween_ease_out;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceGate, PresenceMode,
};

const ENTER_MS: u64 = 300;
const EXIT_MS: u64 = 300;

/// The backdrop only fades; the panel slides in from the right, the way beUI's
/// drawer does.
fn backdrop_config() -> PresenceConfig {
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        tween_ease_out(ENTER_MS),
        tween_ease_out(EXIT_MS),
    )
}

fn panel_config() -> PresenceConfig {
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, x: 24.0 },
        motion_style! { opacity: 1.0, x: 0.0 },
        motion_style! { opacity: 0.0, x: 24.0 },
        tween_ease_out(ENTER_MS),
        tween_ease_out(EXIT_MS),
    )
}

/// Each row enters from its own side, so the mid-exit frame shows which one is
/// leaving.
fn item_config(id: u32) -> PresenceConfig {
    let from = if id % 2 == 0 { -16.0 } else { 16.0 };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, x: from },
        motion_style! { opacity: 1.0, x: 0.0 },
        motion_style! { opacity: 0.0, x: from },
        tween_ease_out(ENTER_MS),
        tween_ease_out(EXIT_MS),
    )
}

#[component]
pub fn Presence() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Foundations"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Presence and exit"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "AnimatePresence keeps a keyed child mounted until its exit has finished, and the gate hands the isPresent boolean to the layers so interaction releases in the same commit the exit starts."
                }
            }
            GatePanel {}
            StackPanel {}
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

// --- gate: mount until the exit finishes ------------------------------------

/// beUI's drawer renders its two layers as two `{open ? <PresenceGate/> : null}`
/// expressions inside one `AnimatePresence`. The Dioxus port has to write that
/// as a **single keyed iterator**: an `AnimatePresence` boundary can split one
/// fragment of keyed children, but not several sibling blocks — with two of
/// them the boundary logs `cannot split multiple static RSX roots` and renders
/// nothing at all. So the layers become one list whose items carry the key the
/// source gave them (`"backdrop"` / `"panel"`), and picking the markup moves
/// inside the layer component, where the DOM is identical either way.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Backdrop,
    Panel,
}

impl Layer {
    const fn key(self) -> &'static str {
        match self {
            Layer::Backdrop => "backdrop",
            Layer::Panel => "panel",
        }
    }
}

#[component]
fn GatePanel() -> Element {
    let open = use_signal(|| false);
    let exits = use_signal(|| 0u32);
    let backdrop_clicks = use_signal(|| 0u32);

    let on_close = EventHandler::new(move |_| {
        let mut signal = open;
        signal.set(false);
    });
    let on_backdrop = EventHandler::new(move |_| {
        let mut clicks = backdrop_clicks;
        clicks += 1;
        // The backdrop's own job in a real overlay. The gate has already
        // released interaction, so this only ever fires while present.
        let mut signal = open;
        signal.set(false);
    });

    let layers: Vec<Layer> = if open() {
        vec![Layer::Backdrop, Layer::Panel]
    } else {
        Vec::new()
    };

    rsx! {
        Panel {
            title: "AnimatePresence + PresenceGate",
            note: "Two keyed layers, both kept in the tree for the 300 ms exit. The layer reads the gate, so from the commit that starts the exit it is inert and takes no pointer events — a click on the backdrop while it fades out must do nothing.",
            div { class: "flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    button {
                        "data-presence-toggle": "true",
                        r#type: "button",
                        class: "press h-9 rounded-lg bg-primary px-3 text-sm text-primary-foreground",
                        onclick: move |_| {
                            let next = !open();
                            let mut signal = open;
                            signal.set(next);
                        },
                        if open() { "Close" } else { "Open" }
                    }
                    Probe { name: "presence-open", value: open().to_string() }
                    Probe { name: "presence-exits", value: exits().to_string() }
                    Probe { name: "backdrop-clicks", value: backdrop_clicks().to_string() }
                }
                div {
                    "data-presence-stage": "true",
                    class: "relative h-64 overflow-hidden rounded-xl border border-border bg-card",

                    AnimatePresence {
                        on_exit_complete: move |_| {
                            let mut exits = exits;
                            exits += 1;
                        },
                        for layer in layers {
                            PresenceGate {
                                key: "{layer.key()}",
                                GateLayerView { layer, on_close, on_backdrop }
                            }
                        }
                    }

                    if !open() {
                        p { class: "grid h-full place-items-center text-sm text-muted-foreground",
                            "The overlay lives inside this box."
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn GateLayerView(
    layer: Layer,
    on_close: EventHandler<()>,
    on_backdrop: EventHandler<()>,
) -> Element {
    let gate = use_gate();
    let style = use_presence_style(match layer {
        Layer::Backdrop => backdrop_config(),
        Layer::Panel => panel_config(),
    })
    .expect("static presence config");
    let css = presence_css(&style);

    match layer {
        Layer::Backdrop => rsx! {
            button {
                "data-presence-backdrop": "true",
                r#type: "button",
                "aria-label": "Close",
                inert: gate.inert_attr(),
                style: gate.style_with(&css),
                class: "absolute inset-0 cursor-default bg-black/40",
                onclick: move |_| on_backdrop.call(()),
            }
        },
        Layer::Panel => rsx! {
            div {
                "data-presence-panel": "true",
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "Presence demo",
                inert: gate.inert_attr(),
                style: gate.style_with(&css),
                class: "absolute inset-y-0 right-0 flex w-72 flex-col gap-3 bg-background p-4 shadow-2xl",
                p { class: "text-sm text-muted-foreground",
                    "An exiting panel is not a panel you can still use."
                }
                button {
                    r#type: "button",
                    class: "press h-9 rounded-lg border border-border px-3 text-sm",
                    onclick: move |_| on_close.call(()),
                    "Close"
                }
            }
        },
    }
}

// --- popLayout: siblings move now, the exit plays out ------------------------

#[component]
fn StackPanel() -> Element {
    let items = use_signal(|| vec![1u32, 2, 3]);

    let on_dismiss = EventHandler::new(move |id: u32| {
        let remaining: Vec<u32> = items().into_iter().filter(|item| *item != id).collect();
        let mut signal = items;
        signal.set(remaining);
    });

    rsx! {
        Panel {
            title: "AnimatePresence · mode=popLayout",
            note: "Dismissing an item pops it out of layout: the exiting row keeps animating while the rows below it move up in the same commit. This is what the toast stack and the swipe lists are built on.",
            div { class: "flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    Probe { name: "stack-count", value: items().len().to_string() }
                    button {
                        "data-stack-reset": "true",
                        r#type: "button",
                        class: "press h-9 rounded-lg border border-border px-3 text-sm",
                        onclick: move |_| {
                            let mut signal = items;
                            signal.set(vec![1, 2, 3]);
                        },
                        "Reset"
                    }
                }
                div { class: "flex flex-col gap-2 rounded-xl border border-border bg-card p-3",
                    AnimatePresence {
                        mode: PresenceMode::PopLayout,
                        for id in items() {
                            StackItem { key: "{id}", id, on_dismiss }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StackItem(id: u32, on_dismiss: EventHandler<u32>) -> Element {
    let gate = use_gate();
    let style = use_presence_style(item_config(id)).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        div {
            "data-stack-item": "{id}",
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "flex items-center justify-between rounded-lg border border-border bg-background px-3 py-2 text-sm",
            span { "Row {id}" }
            button {
                "data-stack-dismiss": "{id}",
                r#type: "button",
                class: "press rounded-md px-2 py-1 text-xs text-muted-foreground",
                onclick: move |_| on_dismiss.call(id),
                "Dismiss"
            }
        }
    }
}

//! `drawer.preview.tsx` — open from either edge.

use beui::prelude::*;
use beui::{Drawer, DrawerSide};

fn side_label(side: DrawerSide) -> &'static str {
    match side {
        DrawerSide::Left => "left",
        DrawerSide::Right => "right",
    }
}

#[component]
pub fn DrawerPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Drawer" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A backdrop and a panel as two fixed siblings inside one presence boundary. Interaction releases in the same commit the exit starts, so an exiting drawer cannot still be typed into."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let open = use_signal(|| false);
    let side = use_signal(|| DrawerSide::Right);
    let mut set_open = open;
    let mut set_side = side;

    rsx! {
        div { class: "flex items-center gap-3",
            button {
                r#type: "button",
                "data-drawer-open": "left",
                class: "inline-flex h-10 items-center rounded-full border border-border bg-card px-5 text-sm font-medium text-foreground transition-colors hover:bg-card/70",
                onclick: move |_| {
                    set_side.set(DrawerSide::Left);
                    set_open.set(true);
                },
                "Open left"
            }
            button {
                r#type: "button",
                "data-drawer-open": "right",
                class: "inline-flex h-10 items-center rounded-full bg-primary px-5 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90",
                onclick: move |_| {
                    set_side.set(DrawerSide::Right);
                    set_open.set(true);
                },
                "Open right"
            }
            Drawer {
                open: open(),
                side: side(),
                aria_label: "Demo drawer",
                class: "gap-4 p-6",
                on_open_change: move |next| {
                    let mut signal = set_open;
                    signal.set(next);
                },
                h2 { class: "text-sm font-semibold text-foreground", "Drawer" }
                p { class: "text-sm text-muted-foreground",
                    "Slides in from the {side_label(side())}. Press Esc or click outside to close."
                }
            }
        }
    }
}

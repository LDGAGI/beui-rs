//! `bottom-sheet.preview.tsx` — snap points and a fling-to-dismiss handle.

use beui::prelude::*;
use beui::{BottomSheet, SnapPoint};

const ACTIONS: [&str; 6] = [
    "Share",
    "Duplicate",
    "Move to folder",
    "Rename",
    "Archive",
    "Delete",
];

#[component]
pub fn BottomSheetPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Bottom sheet" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A Vaul-style glide: a long, fully-damped tween reads smoother than a spring on open, and the same curve drives the backdrop so the surface and the scrim move as one. Only the handle takes the gesture."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let open = use_signal(|| false);
    let mut set_open = open;

    rsx! {
        div { class: "flex items-center gap-3",
            button {
                r#type: "button",
                "data-sheet-open": "true",
                class: "press inline-flex h-10 items-center rounded-full border border-border bg-card px-5 text-sm font-medium text-foreground hover:border-(--color-border-strong)",
                onclick: move |_| set_open.set(true),
                "Open bottom sheet"
            }
            BottomSheet {
                open: open(),
                snap_points: vec![SnapPoint::Fraction(0.4), SnapPoint::Fraction(0.85)],
                title: "Quick actions",
                description: "Drag the handle, fling, or swipe down to dismiss.",
                on_open_change: move |next| {
                    let mut signal = set_open;
                    signal.set(next);
                },
                ul { class: "divide-y divide-border",
                    for item in ACTIONS {
                        li { key: "{item}", class: "py-3 text-sm text-foreground", "{item}" }
                    }
                }
                div { class: "py-12 text-center text-xs text-muted-foreground",
                    "Fling up to expand, fling down to dismiss."
                }
            }
        }
    }
}

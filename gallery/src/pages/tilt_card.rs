//! `TiltCard` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::TiltCard;

#[component]
pub fn TiltCardPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · pointer"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "TiltCard" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Two SPRING_MOUSE rotation axes plus a glare whose origin is the raw pointer ratio. Skipped on touch and under reduced motion."
                }
            }
            div { class: "flex items-center justify-center p-6",
                TiltCard { class: "w-[280px] border border-border bg-card p-8",
                    div { class: "text-xs uppercase tracking-wider text-muted-foreground", "Premium" }
                    h3 { class: "mt-2 text-2xl font-semibold text-foreground", "Tilt me" }
                    p { class: "mt-3 text-sm text-muted-foreground",
                        "Move your cursor across the card to see 3D tilt + glare."
                    }
                }
            }
        }
    }
}

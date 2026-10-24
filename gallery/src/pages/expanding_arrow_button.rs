//! `ExpandingArrowButton` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::ExpandingArrowButton;

#[component]
pub fn ExpandingArrowButtonPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · button"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ExpandingArrowButton" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "On hover or focus the accent capsule opens into a row of dotted chevrons on SPRING_LAYOUT, with a per-chevron stagger. Skipped on touch and under reduced motion."
                }
            }
            div { class: "flex items-center justify-center",
                ExpandingArrowButton { "Book a demo" }
            }
        }
    }
}

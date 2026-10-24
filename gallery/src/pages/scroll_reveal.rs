//! `ScrollReveal` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{InViewAmount, ScrollReveal};

const CARDS: [&str; 4] = [
    "Spring slide",
    "Blur in",
    "Staggered by delay",
    "Reveal once",
];

#[component]
pub fn ScrollRevealPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · scroll"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ScrollReveal" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "On a page this tracks the viewport. Here root points at the box so each card reveals as it scrolls into the contained view."
                }
            }
            div {
                id: "scroll-reveal-root",
                class: "h-80 w-full max-w-lg overflow-y-auto scrollbar-hide rounded-2xl border border-border bg-card",
                div { class: "flex flex-col gap-16 p-6",
                    div { class: "text-center text-sm text-muted-foreground", "Scroll ↓" }
                    for (i, label) in CARDS.iter().enumerate() {
                        ScrollReveal {
                            key: "{label}",
                            root: Some("#scroll-reveal-root".to_string()),
                            once: false,
                            delay: i as f64 * 0.05,
                            amount: InViewAmount::Fraction(0.3),
                            class: "rounded-xl border border-border bg-muted/50 px-4 py-16 text-center text-base font-medium text-foreground",
                            "{label}"
                        }
                    }
                    div { class: "text-center text-sm text-muted-foreground", "End" }
                }
            }
        }
    }
}

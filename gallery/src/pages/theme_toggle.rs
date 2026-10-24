//! `ThemeToggle` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{RectStart, ThemeToggle, ThemeVariant};

const VARIANTS: [(ThemeVariant, &str); 4] = [
    (ThemeVariant::Rectangle, "Rectangle"),
    (ThemeVariant::Circle, "Circle"),
    (ThemeVariant::CircleBlur, "Circle blur"),
    (ThemeVariant::Blinds, "Blinds"),
];

#[component]
pub fn ThemeTogglePage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · theme"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ThemeToggle" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Dark/light switch with a View Transition reveal. Reduced motion, or a browser without View Transitions, switches straight away."
                }
            }
            div { class: "flex h-full w-full items-center justify-center gap-5",
                for (variant, label) in VARIANTS {
                    div { key: "{label}", class: "flex flex-col items-center gap-2",
                        ThemeToggle {
                            variant,
                            start: RectStart::BottomUp,
                            class: "rounded-xl border border-border bg-background p-2.5",
                            icon_class: Some("h-5 w-5".to_string()),
                        }
                        span { class: "text-[11px] text-muted-foreground", "{label}" }
                    }
                }
            }
        }
    }
}

//! `AnimatedNumber` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{locale_integer, AnimatedNumber};

#[component]
pub fn AnimatedNumberPage() -> Element {
    let format = Callback::new(|n: f64| format!("${}", locale_integer(n)));
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · number"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "AnimatedNumber" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "An EASE_OUT count-up that starts when the element is 60% visible. A value change mid-flight continues from the visible number."
                }
            }
            div { class: "flex flex-col items-center gap-3",
                p { class: "text-xs text-muted-foreground", "Monthly recurring revenue" }
                div { class: "text-4xl font-semibold tracking-tight text-foreground tabular-nums",
                    AnimatedNumber { value: 129480.0, format }
                }
                p { class: "text-xs text-(--color-success)", "+12.4% vs last month" }
            }
        }
    }
}

//! `DigitSwap` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{DigitSwap, DigitSwapDirection};

const CARD_NUMBER: &str = "4242 4242 4242 0806";
const MASKED_NUMBER: &str = "•••• •••• •••• 0806";

#[component]
pub fn DigitSwapPage() -> Element {
    let mut revealed = use_signal(|| false);
    let value = if revealed() {
        CARD_NUMBER
    } else {
        MASKED_NUMBER
    };
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · text"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "DigitSwap" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Fixed slots whose glyphs roll when the value changes. Each slot keeps an invisible in-flow copy for the baseline and clips with clip-path."
                }
            }
            div { class: "flex w-80 flex-col gap-5",
                div { class: "flex flex-col gap-2",
                    span { class: "text-xs font-medium text-muted-foreground", "Card number" }
                    DigitSwap {
                        value: value.to_string(),
                        animation_key: Some(if revealed() { "revealed" } else { "masked" }.to_string()),
                        direction: if revealed() { DigitSwapDirection::Up } else { DigitSwapDirection::Down },
                        suffix_length: 4,
                        glyph_class: Some(if revealed() { "text-foreground" } else { "text-muted-foreground" }.to_string()),
                        suffix_class: Some("text-foreground".to_string()),
                        class: "font-mono text-lg tracking-[0.08em] tabular-nums",
                    }
                }
                button {
                    r#type: "button",
                    aria_label: if revealed() { "Animate masked number" } else { "Animate card number" },
                    aria_pressed: if revealed() { "true" } else { "false" },
                    onclick: move |_| revealed.toggle(),
                    class: "h-10 self-start rounded-lg border border-border px-3 text-sm font-medium text-foreground outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
                    "Animate"
                }
            }
        }
    }
}

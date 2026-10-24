//! `CardFolder` — the beUI preview, same copy and structure.

use beui::digit_swap::DigitSwapDirection;
use beui::prelude::*;
use beui::{CardFolder, DigitSwap};

const CARD_NUMBER: &str = "3745 6987 4096 0806";
const MASKED_CARD_NUMBER: &str = "•••• •••• •••• 0806";

fn contour_radii() -> Vec<f64> {
    (0..18).map(|index| 72.0 + index as f64 * 24.0).collect()
}

#[component]
fn CardArtwork(details_visible: bool) -> Element {
    let radii = contour_radii();
    let mask = if details_visible {
        CARD_NUMBER
    } else {
        MASKED_CARD_NUMBER
    };
    let direction = if details_visible {
        DigitSwapDirection::Up
    } else {
        DigitSwapDirection::Down
    };
    let number_class = if details_visible {
        "text-white/90"
    } else {
        "text-white/65"
    };
    let animation_key = if details_visible {
        "revealed"
    } else {
        "masked"
    }
    .to_string();
    let cvv = if details_visible { "123" } else { "•••" };

    rsx! {
        span { class: "relative block h-full w-full overflow-hidden bg-[linear-gradient(135deg,#383a38_0%,#202120_48%,#111211_100%)] text-white",
            svg { "aria-hidden": "true", view_box: "0 0 640 404", class: "absolute inset-0 h-full w-full",
                for (index, radius) in radii.iter().enumerate() {
                    circle {
                        key: "{radius}",
                        cx: "-20",
                        cy: "-28",
                        r: "{radius}",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_opacity: "{0.32 - index as f64 * 0.009}",
                        stroke_width: "1.5",
                    }
                }
            }
            span { class: "absolute inset-0 bg-[radial-gradient(circle_at_78%_8%,rgb(255_255_255/0.11),transparent_32%)]" }
            span { class: "absolute left-[6%] top-[8%] flex flex-col gap-2",
                span { class: "text-[10px] font-medium uppercase tracking-[0.22em] text-white/55", "Saurabh Chauhan" }
                DigitSwap {
                    value: mask.to_string(),
                    animation_key: Some(animation_key.clone()),
                    direction,
                    suffix_length: 4,
                    glyph_class: Some(number_class.to_string()),
                    suffix_class: Some("text-white/90".to_string()),
                    class: "font-mono text-xs tracking-[0.13em]",
                }
            }
            span { class: "absolute top-[8%] right-[7%] h-[20%] w-[13%] overflow-hidden rounded-[18%] border border-black/30 bg-[linear-gradient(135deg,#f2f0ea_0%,#b9b7b1_45%,#e4e1d9_100%)]",
                span { class: "absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-black/30" }
                span { class: "absolute inset-x-0 top-1/3 h-px bg-black/30" }
                span { class: "absolute inset-x-0 bottom-1/3 h-px bg-black/30" }
                span { class: "absolute left-0 top-1/2 h-[34%] w-[28%] -translate-y-1/2 rounded-r-[35%] border border-l-0 border-black/30" }
                span { class: "absolute right-0 top-1/2 h-[34%] w-[28%] -translate-y-1/2 rounded-l-[35%] border border-r-0 border-black/30" }
            }
            span { class: "absolute bottom-[9%] left-[6%] flex items-center gap-2 text-lg font-semibold tracking-[-0.04em]",
                "beUI"
                span { class: "block size-2.5 rotate-45 rounded-[2px] bg-white/85" }
            }
            span { class: "absolute right-[7%] bottom-[10%] flex items-end gap-5",
                span { class: "flex flex-col gap-1",
                    span { class: "text-[8px] font-medium uppercase tracking-[0.16em] text-white/40", "Expiry" }
                    span { class: "text-xs font-medium text-white/90 tabular-nums", "08/29" }
                }
                span { class: "flex flex-col gap-1",
                    span { class: "text-[8px] font-medium uppercase tracking-[0.16em] text-white/40", "CVV" }
                    DigitSwap {
                        value: cvv.to_string(),
                        animation_key: Some(animation_key),
                        direction,
                        class: "font-mono text-xs font-medium text-white/90",
                    }
                }
            }
        }
    }
}

#[component]
pub fn CardFolderPage() -> Element {
    let details_visible = use_signal(|| false);
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "CardFolder" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A landscape card tucked into an animated folder sleeve. One progress value lifts the card (sin(pi*p)) while the purse collapses into its bottom seam; a separate privacy control reveals the number and CVV through DigitSwap."
                }
            }
            div { class: "flex min-h-72 w-full items-center justify-center px-6 py-9",
                CardFolder {
                    title: "Saurabh Chauhan".to_string(),
                    card_number: CARD_NUMBER.to_string(),
                    expiry: "08/29".to_string(),
                    cvv: "123".to_string(),
                    details_visible: Some(details_visible()),
                    on_details_visible_change: Some(EventHandler::new({
                        let mut details_visible = details_visible;
                        move |next: bool| details_visible.set(next)
                    })),
                    card: rsx! { CardArtwork { details_visible: details_visible() } },
                }
            }
        }
    }
}

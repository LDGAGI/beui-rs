//! `ActionSwap` — the beUI previews, same copy and structure: the blur, roll
//! and cascade variants side by side.

use beui::icons::{Check, Copy, Send, Sparkles};
use beui::prelude::*;
use beui::{ActionSwapAnimation, ActionSwapButton, ActionSwapButtonVariant, ActionSwapItem};

fn copy_items() -> Vec<ActionSwapItem> {
    vec![
        ActionSwapItem::new("copy", "Copy link").icon(rsx! {
            Copy { class: Some("h-4 w-4".to_string()) }
        }),
        ActionSwapItem::new("copied", "Copied").icon(rsx! {
            Check { class: Some("h-4 w-4".to_string()) }
        }),
    ]
}

fn send_items() -> Vec<ActionSwapItem> {
    vec![
        ActionSwapItem::new("send", "Send invite").icon(rsx! {
            Send { class: Some("h-4 w-4".to_string()) }
        }),
        ActionSwapItem::new("sent", "Invite sent").icon(rsx! {
            Sparkles { class: Some("h-4 w-4".to_string()) }
        }),
    ]
}

#[component]
pub fn ActionSwapPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · control"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ActionSwap" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A control whose label and icon swap with a blur, a roll, or a per-letter cascade. The variant is the animation prop; cascade falls back to roll for non-string content."
                }
            }
            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "Blur" }
                div { class: "flex flex-wrap items-center justify-center gap-3",
                    ActionSwapButton {
                        items: copy_items(),
                        default_value: Some("copy".to_string()),
                        animation: ActionSwapAnimation::Blur,
                        variant: ActionSwapButtonVariant::Secondary,
                    }
                    ActionSwapButton {
                        items: send_items(),
                        default_value: Some("send".to_string()),
                        animation: ActionSwapAnimation::Blur,
                        variant: ActionSwapButtonVariant::Primary,
                    }
                }
            }
            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "Roll" }
                div { class: "flex flex-wrap items-center justify-center gap-3",
                    ActionSwapButton {
                        items: copy_items(),
                        default_value: Some("copy".to_string()),
                        animation: ActionSwapAnimation::Roll,
                        variant: ActionSwapButtonVariant::Secondary,
                    }
                    ActionSwapButton {
                        items: send_items(),
                        default_value: Some("send".to_string()),
                        animation: ActionSwapAnimation::Roll,
                        variant: ActionSwapButtonVariant::Primary,
                    }
                }
            }
            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "Cascade" }
                div { class: "flex w-full justify-center",
                    ActionSwapButton {
                        items: copy_items(),
                        default_value: Some("copy".to_string()),
                        animation: ActionSwapAnimation::Cascade,
                        variant: ActionSwapButtonVariant::Primary,
                    }
                }
            }
        }
    }
}

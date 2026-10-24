//! `OverflowActions` — the beUI preview, same copy and structure.

use beui::icons::{CalendarClock, Eye, GitBranch, Pin};
use beui::prelude::*;
use beui::{OverflowActionItem, OverflowActions};

fn primary_actions() -> Vec<OverflowActionItem> {
    vec![
        OverflowActionItem::new("preview", "Preview").icon(rsx! {
            Eye { class: Some("h-4 w-4".to_string()) }
        }),
        OverflowActionItem::new("pin", "Pin").icon(rsx! {
            Pin { class: Some("h-4 w-4".to_string()) }
        }),
    ]
}

fn overflow_actions() -> Vec<OverflowActionItem> {
    vec![
        OverflowActionItem::new("branch", "Branch").icon(rsx! {
            GitBranch { class: Some("h-4 w-4".to_string()) }
        }),
        OverflowActionItem::new("schedule", "Schedule").icon(rsx! {
            CalendarClock { class: Some("h-4 w-4".to_string()) }
        }),
    ]
}

#[component]
pub fn OverflowActionsPage() -> Element {
    let expanded = use_signal(|| false);
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "OverflowActions" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A pill bar whose extra actions slide out of a toggle on SHELL_TRANSITION (220, 17, 0.85). The overflow group's width springs from 0 to its measured natural width, so the bar grows with it."
                }
            }
            div { class: "flex w-full items-center justify-center",
                OverflowActions {
                    primary_actions: primary_actions(),
                    overflow_actions: overflow_actions(),
                    expanded: Some(expanded()),
                    on_expanded_change: Some(EventHandler::new({
                        let mut expanded = expanded;
                        move |next: bool| expanded.set(next)
                    })),
                    open_label: "Open action rail".to_string(),
                    close_label: "Collapse action rail".to_string(),
                }
            }
        }
    }
}

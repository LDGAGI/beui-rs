//! `SwipeableList` — the beUI preview, same copy and structure.

use beui::icons::{Check, Clock3, FileText, Flag, Mail, Pin, RotateCcw, Trash2, UserRound};
use beui::prelude::*;
use beui::{SwipeAction, SwipeActionEvent, SwipeActionTone, SwipeableList, SwipeableListItem};

fn left_actions() -> Vec<SwipeAction> {
    vec![
        SwipeAction::new(
            "done",
            "Done",
            rsx! { Check { class: Some("h-4 w-4".to_string()) } },
        )
        .tone(SwipeActionTone::Success),
        SwipeAction::new(
            "pin",
            "Pin",
            rsx! { Pin { class: Some("h-4 w-4".to_string()) } },
        )
        .tone(SwipeActionTone::Primary),
    ]
}

fn right_actions() -> Vec<SwipeAction> {
    vec![
        SwipeAction::new(
            "later",
            "Later",
            rsx! { Clock3 { class: Some("h-4 w-4".to_string()) } },
        )
        .tone(SwipeActionTone::Warning),
        SwipeAction::new(
            "trash",
            "Trash",
            rsx! { Trash2 { class: Some("h-4 w-4".to_string()) } },
        )
        .tone(SwipeActionTone::Danger),
    ]
}

fn leading(icon: Element) -> Element {
    rsx! {
        div { class: "grid h-10 w-10 place-items-center rounded-xl border border-border bg-background text-muted-foreground",
            {icon}
        }
    }
}

fn initial_items() -> Vec<SwipeableListItem> {
    vec![
        SwipeableListItem::new("brief")
            .title("Launch brief")
            .description("Finalize the announcement copy")
            .meta("9:41")
            .leading(leading(
                rsx! { FileText { class: Some("h-4 w-4".to_string()) } },
            ))
            .left_actions(left_actions())
            .right_actions(right_actions()),
        SwipeableListItem::new("feedback")
            .title("Client feedback")
            .description("Three comments need a response")
            .meta("11:08")
            .leading(leading(
                rsx! { Mail { class: Some("h-4 w-4".to_string()) } },
            ))
            .left_actions(left_actions())
            .right_actions(right_actions()),
        SwipeableListItem::new("review")
            .title("Design review")
            .description("Check spacing before handoff")
            .meta("13:20")
            .leading(leading(
                rsx! { UserRound { class: Some("h-4 w-4".to_string()) } },
            ))
            .left_actions(left_actions())
            .right_actions(right_actions()),
        SwipeableListItem::new("incident")
            .title("Flagged run")
            .description("Retry queue has one failed job")
            .meta("Now")
            .leading(leading(
                rsx! { Flag { class: Some("h-4 w-4".to_string()) } },
            ))
            .left_actions(left_actions())
            .right_actions(right_actions()),
    ]
}

#[component]
pub fn SwipeableListPage() -> Element {
    let mut items = use_signal(initial_items);
    let mut last_action = use_signal(|| "Ready".to_string());

    let on_action = EventHandler::new({
        let mut items = items;
        let mut last_action = last_action;
        move |event: SwipeActionEvent| {
            let title = event.item.title.clone().unwrap_or_default();
            last_action.set(format!("{} · {title}", event.action.label));
            if event.action.id == "trash" {
                let remaining: Vec<SwipeableListItem> = items
                    .read()
                    .iter()
                    .filter(|entry| entry.id != event.item.id)
                    .cloned()
                    .collect();
                items.set(remaining);
            }
        }
    });

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "SwipeableList" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Rows that reveal action rails on a horizontal drag. The drag runs through beui-core::drag with the source's flick thresholds and ROW_SETTLE spring; touch-pan-y keeps the page's vertical scroll."
                }
            }
            div { class: "flex min-h-96 w-full items-center justify-center",
                div { class: "w-full max-w-sm rounded-[2rem] border border-border bg-background p-3 shadow-2xl",
                    div { class: "mb-3 flex items-center justify-between px-1",
                        div {
                            p { class: "text-sm font-semibold text-foreground", "Priority queue" }
                            p { class: "text-xs text-muted-foreground", "{last_action()}" }
                        }
                        button {
                            r#type: "button",
                            class: "inline-flex h-8 items-center gap-1.5 rounded-full border border-border px-3 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground",
                            onclick: move |_| {
                                items.set(initial_items());
                                last_action.set("Queue restored".to_string());
                            },
                            RotateCcw { class: Some("h-3.5 w-3.5".to_string()) }
                            "Reset"
                        }
                    }
                    SwipeableList { items: items(), on_action: Some(on_action) }
                    div { class: "mt-3 flex items-center justify-between px-1 text-[11px] font-medium text-muted-foreground",
                        span { "{items().len()} open" }
                        span { "Today" }
                    }
                }
            }
        }
    }
}

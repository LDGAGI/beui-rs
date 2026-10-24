//! `SharedLayoutBg` — the beUI preview, same copy and structure.
//!
//! The source clones `<button>` children; here each row is a
//! [`SharedLayoutItem`] whose content is the same button.

use beui::icons::ArrowUpRight;
use beui::prelude::*;
use beui::{SharedLayoutBg, SharedLayoutItem};

const ITEMS: [(&str, &str); 4] = [
    ("Inbox", "12 unread threads, 3 mentions today."),
    ("Drafts", "4 posts waiting for a final pass."),
    ("Releases", "Last shipped 2 days ago, v0.4.1."),
    ("Billing", "Plan renews on the 1st of next month."),
];

#[component]
pub fn SharedLayoutBgPage() -> Element {
    let items = ITEMS
        .iter()
        .map(|(title, body)| {
            SharedLayoutItem::new(
                *title,
                rsx! {
                    button {
                        r#type: "button",
                        class: "group flex w-full flex-col gap-1 px-2 py-3 text-left",
                        div { class: "flex items-center justify-between gap-3",
                            span { class: "text-sm font-medium text-foreground", "{title}" }
                            ArrowUpRight { class: Some("h-3.5 w-3.5 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5".to_string()) }
                        }
                        p { class: "text-sm text-muted-foreground", "{body}" }
                    }
                },
            )
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · layout"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "SharedLayoutBg" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A hover pill that morphs between rows. The pill is handed between rows by layoutId, which here is beui-core's flip registry."
                }
            }
            div { class: "w-full max-w-lg px-2",
                SharedLayoutBg { items }
            }
        }
    }
}

//! `NotificationStack` — the beUI preview, same copy and structure.

use beui::icons::RotateCw;
use beui::prelude::*;
use beui::{NotificationStack, NotificationStackItem};

fn notifications() -> Vec<NotificationStackItem> {
    vec![
        NotificationStackItem::new("import-failed", "Orders import failed")
            .description("42s · TimeoutError at Step 2")
            .trailing(rsx! {
                span { class: "inline-flex items-center gap-1 text-amber-500 dark:text-amber-400",
                    RotateCw { class: Some("h-3.5 w-3.5".to_string()) }
                    "2"
                }
            }),
        NotificationStackItem::new("sla-breach", "SLA breach")
            .description("2m 11s · Data enrichment"),
        NotificationStackItem::new("sync-fixed", "Product sync auto-fixed")
            .description("5m · 404 on GET /products"),
    ]
}

#[component]
pub fn NotificationStackPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "NotificationStack" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A collapsed card stack that expands on hover, focus or tap, with the source's gesture split between pointer-leave and outside-tap. Motion's layout projection is a known gap; the cards' y and clip-path are animated instead."
                }
            }
            div { class: "flex w-full items-center justify-center pt-52 pb-6",
                NotificationStack { items: notifications() }
            }
        }
    }
}

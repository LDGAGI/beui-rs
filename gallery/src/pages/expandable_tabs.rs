//! `expandable-tabs` — a Dynamic-Island bar that grows into its panel.
//!
//! beUI has no `expandable-tabs.preview.tsx`; this page shows the component's
//! own behaviour: three icon tabs, the active one growing to fit its label while
//! the panel unfolds above the dock.

use beui::prelude::*;
use beui::{ExpandableTabs, ExpandableTabsIcon, ExpandableTabsItem};

fn items() -> Vec<ExpandableTabsItem> {
    vec![
        ExpandableTabsItem::new(
            "home",
            "Home",
            "Everything from your workspace, in one place.",
        )
        .icon(ExpandableTabsIcon::Home),
        ExpandableTabsItem::new("inbox", "Inbox", "3 unread conversations.")
            .icon(ExpandableTabsIcon::Mail),
        ExpandableTabsItem::new("settings", "Settings", "Preferences and notifications.")
            .icon(ExpandableTabsIcon::Settings),
    ]
}

#[component]
pub fn ExpandableTabsPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Expandable tabs" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The shell is sized by two springs: closed it is the bar alone, open it grows to the measured content size. The active tab widens to fit its label while the label unclips, and the panel enters from just above the dock."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    rsx! {
        section { class: "flex flex-col gap-4",
            h2 { class: "text-lg font-semibold tracking-tight", "Expandable tabs" }
            p { class: "max-w-3xl text-sm text-muted-foreground",
                "Click a tab to unfold its panel; click it again, press Escape, or click outside to close."
            }
            div { class: "flex min-h-72 items-center justify-center py-8",
                ExpandableTabs { items: items(), class: "w-max" }
            }
        }
    }
}

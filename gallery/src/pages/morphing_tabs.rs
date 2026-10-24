//! `morphing-tabs` — a rail whose active tab is joined to its panel.
//!
//! beUI has no `morphing-tabs.preview.tsx`; this page shows the component's own
//! behaviour: the liquid surface's notch follows the active tab on a spring, and
//! tabs can be dragged or Alt+arrow-reordered.

use beui::prelude::*;
use beui::{MorphingTabs, MorphingTabsItem};

const TABS: [(&str, &str, &str); 4] = [
    (
        "overview",
        "Overview",
        "A high-level summary of the workspace and its recent activity.",
    ),
    (
        "activity",
        "Activity",
        "Every event, grouped by the day it landed on.",
    ),
    (
        "reports",
        "Reports",
        "Scheduled exports and the people who receive them.",
    ),
    ("settings", "Settings", "Preferences, members, and billing."),
];

#[component]
pub fn MorphingTabsPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Morphing tabs" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The surface is one continuous SVG shape spanning the panel, with a notch cut where the active tab sits. The notch follows a spring as the active tab changes and while a tab is dragged to reorder."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let items = TABS
        .iter()
        .map(|(id, label, content)| MorphingTabsItem::new(*id, *label, *content))
        .collect::<Vec<_>>();
    let order = use_signal(|| "overview".to_string());

    rsx! {
        section { class: "flex flex-col gap-4",
            h2 { class: "text-lg font-semibold tracking-tight", "Morphing tabs" }
            p { class: "max-w-3xl text-sm text-muted-foreground",
                "Click a tab; drag one sideways to reorder, or focus one and press Alt+Left / Alt+Right."
            }
            div { class: "w-full max-w-2xl",
                MorphingTabs {
                    items,
                    default_value: Some("overview".to_string()),
                    on_order_change: move |ids: Vec<String>| {
                        let mut slot = order;
                        slot.set(ids.join(", "));
                    },
                }
            }
            p { class: "text-xs text-muted-foreground", "order: {order()}" }
        }
    }
}

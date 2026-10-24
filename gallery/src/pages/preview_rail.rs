//! `PreviewRail` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{PreviewRail, PreviewRailItem, PreviewRailOrientation};

const ITEMS: [(&str, &str, &str); 14] = [
    (
        "dashboard",
        "Dashboard",
        "Return to your workspace overview and recent activity.",
    ),
    (
        "components",
        "Components",
        "Browse motion primitives for React and Next.js.",
    ),
    (
        "blocks",
        "Blocks",
        "Explore composed, product-ready interface blocks.",
    ),
    (
        "playground",
        "Playground",
        "Tune motion values and preview behavior live.",
    ),
    (
        "docs",
        "Documentation",
        "Read installation, usage, and API reference notes.",
    ),
    (
        "changelog",
        "Changelog",
        "Review newly launched components and improvements.",
    ),
    (
        "sponsors",
        "Sponsors",
        "Support continued development of the open-source library.",
    ),
    (
        "pro",
        "beUI Pro",
        "Get premium components and lifetime access.",
    ),
    (
        "examples",
        "Examples",
        "See components composed in practical interface patterns.",
    ),
    (
        "templates",
        "Templates",
        "Start from polished layouts built with beUI components.",
    ),
    (
        "guides",
        "Guides",
        "Learn how to combine motion primitives effectively.",
    ),
    (
        "community",
        "Community",
        "Discover what other builders are creating with beUI.",
    ),
    (
        "github",
        "GitHub",
        "View the source, report issues, and contribute improvements.",
    ),
    (
        "about",
        "About",
        "Learn more about the ideas and people behind beUI.",
    ),
];

fn items() -> Vec<PreviewRailItem> {
    ITEMS
        .iter()
        .map(|(id, label, description)| {
            PreviewRailItem::new(*id, *label)
                .description(*description)
                .href(format!("#{id}"))
        })
        .collect()
}

#[component]
pub fn PreviewRailPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · navigation"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "PreviewRail" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A tick rail whose preview card follows the pointer, keyboard focus or a tap. Ticks scale by distance from the displayed row; the card is handed between rows by layoutId."
                }
            }
            div { class: "flex w-full flex-col gap-8",
                PreviewRail {
                    items: items(),
                    default_active_id: Some("docs".to_string()),
                    class: "mx-auto h-[360px] w-full max-w-2xl",
                }
                PreviewRail {
                    items: items(),
                    orientation: PreviewRailOrientation::Horizontal,
                    default_active_id: Some("docs".to_string()),
                    class: "mx-auto h-[280px] w-full max-w-2xl",
                }
            }
        }
    }
}

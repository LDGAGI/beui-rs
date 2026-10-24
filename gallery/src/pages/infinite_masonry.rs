//! `InfiniteMasonry` — the beUI preview: a paged visual index.
//!
//! The source's figures show remote photos; the gallery renders gradients of the
//! same heights instead, so the page stays offline and the smoke's console-error
//! check stays meaningful.

use beui::prelude::*;
use beui::{InfiniteMasonry, MasonryItem};
use beui_core::platform;

const PAGE_SIZE: usize = 6;
const MAX_ITEMS: usize = 42;

/// `(title, category, height, gradient)`
const SOURCE: [(&str, &str, f64, &str); 6] = [
    (
        "Soft geometry",
        "Architecture",
        260.0,
        "linear-gradient(160deg,#c7d2fe,#818cf8)",
    ),
    (
        "Open horizon",
        "Landscape",
        190.0,
        "linear-gradient(160deg,#bae6fd,#38bdf8)",
    ),
    (
        "Working rhythm",
        "Workspace",
        230.0,
        "linear-gradient(160deg,#fde68a,#f59e0b)",
    ),
    (
        "Shared table",
        "Studio",
        300.0,
        "linear-gradient(160deg,#fecdd3,#fb7185)",
    ),
    (
        "In session",
        "People",
        210.0,
        "linear-gradient(160deg,#bbf7d0,#34d399)",
    ),
    (
        "After hours",
        "Office",
        280.0,
        "linear-gradient(160deg,#ddd6fe,#a78bfa)",
    ),
];

fn create_items(start: usize, count: usize) -> Vec<MasonryItem> {
    (0..count)
        .map(|offset| {
            let index = start + offset;
            let (title, category, height, gradient) = SOURCE[index % SOURCE.len()];
            let round = index / SOURCE.len() + 1;
            let content = rsx! {
                figure { class: "overflow-hidden rounded-2xl border border-border bg-card",
                    div { class: "relative w-full overflow-hidden", style: "height:{height}px;background:{gradient}" }
                    figcaption { class: "flex items-start justify-between gap-3 p-3",
                        p { class: "text-sm font-medium text-foreground", "{title} {round}" }
                        p { class: "shrink-0 text-[10px] uppercase tracking-wider text-muted-foreground",
                            "{category}"
                        }
                    }
                }
            };
            // `estimateSize` in the source: the image plus the caption.
            MasonryItem::new(format!("gallery-{index}"), content).height(height + 66.0)
        })
        .collect()
}

#[component]
pub fn InfiniteMasonryPage() -> Element {
    let mut items = use_signal(|| create_items(0, 12));
    let mut loading = use_signal(|| false);
    let has_more = items().len() < MAX_ITEMS;

    let mut load_more = move || {
        if loading() || !has_more {
            return;
        }
        loading.set(true);
        spawn(async move {
            let _ = platform::json::<bool>(
                "await new Promise((resolve) => setTimeout(resolve, 650));\nreturn true;",
            )
            .await;
            let current = items().len();
            let count = PAGE_SIZE.min(MAX_ITEMS - current);
            if count > 0 {
                items.write().extend(create_items(current, count));
            }
            loading.set(false);
        });
    };

    let count = items().len();

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "InfiniteMasonry" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A paged masonry feed. Each item joins the shortest column, the tail shows one placeholder per column while loading, and the next page is requested once the third-from-last item comes into view."
                }
            }
            div { class: "w-full max-w-5xl",
                div { class: "mb-3 flex items-end justify-between gap-4 px-1",
                    div {
                        p { class: "text-sm font-semibold text-foreground", "Visual index" }
                        p { class: "mt-0.5 text-xs text-muted-foreground", "Scroll to load the next page" }
                    }
                    p { class: "font-mono text-xs tabular-nums text-muted-foreground", "{count} items" }
                }
                InfiniteMasonry {
                    items: items(),
                    has_more,
                    loading: loading(),
                    prefetch: 3,
                    aria_label: "Visual inspiration gallery".to_string(),
                    class: Some("h-[34rem]".to_string()),
                    on_load_more: EventHandler::new(move |_| load_more()),
                }
            }
        }
    }
}

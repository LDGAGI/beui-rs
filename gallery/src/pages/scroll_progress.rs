//! `ScrollProgress` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{use_scroll_y_progress, ScrollProgress, ScrollProgressVariant};

const SECTIONS: std::ops::RangeInclusive<usize> = 1..=18;

#[component]
pub fn ScrollProgressPage() -> Element {
    let progress = use_scroll_y_progress(Some("#scroll-progress-box".to_string()));
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · scroll"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ScrollProgress" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Real usage drops it anywhere and it reads page scroll, pinning itself with fixed. Here it is scoped to a box with a contained progress source and fixed=false."
                }
            }
            div { class: "relative w-full max-w-lg overflow-hidden rounded-2xl border border-border bg-card",
                ScrollProgress { progress: Some(progress), fixed: false, height: 3.0 }
                div { class: "absolute right-3 top-3 z-10 rounded-full bg-background/70 p-1 backdrop-blur",
                    ScrollProgress {
                        variant: ScrollProgressVariant::Circle,
                        progress: Some(progress),
                        size: 36.0,
                    }
                }
                div { id: "scroll-progress-box", class: "h-64 overflow-y-auto scrollbar-hide",
                    div { class: "space-y-3 p-4",
                        for n in SECTIONS {
                            div {
                                key: "row-{n}",
                                class: "rounded-lg bg-muted/60 px-3 py-4 text-sm text-muted-foreground",
                                "Section {n}"
                            }
                        }
                    }
                }
            }
        }
    }
}

//! `SmoothScroll` + `ScrollTo` — the beUI preview, same copy and structure.
//!
//! Lenis' smoothing engine is a JS-interop gap, so this runs the native path
//! (which the source also uses under reduced motion); the nav buttons glide the
//! contained box to each section.

use beui::prelude::*;
use beui::{ScrollTo, SmoothScroll};

const SECTIONS: [(&str, &str); 4] = [
    ("sec-intro", "Intro"),
    ("sec-features", "Features"),
    ("sec-pricing", "Pricing"),
    ("sec-faq", "FAQ"),
];

#[component]
pub fn ScrollToPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · scroll"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ScrollTo / SmoothScroll" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A scroll provider that shares offset, progress and velocity, and a button that glides to a target. beUI drives this with Lenis; that smoothing engine is a JS-interop gap here, so the native glide path runs instead."
                }
            }
            SmoothScroll {
                root: false,
                class: "relative h-80 w-full max-w-lg overflow-y-auto scrollbar-hide rounded-2xl border border-border bg-card",
                nav { class: "sticky top-0 z-10 flex gap-1.5 border-b border-border bg-background/80 p-2 backdrop-blur",
                    for (id, label) in SECTIONS {
                        ScrollTo {
                            key: "{id}",
                            to: format!("#{id}"),
                            offset: -48.0,
                            class: "rounded-full px-3 py-1 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground",
                            "{label}"
                        }
                    }
                }
                for (id, label) in SECTIONS {
                    section {
                        key: "{id}",
                        id: "{id}",
                        class: "flex h-64 items-center justify-center text-lg font-medium text-foreground",
                        "{label}"
                    }
                }
            }
        }
    }
}

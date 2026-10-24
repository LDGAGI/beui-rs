//! `TextShimmer` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::TextShimmer;

#[component]
pub fn TextShimmerPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · text"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "TextShimmer" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A gradient sweep clipped to the text. Pure CSS: the keyframes travel with the component, and the reduced-motion rule switches the sweep off."
                }
            }
            section { class: "flex flex-col gap-4",
                div { class: "flex flex-col gap-4",
                    TextShimmer { class: "text-3xl font-semibold", "Loading projects…" }
                    TextShimmer { duration: 1.5, class: "text-sm", "Faster shimmer" }
                }
            }
        }
    }
}

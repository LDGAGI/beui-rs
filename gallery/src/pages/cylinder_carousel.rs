//! `CylinderCarousel` — the beUI preview rolls a row of slides; the source's
//! slides are `ShaderBackground` (an external package) and its variant toggle is
//! another lane's `Tabs`, so this page substitutes gradients and plain buttons.

use beui::prelude::*;
use beui::{CylinderCarousel, CylinderVariant};

const SLIDES: [(&str, &str); 8] = [
    ("Dithering", "linear-gradient(160deg,#1a1030,#b98cff)"),
    ("Metaballs", "linear-gradient(160deg,#c9b9a8,#8a8a9a)"),
    ("Warp", "linear-gradient(160deg,#3a5a00,#c8ff00)"),
    ("God rays", "linear-gradient(160deg,#00114d,#6a7bff)"),
    ("Swirl", "linear-gradient(160deg,#1a0000,#ff6a3d)"),
    ("Mesh", "linear-gradient(160deg,#241d9a,#f75092)"),
    ("Voronoi", "linear-gradient(160deg,#ff8247,#ffe53d)"),
    ("Neuro noise", "linear-gradient(160deg,#000000,#47a6ff)"),
];

#[component]
pub fn CylinderCarouselPage() -> Element {
    let mut variant = use_signal(|| CylinderVariant::Concave);
    let mut index = use_signal(|| 0usize);

    let items: Vec<Element> = SLIDES
        .iter()
        .map(|(label, gradient)| {
            rsx! {
                div {
                    class: "flex h-full w-full items-center justify-center overflow-hidden rounded-full border border-border/40",
                    style: "background:{gradient}",
                    span { class: "text-sm font-medium text-white/90", "{label}" }
                }
            }
        })
        .collect();

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "CylinderCarousel" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A row of slides projected onto the inside of a cylinder. Horizontal position, size and lift all share one 1/(cosθ + k) depth term, so the centre slide is smallest and highest while the edges grow and slide out of frame."
                }
            }
            div { class: "flex w-full flex-col items-center gap-4",
                div { class: "flex items-center gap-2 rounded-full border border-border bg-muted/40 p-1",
                    for (candidate, label) in [(CylinderVariant::Concave, "Concave"), (CylinderVariant::Convex, "Convex")] {
                        button {
                            key: "{label}",
                            r#type: "button",
                            class: cn!(
                                "rounded-full px-3 py-1 text-xs font-medium transition-colors",
                                if variant() == candidate { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" },
                            ),
                            onclick: move |_| variant.set(candidate),
                            "{label}"
                        }
                    }
                }
                div { class: "w-full rounded-3xl border border-border/60 bg-muted/20 py-6 [clip-path:inset(0_round_1.5rem)]",
                    CylinderCarousel {
                        items,
                        item_size: 230.0,
                        height: Some(310.0),
                        variant: variant(),
                        on_index_change: Some(EventHandler::new(move |next: usize| index.set(next))),
                    }
                }
                p { class: "text-xs text-muted-foreground",
                    "Drag, scroll or use arrow keys to roll · resting on " span { class: "font-mono", "{index()}" }
                }
            }
        }
    }
}

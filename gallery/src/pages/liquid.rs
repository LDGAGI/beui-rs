//! `Liquid` — no beUI preview exists (it is a primitive other components build
//! on), so this demo drives the morph between three segments.

use beui::prelude::*;
use beui::rect::use_element_width;
use beui::{Liquid, LiquidItem};

const SEGMENTS: [&str; 3] = ["Overview", "Activity", "Settings"];

#[component]
pub fn LiquidPage() -> Element {
    let mut active = use_signal(|| 0usize);
    let width_stream = use_element_width();
    let width = {
        let measured = width_stream.width();
        if measured > 0.0 {
            measured
        } else {
            320.0
        }
    };
    let segment_width = (width - 8.0) / 3.0;

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · primitive"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Liquid" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The goo primitive: an SVG filter that fuses its children's silhouettes, and a box that morphs between positions inside it. The pill below carries the active label; its filtered silhouette is the rect the container draws in the same frame."
                }
            }
            div { class: "flex min-h-40 w-full items-center justify-center",
                div { class: "w-full max-w-sm",
                    Liquid {
                        blur: 6.0,
                        contrast: 18.0,
                        fill: "var(--background)".to_string(),
                        edge_color: "var(--foreground)".to_string(),
                        edge_opacity: 0.08,
                        edge_width: 1.0,
                        class: Some("rounded-2xl border border-border bg-muted p-1".to_string()),
                        div {
                            "data-beui-width": width_stream.attr(),
                            class: "relative grid h-10 grid-cols-3 items-center",
                            for (index, label) in SEGMENTS.iter().enumerate() {
                                button {
                                    key: "{label}",
                                    r#type: "button",
                                    class: cn!(
                                        "relative z-20 h-10 rounded-xl text-sm font-medium transition-colors",
                                        if active() == index { "text-foreground" } else { "text-muted-foreground hover:text-foreground" },
                                    ),
                                    onclick: move |_| active.set(index),
                                    "{label}"
                                }
                            }
                            LiquidItem {
                                x: active() as f64 * segment_width,
                                y: 0.0,
                                width: segment_width,
                                height: 40.0,
                                radius: Some(12.0),
                                class: Some("pointer-events-none flex items-center justify-center".to_string()),
                                span { class: "text-sm font-medium text-foreground",
                                    "{SEGMENTS[active()]}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

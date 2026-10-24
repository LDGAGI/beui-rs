//! `AspectRatio` — the beUI preview, same copy and structure (the ratio picker
//! is plain buttons here; beUI's uses `Tabs`, which is another lane's port).

use beui::prelude::*;
use beui::{AspectRatio, AspectRatioImage};

const RATIOS: [(&str, f64); 9] = [
    ("21:9", 21.0 / 9.0),
    ("16:9", 16.0 / 9.0),
    ("3:2", 3.0 / 2.0),
    ("4:3", 4.0 / 3.0),
    ("1:1", 1.0),
    ("4:5", 4.0 / 5.0),
    ("3:4", 3.0 / 4.0),
    ("2:3", 2.0 / 3.0),
    ("9:16", 9.0 / 16.0),
];

#[component]
pub fn AspectRatioPage() -> Element {
    let mut label = use_signal(|| "16:9".to_string());
    let ratio = RATIOS
        .iter()
        .find(|(name, _)| *name == label())
        .map(|(_, ratio)| *ratio)
        .unwrap_or(16.0 / 9.0);
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · media"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "AspectRatio" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A frame with a fixed ratio and an image that covers it. Motion's nested layout projection is a known gap in this port."
                }
            }
            div { class: "mx-auto w-full max-w-xl min-w-0 px-4 py-6",
                div { class: "flex flex-wrap justify-center gap-1 rounded-full bg-muted p-1",
                    for (name, _) in RATIOS {
                        button {
                            key: "{name}",
                            r#type: "button",
                            "data-ratio": "{name}",
                            class: cn!(
                                "min-h-9 rounded-full px-3 text-xs font-medium",
                                if label() == name { "bg-background text-foreground" } else { "text-muted-foreground" },
                            ),
                            onclick: move |_| label.set(name.to_string()),
                            "{name}"
                        }
                    }
                }
                div {
                    role: "tabpanel",
                    tabindex: "0",
                    class: "mt-6 flex h-[25rem] w-full items-center justify-center rounded-2xl outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    AspectRatio {
                        ratio,
                        class: "bg-muted",
                        style: format!("max-width:{}px;border-radius:16px", (360.0 * ratio).min(440.0)),
                        AspectRatioImage {
                            src: "https://images.unsplash.com/photo-1470770841072-f978cf4d019e?auto=format&fit=crop&w=1000&q=80",
                            alt: "A lakeside cabin beneath mountains, reflected in the water",
                            width: 1000.0,
                            height: 667.0,
                        }
                    }
                }
            }
        }
    }
}

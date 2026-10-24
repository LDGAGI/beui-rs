//! `Parallax` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::Parallax;

#[component]
pub fn ParallaxPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · scroll"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Parallax" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "On a real page this tracks the viewport. Here it is scoped to the box. Scroll inside the box: the background drifts against the scroll, the label and avatar drift with it at different speeds."
                }
            }
            div {
                id: "parallax-root",
                class: "relative h-[600px] w-full max-w-2xl overflow-y-auto scrollbar-hide rounded-2xl border border-border bg-card",
                div { class: "flex h-80 items-center justify-center text-sm text-muted-foreground",
                    "Scroll down ↓"
                }
                div { class: "relative h-96 overflow-hidden",
                    Parallax {
                        container: Some("#parallax-root".to_string()),
                        speed: -0.6,
                        class: "absolute inset-x-0 -top-1/4 h-[150%]",
                        img {
                            src: "https://picsum.photos/seed/beui-parallax/800/600",
                            alt: "",
                            class: "size-full object-cover",
                        }
                    }
                    Parallax {
                        container: Some("#parallax-root".to_string()),
                        speed: 0.5,
                        class: "absolute inset-0 grid place-items-center",
                        span { class: "rounded-full bg-background/85 px-5 py-2 text-base font-medium text-foreground backdrop-blur",
                            "Parallax"
                        }
                    }
                    Parallax {
                        container: Some("#parallax-root".to_string()),
                        speed: 0.9,
                        class: "absolute bottom-4 right-4",
                        img {
                            src: "https://picsum.photos/seed/beui-avatar/120/120",
                            alt: "",
                            class: "size-12 rounded-full border-2 border-background object-cover shadow-lg",
                        }
                    }
                }
                div { class: "flex h-80 items-center justify-center text-sm text-muted-foreground",
                    "↑ Scroll up"
                }
            }
        }
    }
}

//! `SlideActionButton` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::SlideActionButton;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

#[component]
pub fn SlideActionButtonPage() -> Element {
    let continued = use_signal(|| false);
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · control"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "SlideActionButton" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Drag the thumb past 82% of its travel to confirm. The drag runs through beui-core::drag with zero elasticity and no momentum; fill, label fade and the tick path all derive from the offset."
                }
            }
            div { class: "flex flex-col items-center gap-3",
                SlideActionButton {
                    complete_label: "Ready".to_string(),
                    on_complete: Some(EventHandler::new({
                        let mut continued = continued;
                        move |_| {
                            continued.set(true);
                            let mut continued = continued;
                            spawn(async move {
                                let _ = Time::delay(Duration::from_millis(1800)).await;
                                continued.set(false);
                            });
                        }
                    })),
                    "Slide to continue"
                }
                p { aria_live: "polite", class: "h-4 text-xs text-muted-foreground",
                    if continued() { "Action completed" } else { "Drag the arrow to the end" }
                }
            }
        }
    }
}

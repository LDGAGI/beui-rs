//! `HoldActionButton` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{HoldActionButton, HoldActionType};
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

#[component]
pub fn HoldActionButtonPage() -> Element {
    let confirmed = use_signal(|| false);
    let confirm = {
        let mut confirmed = confirmed;
        move || {
            confirmed.set(true);
            let mut confirmed = confirmed;
            spawn(async move {
                let _ = Time::delay(Duration::from_millis(1800)).await;
                confirmed.set(false);
            });
        }
    };
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · button"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "HoldActionButton" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Hold to complete: the liquid fill rises (or sweeps) over the hold duration on a linear transition, with a looping wave. Pointer capture measures leaving the button; a touch gets no leave reading, so the pointer type is checked."
                }
            }
            div { class: "flex flex-col items-center justify-center gap-3",
                HoldActionButton {
                    on_hold_complete: Some(EventHandler::new({
                        let mut confirm = confirm;
                        move |_| confirm()
                    })),
                    "Hold for vertical fill"
                }
                HoldActionButton {
                    r#type: HoldActionType::Horizontal,
                    on_hold_complete: Some(EventHandler::new({
                        let mut confirm = confirm;
                        move |_| confirm()
                    })),
                    "Hold for horizontal fill"
                }
                p { aria_live: "polite", class: "h-4 text-xs text-muted-foreground",
                    if confirmed() { "Action confirmed" } else { "Release early to cancel" }
                }
            }
        }
    }
}

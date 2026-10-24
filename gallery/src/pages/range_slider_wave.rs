//! `WaveSlider` — the beUI preview: a controlled gain slider.

use beui::prelude::*;
use beui::WaveSlider;

#[component]
pub fn RangeSliderWavePage() -> Element {
    let mut value = use_signal(|| 45.0f64);

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · range-slider"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "WaveSlider" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "An equalizer slider. Bars rise into a crest around the handle and fall back as it passes, so the value reads as a travelling wave."
                }
            }
            div { class: "flex min-h-40 w-full items-center justify-center",
                div { class: "flex w-full max-w-md flex-col gap-2",
                    div { class: "flex items-center justify-between text-sm text-muted-foreground",
                        span { "Gain" }
                        span { class: "tabular-nums text-foreground", "{value()}" }
                    }
                    WaveSlider {
                        value: Some(value()),
                        on_value_change: Some(EventHandler::new(move |next: f64| value.set(next))),
                        aria_label: Some("Gain".to_string()),
                    }
                }
            }
        }
    }
}

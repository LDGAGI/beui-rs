//! `NumberTicker` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::{locale_integer, NumberTicker};
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

#[component]
pub fn NumberTickerPage() -> Element {
    let mut value = use_signal(|| 48273.0f64);
    use_future(move || async move {
        let mut seed = 1u64;
        loop {
            if Time::delay(Duration::from_millis(2500)).await.is_err() {
                return;
            }
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let bump = ((seed >> 33) % 50) as f64;
            value += bump;
        }
    });

    let format = Callback::new(|n: f64| locale_integer(n));
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · number"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "NumberTicker" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Digits roll on a 0–9 column keyed by place value, so a changing digit keeps its identity and growing numbers add glyphs on the left."
                }
            }
            div { class: "flex flex-col items-center gap-3",
                p { class: "text-xs text-muted-foreground", "Active users" }
                NumberTicker {
                    value: value(),
                    class: "text-4xl font-semibold tracking-tight text-foreground tabular-nums",
                    format,
                }
                p { class: "text-xs text-muted-foreground", "live · updates every 2.5s" }
            }
        }
    }
}

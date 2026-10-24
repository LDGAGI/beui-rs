//! `DynamicIsland` — the beUI preview, same copy and structure (plain buttons
//! instead of `Button`, another lane's port).

use beui::icons::{Music, Phone, PhoneOff, Timer};
use beui::prelude::*;
use beui::{DynamicIsland, DynamicIslandView, NumberTicker};
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

const EQ_KEYFRAMES: &str =
    "@keyframes beui-eq-bar{0%{transform:scaleY(0.4)}25%{transform:scaleY(1)}\
     50%{transform:scaleY(0.55)}75%{transform:scaleY(0.9)}100%{transform:scaleY(0.4)}}";

const BAR_DELAYS: [f64; 4] = [0.0, 0.18, 0.09, 0.27];

fn format_clock(total: f64) -> String {
    let total = total.round() as i64;
    format!("{}:{:02}", total / 60, total % 60)
}

#[component]
fn EqBars(reduce: bool) -> Element {
    rsx! {
        span { class: "flex h-4 items-end gap-0.5", aria_hidden: "true",
            for delay in BAR_DELAYS {
                span {
                    key: "{delay}",
                    class: "h-full w-0.5 origin-bottom rounded-full bg-(--color-success)",
                    style: if reduce {
                        "transform:scaleY(0.6)".to_string()
                    } else {
                        format!("transform:scaleY(0.6);animation:beui-eq-bar 1.1s ease-in-out infinite;animation-delay:{delay}s")
                    },
                }
            }
        }
    }
}

#[component]
pub fn DynamicIslandPage() -> Element {
    let mut view = use_signal(|| None::<String>);
    let mut seconds = use_signal(|| 154.0f64);
    let reduce = use_reduced_motion();

    use_future(move || async move {
        loop {
            if Time::delay(Duration::from_millis(1000)).await.is_err() {
                return;
            }
            if view().as_deref() == Some("timer") {
                seconds.set((seconds() - 1.0).max(0.0));
            }
        }
    });

    let compact = rsx! {
        span { class: "h-1.5 w-1.5 rounded-full bg-(--color-success)" }
        span { "9:41" }
    };
    let seconds_now = seconds();
    let format = Callback::new(format_clock);

    rsx! {
        style { dangerous_inner_html: EQ_KEYFRAMES }
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "DynamicIsland" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A shell that springs its real width and height to the active view's natural size (0.8s, bounce 0.2/0.35). The radius stays 32px and the browser clamps it, so the pill-to-rect morph falls out of the resize."
                }
            }
            div { class: "flex w-full flex-col items-center gap-4",
                div { class: "flex h-32 w-full items-start justify-center pt-2",
                    DynamicIsland { view: view(), compact: Some(compact),
                        DynamicIslandView { id: "call", class: "gap-4",
                            div { class: "flex flex-col",
                                span { class: "text-[10px] uppercase tracking-wider opacity-60", "Incoming call" }
                                span { class: "text-sm font-semibold", "Saurabh" }
                            }
                            div { class: "flex items-center gap-2",
                                button {
                                    r#type: "button",
                                    aria_label: "Decline",
                                    class: "flex h-8 w-8 items-center justify-center rounded-full bg-destructive text-white",
                                    onclick: move |_| view.set(None),
                                    PhoneOff { class: Some("h-3.5 w-3.5".to_string()) }
                                }
                                button {
                                    r#type: "button",
                                    aria_label: "Accept",
                                    class: "flex h-8 w-8 items-center justify-center rounded-full bg-(--color-success) text-white",
                                    onclick: move |_| view.set(None),
                                    Phone { class: Some("h-3.5 w-3.5".to_string()) }
                                }
                            }
                        }
                        DynamicIslandView { id: "timer", class: "gap-3",
                            Timer { class: Some("h-4 w-4 text-(--color-warning)".to_string()) }
                            span { class: "text-[10px] uppercase tracking-wider opacity-60", "Timer" }
                            NumberTicker {
                                value: seconds_now,
                                start_on_view: false,
                                duration: 0.5,
                                format,
                                class: "text-sm font-semibold",
                            }
                        }
                        DynamicIslandView { id: "music", class: "gap-3",
                            span { class: "flex h-7 w-7 items-center justify-center rounded-lg bg-background/15",
                                Music { class: Some("h-3.5 w-3.5".to_string()) }
                            }
                            div { class: "flex flex-col text-left",
                                span { class: "text-xs font-semibold leading-tight", "Midnight City" }
                                span { class: "text-[10px] opacity-60", "M83" }
                            }
                            EqBars { reduce: reduce() }
                        }
                    }
                }
                div { class: "flex flex-wrap items-center justify-center gap-2",
                    button {
                        r#type: "button",
                        class: "inline-flex h-8 items-center rounded-full border border-border bg-card px-3 text-xs font-medium text-foreground",
                        onclick: move |_| view.set(Some("call".to_string())),
                        "Call"
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex h-8 items-center rounded-full border border-border bg-card px-3 text-xs font-medium text-foreground",
                        onclick: move |_| {
                            seconds.set(154.0);
                            view.set(Some("timer".to_string()));
                        },
                        "Timer"
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex h-8 items-center rounded-full border border-border bg-card px-3 text-xs font-medium text-foreground",
                        onclick: move |_| view.set(Some("music".to_string())),
                        "Music"
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex h-8 items-center rounded-full px-3 text-xs font-medium text-muted-foreground",
                        onclick: move |_| view.set(None),
                        "Dismiss"
                    }
                }
            }
        }
    }
}

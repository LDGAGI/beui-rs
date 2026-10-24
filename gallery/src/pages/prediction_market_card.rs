//! `PredictionMarketCard` — the beUI preview: two listings with outcome CTAs.
//!
//! The source's preview loads remote team logos; this renders initials on the
//! outcome color instead, so the page stays offline.

use beui::prelude::*;
use beui::{MarketSide, PredictionMarketCard, PredictionMarketCardOutcome};

fn initials(label: &str, color: &str) -> Element {
    rsx! {
        span {
            class: "flex size-full items-center justify-center text-xs font-semibold text-white",
            style: "background-color:{color}",
            "{label}"
        }
    }
}

struct MarketOutcome {
    id: &'static str,
    label: &'static str,
    probability: f64,
    color: &'static str,
}

struct Market {
    title: &'static str,
    category: &'static str,
    status: &'static str,
    volume: &'static str,
    live: bool,
    history: Vec<f64>,
    outcomes: Vec<MarketOutcome>,
}

#[component]
pub fn PredictionMarketCardPage() -> Element {
    let mut picked = use_signal(|| "—".to_string());

    let markets = vec![
        Market {
            title: "Boston Celtics vs. Los Angeles Lakers",
            category: "Basketball",
            status: "Sunday · 7:30 PM",
            volume: "$2.4M",
            live: false,
            history: vec![
                12.0, 18.0, 15.0, 26.0, 20.0, 32.0, 29.0, 38.0, 34.0, 48.0, 43.0, 58.0,
            ],
            outcomes: vec![
                MarketOutcome {
                    id: "boston",
                    label: "Celtics",
                    probability: 0.51,
                    color: "#34d399",
                },
                MarketOutcome {
                    id: "lakers",
                    label: "Lakers",
                    probability: 0.49,
                    color: "#fbbf24",
                },
            ],
        },
        Market {
            title: "Tampa Bay Rays vs. Atlanta Braves",
            category: "MLB",
            status: "Mid 6th",
            volume: "$458K",
            live: true,
            history: vec![
                8.0, 12.0, 10.0, 19.0, 15.0, 28.0, 23.0, 34.0, 30.0, 40.0, 36.0, 45.0,
            ],
            outcomes: vec![
                MarketOutcome {
                    id: "rays",
                    label: "Rays",
                    probability: 0.46,
                    color: "#60a5fa",
                },
                MarketOutcome {
                    id: "braves",
                    label: "Braves",
                    probability: 0.54,
                    color: "#f472b6",
                },
            ],
        },
    ];

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks · prediction-market"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "PredictionMarketCard" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A listing surface with outcome CTAs and an independent bookmark toggle. The odds button rolls from its price to the side on hover, and the sparkline is drawn from the volume history."
                }
                p { class: "text-xs text-muted-foreground",
                    "Picked: " span { class: "font-mono text-foreground", "{picked()}" }
                }
            }
            div { class: "grid gap-4 sm:grid-cols-2",
                for market in markets {
                    PredictionMarketCard {
                        key: "{market.title}",
                        title: market.title.to_string(),
                        category: Some(market.category.to_string()),
                        status: Some(market.status.to_string()),
                        volume: Some(market.volume.to_string()),
                        live: market.live,
                        volume_history: Some(market.history.clone()),
                        icon: Some(initials(&market.title[..1], "#334155")),
                        outcomes: market
                            .outcomes
                            .iter()
                            .map(|outcome| {
                                PredictionMarketCardOutcome::new(outcome.id, outcome.label, outcome.probability)
                                    .color(outcome.color)
                                    .icon(initials(outcome.label, outcome.color))
                            })
                            .collect(),
                        on_outcome_click: Some(EventHandler::new(move |selection: beui::PredictionMarketSelection| {
                            let side = match selection.side {
                                MarketSide::Yes => "yes",
                                MarketSide::No => "no",
                            };
                            picked.set(format!("{} {side}", selection.outcome_id));
                        })),
                    }
                }
            }
        }
    }
}

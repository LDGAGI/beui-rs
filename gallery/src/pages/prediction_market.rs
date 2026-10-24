//! `PredictionMarket` — the beUI preview: a controlled ticket on a Yes/No market.

use beui::prelude::*;
use beui::{PredictionMarket, PredictionMarketOrderValue, PredictionMarketOutcome};
use std::collections::HashMap;

#[component]
pub fn PredictionMarketPage() -> Element {
    let mut order = use_signal(|| PredictionMarketOrderValue {
        mode: beui::PredictionMarketMode::Buy,
        outcome_id: "yes".to_string(),
        amount: "115".to_string(),
    });

    let outcomes = vec![
        PredictionMarketOutcome::new("yes", "Yes", 0.167),
        PredictionMarketOutcome::new("no", "No", 0.834),
    ];
    let positions = HashMap::from([("yes".to_string(), 125.0), ("no".to_string(), 48.0)]);

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks · prediction-market"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "PredictionMarket" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A trade ticket. The quote clamps the price to 0.01–0.99, derives shares and payout per side, and fails in order on the minimum, the balance and the position; every digit of the amount arrives out of a blur."
                }
            }
            div { class: "flex w-full items-center justify-center",
                PredictionMarket {
                    outcomes: Some(outcomes),
                    value: Some(order()),
                    on_value_change: Some(EventHandler::new(move |next: PredictionMarketOrderValue| order.set(next))),
                    balance: 500.0,
                    positions: Some(positions),
                    quick_amounts: vec![1.0, 5.0, 10.0, 100.0],
                }
            }
        }
    }
}

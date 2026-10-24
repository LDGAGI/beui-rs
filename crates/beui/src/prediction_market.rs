//! `motion/prediction-market.tsx` — a trade ticket: buy/sell modes, outcome
//! pills, an animated amount, and the payout ticker.
//!
//! All the maths is the source's: `buildQuote`'s clamped price
//! (`max(0.01, min(0.99, price))`), `shares = amount / price` for a buy and
//! `amount` for a sell, `payout = shares` for a buy and `amount * price` for a
//! sell, and its four failures in order ("Enter an amount", the minimum, the
//! balance, the position). So are `sanitizeAmount` (digits and one dot, two
//! decimals), the `keyedAmountChars` occurrence keys, the four
//! `amountInputSize` / four `payoutTickerSize` buckets, the 180ms `EASE_OUT`
//! digit roll, the 650ms fill, the shake on an invalid submit, and the
//! 0.18/0.45s payout ticker.
//!
//! Substitutions, all another lane's: `Tabs` becomes plain buttons keeping the
//! same classes (the sliding indicator is not reproduced), `StatefulButton`
//! becomes a button with the same status-driven label, and the per-character
//! exit of the amount roll is dropped (it would need one layer per character —
//! the digits still arrive out of a blur on their own keyframes).

use crate::icons::{Banknote, ChevronDown};
use crate::number_ticker::NumberTicker;
use crate::prelude::*;
use beui_core::platform::json;
use std::collections::HashMap;

const ANIMATIONS: &str = "\
@keyframes beui-amount-in{\
0%{opacity:0;transform:translateY(18px);filter:blur(10px)}\
100%{opacity:1;transform:translateY(0);filter:blur(0)}}\
@keyframes beui-trade-shake{\
0%,100%{transform:translateX(0)}\
15%{transform:translateX(-5px)}\
35%{transform:translateX(5px)}\
55%{transform:translateX(-3px)}\
75%{transform:translateX(3px)}\
90%{transform:translateX(-1px)}}";

/// Which side of the book the order is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PredictionMarketMode {
    #[default]
    Buy,
    Sell,
}

impl PredictionMarketMode {
    fn label(self) -> &'static str {
        match self {
            Self::Buy => "Buy",
            Self::Sell => "Sell",
        }
    }

    fn is_buy(self) -> bool {
        self == Self::Buy
    }
}

/// One tradable outcome.
#[derive(Clone, PartialEq)]
pub struct PredictionMarketOutcome {
    pub id: String,
    pub label: String,
    /// Price between 0 and 1.
    pub price: f64,
}

impl PredictionMarketOutcome {
    /// An outcome at a price.
    pub fn new(id: impl Into<String>, label: impl Into<String>, price: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            price,
        }
    }
}

/// The order the ticket is composing.
#[derive(Clone, Debug, PartialEq)]
pub struct PredictionMarketOrderValue {
    pub mode: PredictionMarketMode,
    pub outcome_id: String,
    pub amount: String,
}

/// What the ticket says about the current order.
#[derive(Clone, Debug, PartialEq)]
pub struct PredictionMarketQuote {
    pub valid: bool,
    pub amount: f64,
    pub price: f64,
    pub shares: f64,
    pub payout: f64,
    pub error: Option<String>,
}

/// The trade button's state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TradeStatus {
    #[default]
    Idle,
    Placing,
    Filled,
}

/// `sanitizeAmount`: digits and one dot, two decimals.
fn sanitize_amount(value: &str) -> String {
    let normalized: String = value
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = normalized.splitn(2, '.');
    let whole = parts.next().unwrap_or("").to_string();
    match parts.next() {
        None => whole,
        Some(decimal) => {
            let decimal: String = decimal.chars().filter(|c| c.is_ascii_digit()).collect();
            format!("{whole}.{}", &decimal[..decimal.len().min(2)])
        }
    }
}

fn parse_amount(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

/// `Intl.NumberFormat("en-US", { style: "currency", currency: "USD", maximumFractionDigits })`:
/// group the whole part, then at most that many decimals with trailing zeros
/// dropped (so 1 is `$1`, 1234.5 is `$1,234.5` and 1234.567 is `$1,234.57`).
fn format_currency(value: f64, maximum_fraction_digits: usize) -> String {
    let negative = value < 0.0;
    let scale = 10f64.powi(maximum_fraction_digits as i32);
    let scaled = (value.abs() * scale).round();
    let whole = (scaled / scale).trunc() as u64;
    let digits = if scale >= 1.0 {
        (scaled as u64) % (scale as u64)
    } else {
        0
    };

    let mut fraction = String::new();
    if maximum_fraction_digits > 0 && digits > 0 {
        let raw = format!("{digits:0width$}", width = maximum_fraction_digits);
        let trimmed = raw.trim_end_matches('0');
        if !trimmed.is_empty() {
            fraction = format!(".{trimmed}");
        }
    }

    let text = whole.to_string();
    let mut grouped = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index > 0 && (text.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }

    format!("{}{grouped}{fraction}", if negative { "-$" } else { "$" })
}

fn format_compact_currency(value: f64) -> String {
    if value >= 100.0 {
        format_currency(value, 0)
    } else {
        format_currency(value, if value.fract() == 0.0 { 0 } else { 2 })
    }
}

fn format_cents(value: f64) -> String {
    let cents = value * 100.0;
    if cents.fract() == 0.0 {
        format!("{}¢", cents.trunc() as i64)
    } else {
        format!("{cents:.1}¢")
    }
}

/// `buildQuote`.
fn build_quote(
    order: &PredictionMarketOrderValue,
    outcome: &PredictionMarketOutcome,
    balance: f64,
    position: f64,
    min_trade: f64,
) -> PredictionMarketQuote {
    let amount = parse_amount(&order.amount);
    let price = outcome.price.clamp(0.01, 0.99);
    let shares = if order.mode.is_buy() {
        amount / price
    } else {
        amount
    };
    let payout = if order.mode.is_buy() {
        shares
    } else {
        amount * price
    };

    let invalid = |error: &str| PredictionMarketQuote {
        valid: false,
        amount,
        price,
        shares,
        payout,
        error: Some(error.to_string()),
    };

    if amount <= 0.0 {
        return invalid("Enter an amount");
    }
    if order.mode.is_buy() && amount < min_trade {
        return invalid(&format!("Minimum {}", format_compact_currency(min_trade)));
    }
    if order.mode.is_buy() && amount > balance {
        return invalid("Insufficient balance");
    }
    if !order.mode.is_buy() && amount > position {
        return invalid("Not enough shares");
    }

    PredictionMarketQuote {
        valid: true,
        amount,
        price,
        shares,
        payout,
        error: None,
    }
}

/// The amount's font size, by digit count.
fn amount_input_size(value: &str) -> &'static str {
    let length = value.chars().filter(|c| c.is_ascii_digit()).count();
    if length >= 10 {
        "text-3xl sm:text-4xl"
    } else if length >= 8 {
        "text-4xl sm:text-5xl"
    } else if length >= 6 {
        "text-[44px] sm:text-[56px]"
    } else {
        "text-5xl sm:text-6xl"
    }
}

/// The payout's font size, by its rendered length.
fn payout_ticker_size(value: f64) -> &'static str {
    let length = format_currency(value, 2).chars().count();
    if length >= 16 {
        "text-xl sm:text-2xl"
    } else if length >= 13 {
        "text-2xl"
    } else if length >= 10 {
        "text-3xl"
    } else {
        "text-4xl"
    }
}

/// Props for [`PredictionMarket`].
#[derive(Props, Clone, PartialEq)]
pub struct PredictionMarketProps {
    #[props(default)]
    pub outcomes: Option<Vec<PredictionMarketOutcome>>,
    #[props(default)]
    pub value: Option<PredictionMarketOrderValue>,
    #[props(default)]
    pub default_value: Option<PredictionMarketOrderValue>,
    #[props(default)]
    pub on_value_change: Option<EventHandler<PredictionMarketOrderValue>>,
    #[props(default)]
    pub on_trade: Option<EventHandler<(PredictionMarketOrderValue, PredictionMarketQuote)>>,
    #[props(default)]
    pub on_sign_in: Option<EventHandler<()>>,
    #[props(default = true)]
    pub authenticated: bool,
    #[props(default = "Market".to_string())]
    pub order_type_label: String,
    #[props(default = 500.0)]
    pub balance: f64,
    /// Open shares per outcome id. Defaults to `{ up: 24, down: 16 }`.
    #[props(default)]
    pub positions: Option<HashMap<String, f64>>,
    #[props(default = vec![10.0, 50.0, 100.0, 500.0])]
    pub quick_amounts: Vec<f64>,
    #[props(default = 1.0)]
    pub min_trade: f64,
    #[props(default)]
    pub class: Option<String>,
}

/// A prediction-market trade ticket.
#[component]
pub fn PredictionMarket(props: PredictionMarketProps) -> Element {
    let PredictionMarketProps {
        outcomes,
        value,
        default_value,
        on_value_change,
        on_trade,
        on_sign_in,
        authenticated,
        order_type_label,
        balance,
        positions,
        quick_amounts,
        min_trade,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let outcomes = outcomes.unwrap_or_else(|| {
        vec![
            PredictionMarketOutcome::new("up", "Up", 0.09),
            PredictionMarketOutcome::new("down", "Down", 0.91),
        ]
    });
    let positions = positions
        .unwrap_or_else(|| HashMap::from([("up".to_string(), 24.0), ("down".to_string(), 16.0)]));

    let initial = default_value.unwrap_or_else(|| PredictionMarketOrderValue {
        mode: PredictionMarketMode::Buy,
        outcome_id: outcomes
            .first()
            .map(|outcome| outcome.id.clone())
            .unwrap_or_default(),
        amount: String::new(),
    });
    let mut internal_order = use_signal(|| initial.clone());
    let order = value.clone().unwrap_or(internal_order());
    let mut status = use_signal(TradeStatus::default);
    let mut shake_key = use_signal(|| 0u32);

    let selected = outcomes
        .iter()
        .find(|outcome| outcome.id == order.outcome_id)
        .cloned()
        .unwrap_or_else(|| outcomes[0].clone());
    let position = positions.get(&selected.id).copied().unwrap_or(0.0);
    let quote = build_quote(&order, &selected, balance, position, min_trade);

    let set_order = EventHandler::new({
        let controlled = value.is_some();
        move |next: PredictionMarketOrderValue| {
            status.set(TradeStatus::Idle);
            if !controlled {
                internal_order.set(next.clone());
            }
            if let Some(handler) = on_value_change {
                handler.call(next);
            }
        }
    });
    let submit = EventHandler::new({
        let order = order.clone();
        let quote = quote.clone();
        move |_: ()| {
            if !authenticated {
                if let Some(handler) = on_sign_in {
                    handler.call(());
                }
                return;
            }
            if !quote.valid {
                shake_key += 1;
                return;
            }
            status.set(TradeStatus::Placing);
            let order = order.clone();
            let quote = quote.clone();
            spawn(async move {
                let _ = json::<bool>(
                    "await new Promise((resolve) => setTimeout(resolve, 650));\nreturn true;",
                )
                .await;
                status.set(TradeStatus::Filled);
                if let Some(handler) = on_trade {
                    handler.call((order, quote));
                }
            });
        }
    });

    let input_size = amount_input_size(&order.amount);
    let payout_size = payout_ticker_size(quote.payout);
    let display_value = if order.amount.is_empty() {
        "0".to_string()
    } else {
        order.amount.clone()
    };
    // `keyedAmountChars`: the occurrence index keeps repeated digits distinct as
    // keys, so a change re-mounts the characters that moved.
    let mut seen: HashMap<char, usize> = HashMap::new();
    let chars: Vec<(String, char)> = display_value
        .chars()
        .map(|character| {
            let count = seen.entry(character).or_insert(0);
            let key = format!("{character}-{count}");
            *count += 1;
            (key, character)
        })
        .collect();
    let amount_chars = chars.len().max(1);
    let amount_style = format!("--amount-chars:{amount_chars}");
    let amount_width = format!(
        "width:calc((var(--amount-chars) + 1) * 0.62em);max-width:260px;min-width:0.8em;{amount_style}"
    );
    let shake = if shake_key() > 0 && !reduce_value {
        format!(
            "animation:beui-trade-shake 380ms ease-out;--shake:{}",
            shake_key()
        )
    } else {
        String::new()
    };
    let action_state = match status() {
        TradeStatus::Placing => "loading",
        TradeStatus::Filled => "success",
        TradeStatus::Idle if quote.valid => "idle",
        _ => "error",
    };
    let action_label = match status() {
        TradeStatus::Placing => "Trading".to_string(),
        TradeStatus::Filled => "Trade filled".to_string(),
        TradeStatus::Idle if !quote.valid => quote
            .error
            .clone()
            .unwrap_or_else(|| "Enter an amount".to_string()),
        _ => "Trade".to_string(),
    };
    let action_class = cn!(
        "h-12 w-full rounded-2xl text-base font-semibold transition-colors",
        if quote.valid || status() != TradeStatus::Idle {
            "bg-primary text-primary-foreground hover:bg-primary/90"
        } else {
            "bg-muted text-muted-foreground"
        },
    );
    let root_class = cn!(
        "w-full max-w-[400px] overflow-hidden rounded-3xl border border-border bg-background",
        class,
    );

    rsx! {
        style { dangerous_inner_html: ANIMATIONS }
        div { class: root_class,
            div { class: "border-b border-border/80 px-4 pt-4",
                div { class: "flex items-end justify-between gap-4",
                    // `Tabs variant="underline"`, as buttons.
                    div { class: "flex shrink-0 items-end gap-5",
                        for mode in [PredictionMarketMode::Buy, PredictionMarketMode::Sell] {
                            button {
                                key: "{mode.label()}",
                                r#type: "button",
                                class: cn!(
                                    "relative px-0 pb-3 pt-0 text-2xl font-semibold transition-colors",
                                    if order.mode == mode { "text-foreground" } else { "text-muted-foreground" },
                                ),
                                onclick: {
                                                                        let order = order.clone();
                                    let outcome_id = order.outcome_id.clone();
                                    move |_| set_order.call(PredictionMarketOrderValue {
                                        mode,
                                        outcome_id: outcome_id.clone(),
                                        amount: String::new(),
                                    })
                                },
                                "{mode.label()}"
                                if order.mode == mode {
                                    span { class: "absolute inset-x-0 bottom-0 h-0.5 bg-foreground" }
                                }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        disabled: status() == TradeStatus::Placing,
                        class: "mb-3 inline-flex items-center gap-2 text-xl font-semibold text-foreground transition-opacity disabled:opacity-50",
                        "{order_type_label}"
                        ChevronDown { class: Some("h-5 w-5".to_string()) }
                    }
                }
            }

            div { class: "space-y-4 p-3",
                // `Tabs variant="pill"`, as buttons.
                div { class: "grid w-full grid-cols-2 gap-2 rounded-2xl bg-muted p-1.5",
                    for outcome in outcomes.iter() {
                        {
                            let selected_now = outcome.id == selected.id;
                            let is_no = matches!(outcome.label.to_lowercase().as_str(), "no" | "down");
                            let tone = if is_no {
                                if selected_now { "text-red-300" } else { "text-red-300/55" }
                            } else if selected_now {
                                "text-emerald-400 dark:text-emerald-300"
                            } else {
                                "text-muted-foreground"
                            };
                            rsx! {
                                button {
                                    key: "{outcome.id}",
                                    r#type: "button",
                                    class: cn!("relative h-14 w-full rounded-[1.35rem] px-0 py-0 text-base font-semibold transition-transform active:scale-[0.99]", tone),
                                    onclick: {
                                                                                let order = order.clone();
                                        let id = outcome.id.clone();
                                        move |_| set_order.call(PredictionMarketOrderValue {
                                            mode: order.mode,
                                            outcome_id: id.clone(),
                                            amount: order.amount.clone(),
                                        })
                                    },
                                    if selected_now {
                                        span {
                                            class: cn!(
                                                "absolute inset-0 rounded-[1.35rem]",
                                                if is_no { "bg-red-500/10 dark:bg-red-500/15" } else { "bg-emerald-500/20" },
                                            )
                                        }
                                    }
                                    span { class: "relative", "{outcome.label} {format_cents(outcome.price)}" }
                                }
                            }
                        }
                    }
                }

                div { class: "rounded-3xl bg-card p-4", style: shake,
                    div { class: "flex min-h-24 flex-col items-center justify-center gap-5 text-center",
                        label {
                            class: "mr-6 text-xl font-medium text-foreground",
                            if order.mode.is_buy() { "Amount" } else { "Shares" }
                        }
                        div { class: "w-full min-w-0",
                            div { class: "flex min-w-0 items-center justify-center overflow-hidden",
                                if order.mode.is_buy() {
                                    span {
                                        aria_hidden: "true",
                                        class: cn!("shrink-0 font-semibold leading-none tracking-normal text-muted-foreground/65 tabular-nums transition-[font-size] duration-200", input_size),
                                        "$"
                                    }
                                }
                                div { class: "relative min-w-0 shrink",
                                    input {
                                        value: "{order.amount}",
                                        disabled: status() == TradeStatus::Placing,
                                        placeholder: "0",
                                        inputmode: "decimal",
                                        aria_label: if order.mode.is_buy() { "Amount" } else { "Shares" },
                                        autocomplete: "off",
                                        class: cn!(
                                            "min-w-0 bg-transparent text-left font-semibold leading-none tracking-normal text-transparent outline-none tabular-nums caret-foreground transition-[font-size] duration-200 placeholder:text-transparent selection:bg-foreground/10 disabled:cursor-not-allowed",
                                            input_size,
                                        ),
                                        style: amount_width.clone(),
                                        oninput: {
                                                                                        let order = order.clone();
                                            move |event: Event<FormData>| {
                                                let amount = sanitize_amount(&event.value());
                                                set_order.call(PredictionMarketOrderValue { amount, ..order.clone() });
                                            }
                                        },
                                    }
                                    div {
                                        aria_hidden: "true",
                                        class: cn!(
                                            "pointer-events-none absolute inset-0 flex min-w-0 items-center justify-start overflow-hidden font-semibold leading-none tracking-normal text-foreground tabular-nums transition-[font-size] duration-200",
                                            if order.amount.is_empty() { "text-muted-foreground/55" } else { "" },
                                            input_size,
                                        ),
                                        style: amount_width.clone(),
                                        for (key, character) in chars {
                                            span {
                                                key: "{key}",
                                                class: "inline-block min-w-[0.55em] text-center will-change-[transform,opacity,filter]",
                                                style: if reduce_value {
                                                    String::new()
                                                } else {
                                                    "animation:beui-amount-in 180ms ease-out both".to_string()
                                                },
                                                "{character}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "mt-8 flex flex-wrap justify-center gap-2",
                        for amount in quick_amounts.iter().copied() {
                            button {
                                key: "quick-{amount}",
                                r#type: "button",
                                disabled: status() == TradeStatus::Placing,
                                class: "h-9 rounded-xl bg-background px-3.5 text-sm font-semibold text-foreground transition-[background-color,transform] duration-150 active:scale-95 disabled:pointer-events-none disabled:opacity-50",
                                onclick: {
                                                                        let order = order.clone();
                                    move |_| {
                                        let next = parse_amount(&order.amount) + amount;
                                        set_order.call(PredictionMarketOrderValue { amount: format_number(next), ..order.clone() });
                                    }
                                },
                                if order.mode.is_buy() { "+{format_compact_currency(amount)}" } else { "+{format_number(amount)}" }
                            }
                        }
                        button {
                            r#type: "button",
                            disabled: status() == TradeStatus::Placing,
                            class: "h-9 rounded-xl bg-background px-3.5 text-sm font-semibold text-foreground transition-[background-color,transform] duration-150 active:scale-95 disabled:pointer-events-none disabled:opacity-50",
                            onclick: {
                                                                let order = order.clone();
                                let mode = order.mode;
                                move |_| {
                                    let amount = if mode.is_buy() {
                                        format_number(balance.floor())
                                    } else if position.fract() == 0.0 {
                                        format!("{}", position.trunc() as i64)
                                    } else {
                                        format!("{position:.2}")
                                    };
                                    set_order.call(PredictionMarketOrderValue { amount, ..order.clone() });
                                }
                            },
                            "Max"
                        }
                    }
                }
            }

            if authenticated {
                div { class: "border-t border-border/80 px-4 py-4",
                    div { class: "mb-4 flex items-end justify-between gap-3",
                        div { class: "min-w-0 shrink",
                            div { class: "flex items-center gap-2 text-xl font-semibold text-foreground",
                                if order.mode.is_buy() { "To win" } else { "To receive" }
                                Banknote { class: Some("h-5 w-5 text-emerald-500".to_string()) }
                            }
                            p { class: "text-sm font-medium text-muted-foreground",
                                "Avg. Price {format_cents(quote.price)}"
                            }
                        }
                        NumberTicker {
                            value: quote.payout * 100.0,
                            start_on_view: false,
                            duration: 0.45,
                            stagger: 0.0,
                            blur: true,
                            locale: true,
                            format: Some(Callback::new(|cents: f64| format_currency(cents / 100.0, 2))),
                            class: Some(cn!(
                                "ml-auto min-w-0 shrink-0 justify-end whitespace-nowrap text-right font-semibold leading-none tracking-tight text-emerald-500 tabular-nums transition-[font-size] duration-200",
                                payout_size,
                            )),
                        }
                    }
                    button {
                        r#type: "button",
                        "data-trade-state": action_state,
                        class: action_class,
                        onclick: move |_| submit.call(()),
                        "{action_label}"
                    }
                }
            } else {
                div { class: "px-4 pb-5",
                    button {
                        r#type: "button",
                        "data-trade-state": "idle",
                        class: "h-14 w-full rounded-2xl bg-primary text-base font-semibold text-primary-foreground transition-colors hover:bg-primary/90",
                        onclick: move |_| submit.call(()),
                        "Connect"
                    }
                }
            }
        }
    }
}

/// A number for the amount field: no trailing zeros, at most two decimals.
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value.trunc() as i64)
    } else {
        format!("{value:.2}")
    }
}

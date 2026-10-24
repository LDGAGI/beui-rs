//! `charts/order-book/` — a snapshot-driven depth ladder.
//!
//! `prepare`, `summarizeDepth` and `buildOrderBook` are ported verbatim
//! (invalid/zero-size levels dropped, equal prices combined, cumulative totals,
//! best-price-first ordering, midpoint/spread). The inspected level and its
//! better neighbours are highlighted and totalled through a shared context.
//!
//! Differences from the source: the source animates each depth bar and
//! flashes changed quantities with motion; the port renders the resolved bar
//! width and skips the change flash. The `children` render function on
//! `OrderBookDepthSummary` is not ported.

use beui::prelude::*;

use crate::candlestick_chart::default_volume_format;

/// `OrderBookLevel`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
}

impl OrderBookLevel {
    pub fn new(price: f64, size: f64) -> Self {
        Self { price, size }
    }
}

/// A side of the book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

/// `DepthLevel`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthLevel {
    pub price: f64,
    pub size: f64,
    pub total: f64,
}

/// `DepthSummary`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthSummary {
    pub side: Side,
    pub levels: usize,
    pub size: f64,
    pub quote: f64,
    pub average_price: f64,
}

/// `OrderBookActiveLevel`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrderBookActiveLevel {
    pub side: Side,
    pub price: f64,
}

/// `buildOrderBook` result.
#[derive(Clone, Debug, PartialEq)]
pub struct OrderBookBuild {
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub bid_total: f64,
    pub ask_total: f64,
    pub max_total: f64,
    pub midpoint: Option<f64>,
    pub spread: Option<f64>,
}

fn prepare(levels: &[OrderBookLevel], side: Side, limit: usize) -> Vec<DepthLevel> {
    let mut prices: Vec<(f64, f64)> = Vec::new();
    for level in levels {
        if !level.price.is_finite()
            || !level.size.is_finite()
            || level.price <= 0.0
            || level.size <= 0.0
        {
            continue;
        }
        match prices.iter_mut().find(|(price, _)| *price == level.price) {
            Some(entry) => entry.1 += level.size,
            None => prices.push((level.price, level.size)),
        }
    }
    prices.sort_by(|a, b| match side {
        Side::Bid => b.0.total_cmp(&a.0),
        Side::Ask => a.0.total_cmp(&b.0),
    });
    prices.truncate(limit);
    let mut total = 0.0;
    prices
        .into_iter()
        .map(|(price, size)| {
            total += size;
            DepthLevel { price, size, total }
        })
        .collect()
}

/// `summarizeDepth`.
pub fn summarize_depth(levels: &[DepthLevel], side: Side) -> DepthSummary {
    let size: f64 = levels.iter().map(|level| level.size).sum();
    let quote: f64 = levels.iter().map(|level| level.size * level.price).sum();
    DepthSummary {
        side,
        levels: levels.len(),
        size,
        quote,
        average_price: if size != 0.0 { quote / size } else { 0.0 },
    }
}

/// `buildOrderBook`.
pub fn build_order_book(
    bids: &[OrderBookLevel],
    asks: &[OrderBookLevel],
    levels: f64,
) -> OrderBookBuild {
    let limit = if levels.is_finite() {
        levels.max(0.0).floor() as usize
    } else {
        9
    };
    let buy = prepare(bids, Side::Bid, limit);
    let sell = prepare(asks, Side::Ask, limit);
    let bid_total = buy.last().map(|level| level.total).unwrap_or(0.0);
    let ask_total = sell.last().map(|level| level.total).unwrap_or(0.0);
    let best_bid = buy.first().map(|level| level.price);
    let best_ask = sell.first().map(|level| level.price);
    let midpoint = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
        _ => None,
    };
    let spread = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => Some(ask - bid),
        _ => None,
    };
    OrderBookBuild {
        bids: buy,
        asks: sell,
        bid_total,
        ask_total,
        max_total: bid_total.max(ask_total),
        midpoint,
        spread,
    }
}

fn default_price(value: f64) -> String {
    format!("{value:.2}")
}

#[derive(Clone, Copy)]
struct OrderBookContext {
    book: Signal<OrderBookBuild>,
    active: Signal<Option<OrderBookActiveLevel>>,
    controlled: bool,
    on_active_level_change: Option<EventHandler<Option<OrderBookActiveLevel>>>,
    on_level_select: Option<EventHandler<(DepthLevel, Side)>>,
    price: Signal<Option<f64>>,
    base_symbol: Signal<String>,
    quote_symbol: Signal<String>,
    format_price: Option<Callback<f64, String>>,
    format_size: Option<Callback<f64, String>>,
}

impl OrderBookContext {
    fn price(&self, value: f64) -> String {
        match self.format_price {
            Some(callback) => callback.call(value),
            None => default_price(value),
        }
    }

    fn size(&self, value: f64) -> String {
        match self.format_size {
            Some(callback) => callback.call(value),
            None => default_volume_format(value),
        }
    }

    fn levels(&self, side: Side) -> Vec<DepthLevel> {
        match side {
            Side::Bid => (self.book)().bids,
            Side::Ask => (self.book)().asks,
        }
    }

    fn active_resolved(&self) -> Option<(Side, usize)> {
        let active = (self.active)()?;
        let index = self
            .levels(active.side)
            .iter()
            .position(|level| level.price == active.price)?;
        Some((active.side, index))
    }

    fn summary(&self) -> Option<DepthSummary> {
        let (side, index) = self.active_resolved()?;
        let levels = self.levels(side);
        Some(summarize_depth(&levels[..=index], side))
    }

    fn set_active(&self, level: Option<OrderBookActiveLevel>) {
        if !self.controlled {
            let mut active = self.active;
            active.set(level);
        }
        if let Some(handler) = self.on_active_level_change {
            handler.call(level);
        }
    }
}

/// `OrderBook`.
#[component]
pub fn OrderBook(
    bids: Vec<OrderBookLevel>,
    asks: Vec<OrderBookLevel>,
    #[props(default = 9.0)] levels: f64,
    #[props(default)] last_price: Option<f64>,
    #[props(default = "Order book".to_string())] label: String,
    #[props(default)] base_symbol: String,
    #[props(default = "USD".to_string())] quote_symbol: String,
    #[props(default)] format_price: Option<Callback<f64, String>>,
    #[props(default)] format_size: Option<Callback<f64, String>>,
    #[props(default)] active_level: Option<OrderBookActiveLevel>,
    #[props(default)] default_active_level: Option<OrderBookActiveLevel>,
    #[props(default)] on_active_level_change: Option<EventHandler<Option<OrderBookActiveLevel>>>,
    #[props(default)] on_level_select: Option<EventHandler<(DepthLevel, Side)>>,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let book = build_order_book(&bids, &asks, levels);
    let book_signal = use_signal(|| book);
    let mut internal_active = use_signal(|| default_active_level);
    let base_signal = use_signal(|| base_symbol.clone());
    let quote_signal = use_signal(|| quote_symbol.clone());
    let controlled = active_level.is_some();

    use_effect(use_reactive((&active_level,), move |(active_level,)| {
        if let Some(active_level) = active_level {
            internal_active.set(Some(active_level));
        }
    }));

    let midpoint = (book_signal)().midpoint;
    let price = last_price
        .filter(|value| value.is_finite() && *value > 0.0)
        .or(midpoint);
    let price_signal = use_signal(|| price);

    let context = OrderBookContext {
        book: book_signal,
        active: internal_active,
        controlled,
        on_active_level_change,
        on_level_select,
        price: price_signal,
        base_symbol: base_signal,
        quote_symbol: quote_signal,
        format_price,
        format_size,
    };
    use_context_provider(|| context);

    rsx! {
        section {
            "aria-label": "{label}",
            class: cn!(
                "w-full max-w-[560px] overflow-hidden rounded-2xl border border-border bg-background font-mono text-xs tabular-nums",
                class,
            ),
            if let Some(children) = children {
                {children}
            } else {
                OrderBookHeader {}
                OrderBookSide { side: Side::Ask }
                OrderBookSpread {}
                OrderBookSide { side: Side::Bid }
                OrderBookBalance {}
            }
        }
    }
}

/// `OrderBookHeader`.
#[component]
pub fn OrderBookHeader(#[props(default)] class: String) -> Element {
    let context = use_context::<OrderBookContext>();
    let base = (context.base_symbol)();
    let quote = (context.quote_symbol)();
    let price_label = if quote.is_empty() {
        "Price".to_string()
    } else {
        format!("Price ({quote})")
    };
    let size_label = if base.is_empty() {
        "Size".to_string()
    } else {
        format!("Size ({base})")
    };
    rsx! {
        div {
            "aria-hidden": "true",
            class: cn!(
                "grid grid-cols-3 py-3 text-[10px] uppercase tracking-wider text-muted-foreground",
                class,
            ),
            span { class: "px-4", "{price_label}" }
            span { class: "px-4 text-right", "{size_label}" }
            span { class: "px-4 text-right", "Total" }
        }
    }
}

/// `OrderBookSide`.
#[component]
pub fn OrderBookSide(side: Side, #[props(default)] class: String) -> Element {
    let context = use_context::<OrderBookContext>();
    let book = (context.book)();
    let max_total = book.max_total;
    let active = context.active_resolved();
    let rows: Vec<DepthLevel> = match side {
        Side::Ask => book.asks.iter().rev().copied().collect(),
        Side::Bid => book.bids.clone(),
    };
    let color = match side {
        Side::Ask => "text-rose-600 dark:text-rose-400",
        Side::Bid => "text-emerald-700 dark:text-emerald-400",
    };
    let aria = match side {
        Side::Ask => "Asks · sell orders",
        Side::Bid => "Bids · buy orders",
    };
    let base = (context.base_symbol)();
    let quote = (context.quote_symbol)();
    let empty_label = match side {
        Side::Ask => "No asks",
        Side::Bid => "No bids",
    };
    let row_count = rows.len();
    let on_level_select = context.on_level_select;

    rsx! {
        table {
            "aria-label": "{aria}",
            class: cn!("w-full table-fixed border-separate border-spacing-0", class),
            thead { class: "sr-only",
                tr {
                    th { scope: "col", "Price {quote}" }
                    th { scope: "col", "Size {base}" }
                    th { scope: "col", "Cumulative total {base}" }
                }
            }
            tbody {
                onpointerleave: move |_| context.set_active(None),
                if rows.is_empty() {
                    tr {
                        td { colspan: "3", class: "px-4 py-6 text-center text-muted-foreground", "{empty_label}" }
                    }
                } else {
                    for (index, row) in rows.iter().enumerate() {
                        {
                            let depth_index = match side {
                                Side::Ask => row_count - 1 - index,
                                Side::Bid => index,
                            };
                            let in_range = active.map(|(active_side, active_index)| {
                                active_side == side && depth_index <= active_index
                            }).unwrap_or(false);
                            let fraction = if max_total != 0.0 { row.total / max_total } else { 0.0 };
                            let on_select = on_level_select.map(|handler| {
                                let row = *row;
                                EventHandler::new(move |_| handler.call((row, side)))
                            });
                            let row_for_inspect = *row;
                            rsx! {
                                OrderBookDepthRow {
                                    key: "{row.price}",
                                    row: *row,
                                    fraction,
                                    entrance_index: depth_index,
                                    color: color.to_string(),
                                    in_range,
                                    on_inspect: Some(EventHandler::new(move |inspect: bool| {
                                        context.set_active(if inspect {
                                            Some(OrderBookActiveLevel { side, price: row_for_inspect.price })
                                        } else {
                                            None
                                        });
                                    })),
                                    on_select,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `OrderBookDepthRow`.
#[component]
fn OrderBookDepthRow(
    row: DepthLevel,
    fraction: f64,
    #[props(default)] entrance_index: usize,
    #[props(default)] color: String,
    #[props(default)] in_range: bool,
    #[props(default)] on_inspect: Option<EventHandler<bool>>,
    #[props(default)] on_select: Option<EventHandler<()>>,
) -> Element {
    let context = use_context::<OrderBookContext>();
    let reduce = use_reduced_motion()();
    let _ = entrance_index;
    let bar_style = format!(
        "transform:scaleX({fraction});opacity:1;{}",
        if reduce {
            ""
        } else {
            "transition:transform 320ms cubic-bezier(0.32,0.72,0,1);"
        }
    );
    let range_style = if in_range {
        "opacity:0.08;transition:opacity 150ms;"
    } else {
        "opacity:0;transition:opacity 150ms;"
    };
    let price_text = context.price(row.price);
    let size_text = context.size(row.size);
    let total_text = context.size(row.total);
    let hover_signal = on_inspect;

    rsx! {
        tr {
            class: cn!("group", color),
            onpointerenter: move |_| {
                if let Some(handler) = hover_signal {
                    handler.call(true);
                }
            },
            td { class: "relative h-8 px-4 py-0",
                div {
                    "aria-hidden": "true",
                    class: "pointer-events-none absolute left-1 inset-y-px w-[calc(300%-8px)] overflow-hidden rounded-sm",
                    div { class: "absolute inset-0 origin-right overflow-hidden rounded-sm", style: "{bar_style}",
                        div { class: "absolute inset-0 bg-current opacity-[0.12]" }
                    }
                    div { class: "absolute inset-0 bg-current", style: "{range_style}" }
                }
                if let Some(on_select) = on_select {
                    button {
                        r#type: "button",
                        onclick: move |_| on_select.call(()),
                        onfocus: move |_| {
                            if let Some(handler) = hover_signal {
                                handler.call(true);
                            }
                        },
                        onblur: move |_| {
                            if let Some(handler) = hover_signal {
                                handler.call(false);
                            }
                        },
                        class: "relative rounded-sm outline-none hover:underline focus-visible:ring-2 focus-visible:ring-ring",
                        "{price_text}"
                    }
                } else {
                    span { class: "relative", "{price_text}" }
                }
            }
            td { class: "relative px-4 py-0 text-right text-foreground", "{size_text}" }
            td { class: "relative px-4 py-0 text-right text-muted-foreground", "{total_text}" }
        }
    }
}

/// `OrderBookSpread`.
#[component]
pub fn OrderBookSpread(#[props(default)] class: String) -> Element {
    let context = use_context::<OrderBookContext>();
    let book = (context.book)();
    let price = (context.price)();
    let spread = book.spread;
    let midpoint = book.midpoint;
    let spread_label = match (spread, midpoint) {
        (Some(spread), Some(midpoint)) if midpoint != 0.0 => {
            if spread < 0.0 {
                "Crossed book".to_string()
            } else {
                format!(
                    "Spread {} · {:.3}%",
                    context.price(spread),
                    (spread / midpoint) * 100.0
                )
            }
        }
        _ => "Spread unavailable".to_string(),
    };
    let price_label = match price {
        Some(price) => context.price(price),
        None => "—".to_string(),
    };
    rsx! {
        div {
            class: cn!(
                "my-1 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 border-y border-border bg-muted/40 px-4 py-3",
                class,
            ),
            span { class: "text-base font-medium tracking-tight",
                span { class: "sr-only", "Reference price: " }
                "{price_label}"
            }
            span { class: "text-[10px] text-muted-foreground", "{spread_label}" }
        }
    }
}

/// `OrderBookBalance`.
#[component]
pub fn OrderBookBalance(#[props(default)] class: String) -> Element {
    let context = use_context::<OrderBookContext>();
    let reduce = use_reduced_motion()();
    let book = (context.book)();
    let total = book.bid_total + book.ask_total;
    let ratio = if total != 0.0 {
        book.bid_total / total
    } else {
        0.5
    };
    let bid_label = if total != 0.0 {
        format!("Bids {:.1}%", ratio * 100.0)
    } else {
        "Bids —".to_string()
    };
    let ask_label = if total != 0.0 {
        format!("Asks {:.1}%", (1.0 - ratio) * 100.0)
    } else {
        "Asks —".to_string()
    };
    let bar_style = format!(
        "transform:scaleX({ratio});transform-origin:left;{}",
        if reduce {
            ""
        } else {
            "transition:transform 320ms cubic-bezier(0.32,0.72,0,1);"
        }
    );
    rsx! {
        div { class: cn!("space-y-2 border-t border-border px-4 pb-3 pt-3", class),
            div { class: "flex justify-between text-[10px]",
                span { class: "text-emerald-700 dark:text-emerald-400", "{bid_label}" }
                span { class: "text-muted-foreground", "Visible depth" }
                span { class: "text-rose-600 dark:text-rose-400", "{ask_label}" }
            }
            div { "aria-hidden": "true", class: "relative h-1 overflow-hidden rounded-full bg-rose-500/25",
                div { class: "absolute inset-0 bg-emerald-500/60", style: "{bar_style}" }
            }
        }
    }
}

/// `OrderBookDepthSummary`.
#[component]
pub fn OrderBookDepthSummary(
    #[props(default = "Hover a level to see the cost of filling up to it".to_string())] placeholder: String,
    #[props(default)] class: String,
) -> Element {
    let context = use_context::<OrderBookContext>();
    let summary = context.summary();
    let base = (context.base_symbol)();
    let quote = (context.quote_symbol)();
    let size_suffix = if base.is_empty() {
        String::new()
    } else {
        format!(" {base}")
    };
    let quote_suffix = if quote.is_empty() {
        String::new()
    } else {
        format!(" {quote}")
    };
    rsx! {
        p {
            "aria-live": "polite",
            class: cn!(
                "flex min-h-10 flex-wrap items-center justify-between gap-x-4 gap-y-1 border-t border-border px-4 py-2 text-muted-foreground",
                class,
            ),
            if let Some(summary) = summary {
                span {
                    "Avg "
                    span { class: "text-foreground", "{context.price(summary.average_price)}" }
                }
                span {
                    span { class: "text-foreground", "{context.size(summary.size)}{size_suffix}" }
                    " · "
                    "{context.price(summary.quote)}{quote_suffix}"
                }
            } else {
                span { class: "font-sans", "{placeholder}" }
            }
        }
    }
}

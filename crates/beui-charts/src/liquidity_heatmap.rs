//! `charts/liquidity-heatmap/` — resting liquidity by price and time.
//!
//! `buildLiquidityHeatmap` (dedupe, level aggregation, discrete price rows,
//! maximum) and `pricePosition` (interpolated trace between price bands) are
//! ported verbatim. The model is a context shared by the plot and legend.
//!
//! Differences from the source: the source animates cell color/opacity
//! and the trace `d` through motion; the port renders the final color and path
//! with CSS transitions. `NumberTicker` is not ported and the tooltip is a
//! lane-local bubble positioned over the active cell.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use beui::prelude::*;

use crate::candlestick_chart::default_volume_format;

/// `colors`.
pub const LIQUIDITY_COLORS: [&str; 6] = [
    "#17233b", "#254f80", "#217c90", "#27af98", "#97d56b", "#f4df76",
];

/// `LiquiditySnapshot` level.
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidityLevel {
    pub price: f64,
    pub size: f64,
}

impl LiquidityLevel {
    pub fn new(price: f64, size: f64) -> Self {
        Self { price, size }
    }
}

/// `LiquiditySnapshot`.
#[derive(Clone, Debug, PartialEq)]
pub struct LiquiditySnapshot {
    pub id: String,
    pub label: String,
    pub levels: Vec<LiquidityLevel>,
    pub price: Option<f64>,
}

impl LiquiditySnapshot {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        levels: Vec<LiquidityLevel>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            levels,
            price: None,
        }
    }

    pub fn price(mut self, price: f64) -> Self {
        self.price = Some(price);
        self
    }
}

/// A total-ordered f64 key for the price buckets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceKey(pub f64);

impl Eq for PriceKey {}

impl PartialOrd for PriceKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriceKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// A resolved column.
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidityColumn {
    pub id: String,
    pub label: String,
    pub price: Option<f64>,
    pub levels: BTreeMap<PriceKey, f64>,
}

impl LiquidityColumn {
    pub fn size(&self, price: f64) -> Option<f64> {
        self.levels.get(&PriceKey(price)).copied()
    }
}

/// `buildLiquidityHeatmap` result.
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidityBuild {
    pub columns: Vec<LiquidityColumn>,
    pub prices: Vec<f64>,
    pub maximum: f64,
}

/// `buildLiquidityHeatmap`.
pub fn build_liquidity_heatmap(snapshots: &[LiquiditySnapshot]) -> LiquidityBuild {
    let mut seen: Vec<String> = Vec::new();
    let mut columns = Vec::new();
    for snapshot in snapshots {
        if seen.contains(&snapshot.id) {
            continue;
        }
        seen.push(snapshot.id.clone());
        let mut levels: BTreeMap<PriceKey, f64> = BTreeMap::new();
        for level in &snapshot.levels {
            if !level.price.is_finite()
                || level.price <= 0.0
                || !level.size.is_finite()
                || level.size < 0.0
            {
                continue;
            }
            let total = levels.get(&PriceKey(level.price)).copied().unwrap_or(0.0) + level.size;
            if total.is_finite() {
                levels.insert(PriceKey(level.price), total);
            }
        }
        columns.push(LiquidityColumn {
            id: snapshot.id.clone(),
            label: snapshot.label.clone(),
            price: snapshot.price,
            levels,
        });
    }
    let mut prices: Vec<f64> = columns
        .iter()
        .flat_map(|column| column.levels.keys().map(|key| key.0))
        .collect();
    prices.sort_by(|a, b| b.total_cmp(a));
    prices.dedup();
    let mut maximum = 0.0f64;
    for column in &columns {
        for size in column.levels.values() {
            maximum = maximum.max(*size);
        }
    }
    LiquidityBuild {
        columns,
        prices,
        maximum,
    }
}

/// `pricePosition`.
pub fn price_position(price: f64, prices: &[f64]) -> Option<f64> {
    if !price.is_finite()
        || prices.is_empty()
        || price > prices[0]
        || price < prices[prices.len() - 1]
    {
        return None;
    }
    for index in 0..prices.len() - 1 {
        if price <= prices[index] && price >= prices[index + 1] {
            return Some(
                index as f64 + 0.5 + (prices[index] - price) / (prices[index] - prices[index + 1]),
            );
        }
    }
    Some(prices.len() as f64 - 0.5)
}

/// `heatColor`.
pub fn heat_color(value: f64, max: f64) -> &'static str {
    let ratio = if max > 0.0 { value / max } else { 0.0 };
    let index =
        ((ratio * LIQUIDITY_COLORS.len() as f64).floor() as usize).min(LIQUIDITY_COLORS.len() - 1);
    LIQUIDITY_COLORS[index]
}

#[derive(Clone, Copy)]
struct LiquidityContext {
    columns: Signal<Vec<LiquidityColumn>>,
    prices: Signal<Vec<f64>>,
    ceiling: Signal<f64>,
    unit: Signal<String>,
    format_price: Option<Callback<f64, String>>,
    format_size: Option<Callback<f64, String>>,
}

impl LiquidityContext {
    fn price(&self, value: f64) -> String {
        match self.format_price {
            Some(callback) => callback.call(value),
            None => crate::donut_chart::default_format(value),
        }
    }

    fn size(&self, value: f64) -> String {
        match self.format_size {
            Some(callback) => callback.call(value),
            None => default_volume_format(value),
        }
    }
}

fn cell_key(column_id: &str, price: f64) -> String {
    format!("{column_id}\u{1}{price}")
}

/// `LiquidityHeatmap`.
#[component]
pub fn LiquidityHeatmap(
    snapshots: Vec<LiquiditySnapshot>,
    #[props(default)] max_size: Option<f64>,
    #[props(default = "units".to_string())] unit: String,
    #[props(default = "Liquidity heatmap".to_string())] label: String,
    #[props(default)] format_price: Option<Callback<f64, String>>,
    #[props(default)] format_size: Option<Callback<f64, String>>,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let build = build_liquidity_heatmap(&snapshots);
    let ceiling = max_size
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(build.maximum);
    let columns_signal = use_signal(|| build.columns);
    let prices_signal = use_signal(|| build.prices);
    let ceiling_signal = use_signal(|| ceiling);
    let unit_signal = use_signal(|| unit.clone());

    let context = LiquidityContext {
        columns: columns_signal,
        prices: prices_signal,
        ceiling: ceiling_signal,
        unit: unit_signal,
        format_price,
        format_size,
    };
    use_context_provider(|| context);

    rsx! {
        section {
            "aria-label": "{label}",
            class: cn!("w-full space-y-4", class),
            if let Some(children) = children {
                {children}
            } else {
                LiquidityHeatmapPlot {}
                LiquidityHeatmapLegend {}
            }
        }
    }
}

/// `LiquidityHeatmapPlot`.
#[component]
pub fn LiquidityHeatmapPlot(#[props(default)] class: String) -> Element {
    let context = use_context::<LiquidityContext>();
    let reduce = use_reduced_motion()();
    let columns = (context.columns)();
    let prices = (context.prices)();
    let ceiling = (context.ceiling)();
    let unit = (context.unit)();
    let tooltip_id = crate::common::use_unique_id("liquidity-tip");
    let mut active = use_signal(|| None::<String>);
    let cursor = use_signal(|| None::<String>);

    if columns.is_empty() || prices.is_empty() {
        return rsx! {
            p { class: cn!("py-16 text-center text-sm text-muted-foreground", class), "No liquidity data" }
        };
    }

    let mut keys: Vec<String> = Vec::with_capacity(prices.len() * columns.len());
    for price in prices.iter() {
        for column in columns.iter() {
            keys.push(cell_key(&column.id, *price));
        }
    }
    let valid_cursor = match cursor() {
        Some(cursor) if keys.contains(&cursor) => cursor,
        _ => keys.first().cloned().unwrap_or_default(),
    };
    let key_count = keys.len();
    let active_key = active();
    let active_index = active_key
        .as_ref()
        .and_then(|key| keys.iter().position(|candidate| candidate == key));
    let row = active_index.map(|index| index / columns.len());
    let col = active_index.map(|index| index % columns.len());
    let selected = col.and_then(|col| columns.get(col).cloned());
    let size = match (row, selected.as_ref()) {
        (Some(row), Some(column)) => column.size(prices[row]),
        _ => None,
    };

    // The trace across columns; a missing market price breaks the line.
    let mut connected = false;
    let mut path = String::new();
    for (index, column) in columns.iter().enumerate() {
        let y = column
            .price
            .and_then(|price| price_position(price, &prices));
        let Some(y) = y else {
            connected = false;
            continue;
        };
        let command = if connected { "L" } else { "M" };
        connected = true;
        if !path.is_empty() {
            path.push(' ');
        }
        path.push_str(&format!("{command}{},{}", index as f64 + 0.5, y));
    }

    let min_width = 480.0f64.max(columns.len() as f64 * 24.0);
    let column_count = columns.len();
    let price_count = prices.len();
    let tooltip_x = col.map(|col| (col as f64 + 0.5) / column_count as f64 * 100.0);
    let tooltip_y = row.map(|row| (row as f64 + 0.5) / price_count as f64 * 100.0);
    let selected_label = selected.as_ref().map(|column| column.label.clone());
    let selected_market = selected.as_ref().and_then(|column| column.price);
    let active_price = row.map(|row| prices[row]);
    let label_step = 1usize.max((column_count as f64 / 5.0).ceil() as usize);

    rsx! {
        div { class: cn!("overflow-x-auto", class),
            div { class: "relative", style: "min-width:{min_width}px;padding-right:72px",
                div { class: "relative",
                    table {
                        "aria-label": "Liquidity by price and time",
                        class: "w-full table-fixed border-separate border-spacing-[2px]",
                        thead { class: "sr-only",
                            tr {
                                for column in columns.iter() {
                                    th { key: "{column.id}", scope: "col", "{column.label}" }
                                }
                            }
                        }
                        tbody {
                            for (row_index, price) in prices.iter().enumerate() {
                                tr { key: "{price}",
                                    for (column_index, column) in columns.iter().enumerate() {
                                        {
                                            let value = column.size(*price);
                                            let key = cell_key(&column.id, *price);
                                            let index = row_index * column_count + column_index;
                                            let is_active = active_key.as_deref() == Some(key.as_str());
                                            let aria = format!(
                                                "{}, {}: {}",
                                                column.label,
                                                context.price(*price),
                                                match value {
                                                    Some(value) => format!("{} {}", context.size(value), unit),
                                                    None => "No data".to_string(),
                                                },
                                            );
                                            let color = heat_color(value.unwrap_or(0.0), ceiling);
                                            let opacity = if value.is_none() { 0.0 } else { 1.0 };
                                            let duration = if reduce { 0 } else { 280 };
                                            let bg_style = format!(
                                                "background-color:{color};opacity:{opacity};transition:opacity {duration}ms,background-color {duration}ms;"
                                            );
                                            rsx! {
                                                td { key: "{column.id}", class: "p-0",
                                                    button {
                                                        r#type: "button",
                                                        "data-cell": "{index}",
                                                        tabindex: if key == valid_cursor { "0" } else { "-1" },
                                                        "aria-label": "{aria}",
                                                        "aria-describedby": if is_active { Some(tooltip_id.clone()) } else { None },
                                                        class: "relative block h-6 w-full rounded-[2px] bg-muted/30 outline-none focus-visible:z-10 focus-visible:ring-2 focus-visible:ring-foreground",
                                                        onpointerenter: {
                                                            let mut active = active;
                                                            let key = key.clone();
                                                            move |event: PointerEvent| {
                                                                if event.pointer_type() != "touch" {
                                                                    active.set(Some(key.clone()));
                                                                }
                                                            }
                                                        },
                                                        onpointerleave: {
                                                            let mut active = active;
                                                            move |_| active.set(None)
                                                        },
                                                        onfocus: {
                                                            let mut active = active;
                                                            let mut cursor = cursor;
                                                            let key = key.clone();
                                                            move |_| {
                                                                cursor.set(Some(key.clone()));
                                                                active.set(Some(key.clone()));
                                                            }
                                                        },
                                                        onblur: {
                                                            let mut active = active;
                                                            move |_| active.set(None)
                                                        },
                                                        onclick: {
                                                            let mut active = active;
                                                            let key = key.clone();
                                                            move |_| active.set(Some(key.clone()))
                                                        },
                                                        onkeydown: {
                                                            move |event: KeyboardEvent| {
                                                                if event.key() == Key::Escape {
                                                                    active.set(None);
                                                                    return;
                                                                }
                                                                let offset: i64 = if event.key() == Key::ArrowRight {
                                                                    1
                                                                } else if event.key() == Key::ArrowLeft {
                                                                    -1
                                                                } else if event.key() == Key::ArrowDown {
                                                                    column_count as i64
                                                                } else if event.key() == Key::ArrowUp {
                                                                    -(column_count as i64)
                                                                } else if event.key() == Key::Home {
                                                                    -(column_index as i64)
                                                                } else if event.key() == Key::End {
                                                                    column_count as i64 - 1 - column_index as i64
                                                                } else {
                                                                    return;
                                                                };
                                                                event.prevent_default();
                                                                let next = (index as i64 + offset)
                                                                    .clamp(0, key_count as i64 - 1);
                                                                beui_core::platform::fire(&format!(
                                                                    "var el=document.querySelector({});if(el)el.focus();",
                                                                    beui_core::platform::js_string(&format!("[data-cell=\"{next}\"]")),
                                                                ));
                                                            }
                                                        },
                                                        span {
                                                            "aria-hidden": "true",
                                                            class: "pointer-events-none absolute inset-0 rounded-[2px]",
                                                            style: "{bg_style}",
                                                        }
                                                        if is_active {
                                                            span {
                                                                "aria-hidden": "true",
                                                                class: "pointer-events-none absolute inset-0 rounded-[2px] ring-1 ring-inset ring-white",
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div {
                        "aria-hidden": "true",
                        class: "absolute -right-[72px] top-0 bottom-0 flex w-16 flex-col justify-around text-right font-mono text-[10px] tabular-nums text-muted-foreground",
                        for price in prices.iter() {
                            span { key: "{price}", "{context.price(*price)}" }
                        }
                    }
                    svg {
                        "aria-hidden": "true",
                        class: "pointer-events-none absolute inset-0 h-full w-full overflow-visible",
                        view_box: "0 0 {column_count} {price_count}",
                        preserve_aspect_ratio: "none",
                        path {
                            d: "{path}",
                            fill: "none",
                            stroke: "black",
                            "stroke-opacity": "0.35",
                            "stroke-width": "4",
                            "vector-effect": "non-scaling-stroke",
                        }
                        path {
                            d: "{path}",
                            fill: "none",
                            stroke: "white",
                            "stroke-opacity": "0.9",
                            "stroke-width": "1.5",
                            "stroke-linejoin": "round",
                            "vector-effect": "non-scaling-stroke",
                        }
                    }
                    if let (Some(x), Some(y)) = (tooltip_x, tooltip_y) {
                        if let Some(selected_label) = selected_label {
                            div {
                                id: "{tooltip_id}",
                                role: "tooltip",
                                class: "pointer-events-none absolute z-20 flex -translate-x-1/2 -translate-y-full flex-col gap-1 rounded-lg border border-border bg-popover px-2 py-1 font-mono text-xs text-popover-foreground shadow-md",
                                style: "left:{x}%;top:{y}%",
                                span { class: "text-muted-foreground",
                                    "{selected_label} · {active_price.map(|price| context.price(price)).unwrap_or_default()}"
                                }
                                span { class: "inline-flex items-center tabular-nums",
                                    match size {
                                        Some(size) => format!("{} {}", context.size(size), unit),
                                        None => "No data".to_string(),
                                    }
                                }
                                if let Some(market) = selected_market {
                                    if market.is_finite() {
                                        span { class: "text-muted-foreground", "Market {context.price(market)}" }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    "aria-hidden": "true",
                    class: "mt-3 flex justify-between font-mono text-[10px] text-muted-foreground",
                    for (index, column) in columns.iter().enumerate() {
                        if index % label_step == 0 || index == column_count - 1 {
                            span { key: "{column.id}", "{column.label}" }
                        }
                    }
                }
            }
        }
    }
}

/// `LiquidityHeatmapLegend`.
#[component]
pub fn LiquidityHeatmapLegend(#[props(default)] class: String) -> Element {
    let context = use_context::<LiquidityContext>();
    let ceiling = (context.ceiling)();
    let unit = (context.unit)();
    rsx! {
        div {
            class: cn!(
                "flex flex-wrap items-center justify-between gap-3 text-[10px] text-muted-foreground",
                class,
            ),
            span { "Resting liquidity · {unit}" }
            span { class: "flex items-center gap-2",
                span { "0" }
                span { "aria-hidden": "true", class: "flex gap-0.5",
                    for color in LIQUIDITY_COLORS.iter() {
                        span {
                            key: "{color}",
                            class: "h-2 w-5 rounded-[1px]",
                            style: "background-color:{color};",
                        }
                    }
                }
                span { "{context.size(ceiling)}+" }
            }
        }
    }
}

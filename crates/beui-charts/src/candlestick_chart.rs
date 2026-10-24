//! `charts/candlestick-chart/` — OHLC candles with volume and a crosshair.
//!
//! The price scale (padded low/high range, volume max), the plot geometry
//! (`y`, `priceAt`, slots, five ticks), pointer/keyboard crosshair movement and
//! the OHLC readout are ported. State lives in a shared context.
//!
//! Differences from the source: the source animates each candle's
//! wick/body/volume with motion springs and draws them at unit height with
//! `scaleY`; the port renders the resolved geometry directly, so live updates
//! do not tween. `formatVolume`'s `Intl.NumberFormat` compact notation is
//! approximated, and the legend's `children` render function is not ported.

use beui::prelude::*;

use crate::donut_chart::default_format;

/// Right gutter reserved for price labels, in px.
const AXIS: f64 = 64.0;
const VOLUME_SHARE: f64 = 0.18;
const GAP: f64 = 8.0;

/// `Candle`.
#[derive(Clone, Debug, PartialEq)]
pub struct Candle {
    pub id: String,
    pub label: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Candle {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            open,
            high,
            low,
            close,
            volume,
        }
    }
}

/// `defaultVolume` — compact notation without `Intl`.
pub fn default_volume_format(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1e9 {
        format!("{:.1}B", value / 1e9)
    } else if abs >= 1e6 {
        format!("{:.1}M", value / 1e6)
    } else if abs >= 1e3 {
        format!("{:.1}K", value / 1e3)
    } else {
        format!("{value}")
    }
}

#[derive(Clone, Copy)]
struct CandlestickContext {
    candles: Signal<Vec<Candle>>,
    last_price: Signal<Option<f64>>,
    active: Signal<Option<String>>,
    controlled: bool,
    on_active_id_change: Option<EventHandler<Option<String>>>,
    min: Signal<f64>,
    max: Signal<f64>,
    volume_max: Signal<f64>,
    label: Signal<String>,
    format_price: Option<Callback<f64, String>>,
    format_volume: Option<Callback<f64, String>>,
}

impl CandlestickContext {
    fn price(&self, value: f64) -> String {
        match self.format_price {
            Some(callback) => callback.call(value),
            None => default_format(value),
        }
    }

    fn volume(&self, value: f64) -> String {
        match self.format_volume {
            Some(callback) => callback.call(value),
            None => default_volume_format(value),
        }
    }

    fn active_index(&self) -> Option<usize> {
        let id = (self.active)()?;
        let candles = (self.candles)();
        candles.iter().position(|candle| candle.id == id)
    }

    fn set_active(&self, next: Option<usize>) {
        let id = next.and_then(|index| (self.candles)().get(index).map(|candle| candle.id.clone()));
        if !self.controlled {
            let mut active = self.active;
            active.set(id.clone());
        }
        if let Some(handler) = self.on_active_id_change {
            handler.call(id);
        }
    }
}

/// `CandlestickChart`.
#[component]
pub fn CandlestickChart(
    candles: Vec<Candle>,
    #[props(default)] last_price: Option<f64>,
    #[props(default)] active_id: Option<String>,
    #[props(default)] default_active_id: Option<String>,
    #[props(default)] on_active_id_change: Option<EventHandler<Option<String>>>,
    #[props(default)] format_price: Option<Callback<f64, String>>,
    #[props(default)] format_volume: Option<Callback<f64, String>>,
    #[props(default = "Candlestick chart".to_string())] label: String,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let mut lows: Vec<f64> = candles.iter().map(|candle| candle.low).collect();
    let mut highs: Vec<f64> = candles.iter().map(|candle| candle.high).collect();
    if let Some(last_price) = last_price {
        lows.push(last_price);
        highs.push(last_price);
    }
    let min = lows.iter().copied().fold(f64::INFINITY, f64::min);
    let min = if min.is_finite() { min } else { 0.0 };
    let max = highs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let max = if max.is_finite() { max } else { 1.0 };
    let pad = ((max - min) * 0.08).max(max * 0.0005).max(0.0001);
    let volume_max = candles
        .iter()
        .map(|candle| candle.volume)
        .fold(1.0f64, f64::max);

    let candles_signal = use_signal(|| candles.clone());
    let last_price_signal = use_signal(|| last_price);
    let mut internal_active = use_signal(|| default_active_id.clone());
    let min_signal = use_signal(|| min - pad);
    let max_signal = use_signal(|| max + pad);
    let volume_max_signal = use_signal(|| volume_max);
    let label_signal = use_signal(|| label.clone());
    let controlled = active_id.is_some();

    use_effect(use_reactive((&active_id,), move |(active_id,)| {
        if let Some(active_id) = active_id {
            internal_active.set(Some(active_id));
        }
    }));

    let context = CandlestickContext {
        candles: candles_signal,
        last_price: last_price_signal,
        active: internal_active,
        controlled,
        on_active_id_change,
        min: min_signal,
        max: max_signal,
        volume_max: volume_max_signal,
        label: label_signal,
        format_price,
        format_volume,
    };
    use_context_provider(|| context);

    rsx! {
        div { class: cn!("w-full", class),
            if let Some(children) = children {
                {children}
            } else {
                div { class: "grid gap-3",
                    CandlestickChartLegend {}
                    CandlestickChartPlot {}
                }
            }
        }
    }
}

/// `CandlestickChartPlot`.
#[component]
pub fn CandlestickChartPlot(
    #[props(default = 320.0)] height: f64,
    #[props(default)] class: String,
) -> Element {
    let context = use_context::<CandlestickContext>();
    let plot_id = crate::common::use_unique_id("candlestick-plot");
    let plot_id_for_effect = plot_id.clone();
    let width_signal = use_signal(|| 0.0f64);
    let pointer_y = use_signal(|| None::<f64>);

    // Measure the plot width and keep it current.
    use_effect(move || {
        let selector = format!("[data-beui-candlestick=\"{plot_id_for_effect}\"]");
        let script = format!(
            "(function(){{var el=document.querySelector({sel});if(!el)return;\
             function send(){{dioxus.send({{left:0,top:0,width:el.clientWidth,height:el.clientHeight}});}}\
             send();var ro=new ResizeObserver(send);ro.observe(el);}})();",
            sel = beui_core::platform::js_string(&selector),
        );
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(rect) = eval.recv::<beui_core::platform::Rect>().await {
                if (width_signal)() != rect.width {
                    let mut signal = width_signal;
                    signal.set(rect.width);
                }
            }
        });
    });

    let candles = (context.candles)();
    let active = context.active_index();
    let min = (context.min)();
    let max = (context.max)();
    let volume_max = (context.volume_max)();
    let label = (context.label)();
    let width = width_signal();
    let plot_width = (width - AXIS).max(0.0);
    let price_height = height * (1.0 - VOLUME_SHARE) - GAP;
    let count = candles.len();
    let slot = if count > 0 {
        plot_width / count as f64
    } else {
        0.0
    };
    let body = (slot * 0.62).max(1.0);
    let range = if (max - min).abs() > f64::EPSILON {
        max - min
    } else {
        1.0
    };
    let y = |price: f64| ((max - price) / range) * price_height;
    let price_at = |py: f64| max - (py / price_height) * range;
    let ticks: Vec<f64> = (0..5)
        .map(|i| max - (range * (i as f64 + 0.5)) / 5.0)
        .collect();
    let focus_index = active.unwrap_or(count.saturating_sub(1));
    let focus = candles.get(focus_index).cloned();
    let cross_x = active.map(|index| index as f64 * slot + slot / 2.0);
    let last_price = (context.last_price)();
    let pointer_y_value = pointer_y();
    let valuetext = focus
        .as_ref()
        .map(|focus| {
            format!(
                "{}: open {}, high {}, low {}, close {}",
                focus.label,
                context.price(focus.open),
                context.price(focus.high),
                context.price(focus.low),
                context.price(focus.close),
            )
        })
        .unwrap_or_default();
    let scroll_ok = width > 0.0;

    rsx! {
        div {
            "data-beui-candlestick": "{plot_id}",
            role: "slider",
            tabindex: "0",
            "aria-label": "{label}",
            "aria-valuemin": "0",
            "aria-valuemax": "{count.saturating_sub(1)}",
            "aria-valuenow": "{focus_index}",
            "aria-valuetext": "{valuetext}",
            class: cn!(
                "relative w-full cursor-crosshair touch-pan-y select-none rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            style: "height:{height}px",
            onpointermove: move |event: PointerEvent| {
                let coords = event.data().element_coordinates();
                let x = coords.x;
                let py = coords.y;
                if x > plot_width || slot == 0.0 {
                    return;
                }
                let index = ((x / slot).floor() as i64).clamp(0, count as i64 - 1) as usize;
                context.set_active(Some(index));
                let mut signal = pointer_y;
                signal.set(if py <= price_height { Some(py) } else { None });
            },
            onpointerleave: move |_| {
                context.set_active(None);
                let mut signal = pointer_y;
                signal.set(None);
            },
            onkeydown: move |event: KeyboardEvent| {
                let current = active.unwrap_or(count.saturating_sub(1));
                let next = if event.key() == Key::ArrowLeft {
                    Some(current.saturating_sub(1))
                } else if event.key() == Key::ArrowRight {
                    Some((current + 1).min(count.saturating_sub(1)))
                } else if event.key() == Key::Home {
                    Some(0)
                } else if event.key() == Key::End {
                    Some(count.saturating_sub(1))
                } else if event.key() == Key::Escape {
                    None
                } else {
                    return;
                };
                event.prevent_default();
                let mut signal = pointer_y;
                signal.set(None);
                context.set_active(next);
            },
            onblur: move |_| context.set_active(None),
            if scroll_ok {
                svg {
                    width: "{width}",
                    height: "{height}",
                    class: "absolute inset-0 overflow-visible",
                    "aria-hidden": "true",
                    for tick in ticks.iter() {
                        line {
                            key: "{tick}",
                            x1: "0",
                            x2: "{plot_width}",
                            y1: "{y(*tick)}",
                            y2: "{y(*tick)}",
                            stroke: "var(--border)",
                            "stroke-dasharray": "2 6",
                        }
                    }
                    for (index, candle) in candles.iter().enumerate() {
                        {
                            let up = candle.close >= candle.open;
                            let cx = index as f64 * slot + slot / 2.0;
                            let top = y(candle.open.max(candle.close));
                            let body_height = (y(candle.open) - y(candle.close)).abs().max(1.0);
                            let wick_top = y(candle.high);
                            let wick_height = (y(candle.low) - wick_top).max(1.0);
                            let volume_height = ((candle.volume / volume_max) * height * VOLUME_SHARE).max(0.5);
                            let dimmed = active.is_some() && active != Some(index);
                            let opacity = if dimmed { 0.45 } else { 1.0 };
                            let tone = if up { "fill-emerald-500 stroke-emerald-500" } else { "fill-rose-500 stroke-rose-500" };
                            rsx! {
                                g {
                                    key: "{candle.id}",
                                    class: "{tone}",
                                    style: "opacity:{opacity};transition:opacity 300ms;",
                                    line {
                                        x1: "{cx}",
                                        x2: "{cx}",
                                        y1: "{wick_top}",
                                        y2: "{wick_top + wick_height}",
                                        "vector-effect": "non-scaling-stroke",
                                        "stroke-width": "1",
                                    }
                                    rect {
                                        x: "{cx - body / 2.0}",
                                        y: "{top}",
                                        width: "{body}",
                                        height: "{body_height}",
                                        "stroke-width": "0",
                                    }
                                    rect {
                                        x: "{cx - body / 2.0}",
                                        y: "{height - volume_height}",
                                        width: "{body}",
                                        height: "{volume_height}",
                                        "stroke-width": "0",
                                        opacity: "0.28",
                                    }
                                }
                            }
                        }
                    }
                    if let Some(last_price) = last_price {
                        line {
                            x1: "0",
                            x2: "{plot_width}",
                            y1: "{y(last_price)}",
                            y2: "{y(last_price)}",
                            stroke: "var(--foreground)",
                            "stroke-opacity": "0.45",
                            "stroke-dasharray": "3 4",
                        }
                    }
                    if let Some(cross_x) = cross_x {
                        line {
                            x1: "{cross_x}",
                            x2: "{cross_x}",
                            y1: "0",
                            y2: "{height}",
                            stroke: "var(--foreground)",
                            "stroke-opacity": "0.3",
                        }
                    }
                    if let Some(pointer_y) = pointer_y_value {
                        line {
                            x1: "0",
                            x2: "{plot_width}",
                            y1: "{pointer_y}",
                            y2: "{pointer_y}",
                            stroke: "var(--foreground)",
                            "stroke-opacity": "0.3",
                        }
                    }
                }
                for tick in ticks.iter() {
                    span {
                        key: "lab{tick}",
                        class: "pointer-events-none absolute right-0 -translate-y-1/2 pr-1 font-mono text-xs text-muted-foreground tabular-nums",
                        style: "top:{y(*tick)}px",
                        "{context.price(*tick)}"
                    }
                }
                if let Some(last_price) = last_price {
                    div { class: "pointer-events-none absolute top-0 right-0", style: "transform:translateY({y(last_price)}px);",
                        span { class: "block -translate-y-1/2 rounded-md bg-foreground px-1.5 py-0.5 font-mono text-xs text-background tabular-nums",
                            "{context.price(last_price)}"
                        }
                    }
                }
                if let Some(pointer_y) = pointer_y_value {
                    span {
                        class: "pointer-events-none absolute right-0 -translate-y-1/2 rounded-md bg-muted px-1.5 py-0.5 font-mono text-xs text-foreground tabular-nums",
                        style: "top:{pointer_y}px",
                        "{context.price(price_at(pointer_y))}"
                    }
                }
            }
        }
    }
}

/// `CandlestickChartLegend`.
#[component]
pub fn CandlestickChartLegend(#[props(default)] class: String) -> Element {
    let context = use_context::<CandlestickContext>();
    let candles = (context.candles)();
    let active = context.active_index();
    let candle = active
        .and_then(|index| candles.get(index).cloned())
        .or_else(|| candles.last().cloned());
    let Some(candle) = candle else {
        return rsx! {};
    };
    let change = if candle.open != 0.0 {
        ((candle.close - candle.open) / candle.open) * 100.0
    } else {
        0.0
    };
    let tone = if change >= 0.0 {
        "text-emerald-600 dark:text-emerald-400"
    } else {
        "text-rose-600 dark:text-rose-400"
    };
    let change_text = format!(
        "{}{:.2}%",
        if change >= 0.0 { "+" } else { "−" },
        change.abs()
    );
    let items: [(&str, String); 5] = [
        ("O", context.price(candle.open)),
        ("H", context.price(candle.high)),
        ("L", context.price(candle.low)),
        ("C", context.price(candle.close)),
        ("Vol", context.volume(candle.volume)),
    ];

    rsx! {
        dl {
            class: cn!("flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-xs tabular-nums", class),
            dt { class: "sr-only", "Candle" }
            dd { class: "text-muted-foreground", "{candle.label}" }
            for (term, value) in items.iter() {
                div { key: "{term}", class: "flex gap-1",
                    dt { class: "text-muted-foreground", "{term}" }
                    dd { class: if *term == "Vol" { "" } else { tone }, "{value}" }
                }
            }
            dd { class: "{tone}", "{change_text}" }
        }
    }
}

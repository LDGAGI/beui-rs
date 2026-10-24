//! `charts/price-target-fan/` — a year of history then three analyst targets.
//!
//! The domain (min/max of history, current and targets plus padding), the
//! geometry (history x-scale, the line, the three quadratic projections), the
//! pointer hit-testing (scrub the history, else nearest projection in the fan),
//! the overlay card and the round-numbered gridlines are ported.
//!
//! Differences from the source: invalid history/targets are filtered
//! rather than thrown; the source's motion draw-on and sliding card are CSS/static;
//! `NumberTicker` is not ported; the tooltip is a lane-local bubble. The parts
//! are exported but the composition is monolithic (one component renders header,
//! plot and tooltip).

use beui::prelude::*;

use crate::heat_calendar::{civil_from_days, days_from_civil};

const UP: &str = "var(--success)";
const MID: &str = "var(--accent)";
const LOW: &str = "var(--warning)";
/// `W`.
pub const W: f64 = 520.0;
/// `H`.
pub const H: f64 = 236.0;
const PAD_L: f64 = 36.0;
const PAD_R: f64 = 108.0;
const PAD_T: f64 = 16.0;
const PAD_B: f64 = 28.0;

const RING_KEYFRAMES: &str =
    "@keyframes beui-fan-ring{from{opacity:0.45;transform:scale(1)}to{opacity:0;transform:scale(2.8)}}";

/// `ink` — cap lightness on the light page.
fn ink(color: &str) -> String {
    format!("oklch(from {color} min(l, var(--ink-l, 1)) c h)")
}

/// `PriceTarget`.
#[derive(Clone, Debug, PartialEq)]
pub struct PriceTarget {
    pub key: String,
    pub price: f64,
    pub analysts: i64,
    pub color: Option<String>,
}

impl PriceTarget {
    pub fn new(key: impl Into<String>, price: f64, analysts: i64) -> Self {
        Self {
            key: key.into(),
            price,
            analysts,
            color: None,
        }
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// `PriceHistoryPoint`.
#[derive(Clone, Debug, PartialEq)]
pub struct PriceHistoryPoint {
    pub date: String,
    pub price: f64,
}

impl PriceHistoryPoint {
    pub fn new(date: impl Into<String>, price: f64) -> Self {
        Self {
            date: date.into(),
            price,
        }
    }
}

/// `PriceTargetFanActive`.
#[derive(Clone, Debug, PartialEq)]
pub enum PriceTargetFanActive {
    History { date: String },
    Target { key: String },
}

/// `dates`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FanDates {
    pub start: Option<String>,
    pub mid: Option<String>,
    pub horizon: Option<String>,
}

/// Parse `YYYY-MM-DD[THH:MM[:SS][Z]]` to epoch milliseconds (UTC), plus a crude
/// byte-lexicographic date key. Offsets other than `Z` are ignored.
fn parse_iso_ms(value: &str) -> Option<f64> {
    let bytes = value.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let year: i64 = value.get(0..4)?.parse().ok()?;
    let month: u32 = value.get(5..7)?.parse().ok()?;
    let day: u32 = value.get(8..10)?.parse().ok()?;
    let mut ms = days_from_civil(year, month, day) as f64 * 86_400_000.0;
    if let Some(rest) = value.get(11..) {
        let hms: Vec<&str> = rest.trim_end_matches('Z').split(':').collect();
        let h: f64 = hms.first().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let m: f64 = hms.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let s: f64 = hms.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        ms += h * 3_600_000.0 + m * 60_000.0 + s * 1000.0;
    }
    Some(ms)
}

fn weekday_abbr(index: i64) -> &'static str {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][index.rem_euclid(7) as usize]
}

fn month_abbr(month: u32) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month.saturating_sub(1)).min(11) as usize]
}

/// `fmtScrubDate` — `"Mon, Jul 15"`.
pub fn fmt_scrub_date(ms: f64) -> String {
    let day = (ms / 86_400_000.0).floor() as i64;
    let (_, month, date) = civil_from_days(day);
    format!(
        "{}, {} {}",
        weekday_abbr((day + 3).rem_euclid(7)),
        month_abbr(month),
        date
    )
}

/// `fmtAxisDate` — `"Jul 2025"`.
pub fn fmt_axis_date(value: &str) -> String {
    match value.get(0..4).and_then(|y| y.parse::<i64>().ok()) {
        Some(year) => match value.get(5..7).and_then(|m| m.parse::<u32>().ok()) {
            Some(month) => format!("{} {year}", month_abbr(month)),
            None => value.to_string(),
        },
        None => value.to_string(),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Projection {
    key: String,
    price: f64,
    analysts: i64,
    color: String,
    ty: f64,
    d: String,
    cx: f64,
    cy: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Overlay {
    px: f64,
    py: f64,
    title: String,
    metrics: Vec<(String, String)>,
    accent: Option<(String, String, String)>,
}

fn grid_label(_index: usize, value: f64, step: f64) -> String {
    if step >= 1.0 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}")
    }
}

/// `PriceTargetFan`.
#[component]
pub fn PriceTargetFan(
    #[props(default = "Price target · 12 months".to_string())] label: String,
    current: f64,
    targets: Vec<PriceTarget>,
    #[props(default)] dates: FanDates,
    #[props(default)] history: Vec<PriceHistoryPoint>,
    #[props(default)] active: Option<PriceTargetFanActive>,
    #[props(default)] default_active: Option<PriceTargetFanActive>,
    #[props(default)] on_active_change: Option<EventHandler<Option<PriceTargetFanActive>>>,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let can_hover = beui_core::use_hover_capable();
    let mut internal_active = use_signal(|| default_active);
    let controlled = active.is_some();
    use_effect(use_reactive((&active,), move |(active,)| {
        if let Some(active) = active {
            internal_active.set(Some(active));
        }
    }));
    let requested = active.clone().or(internal_active());
    let set_active = use_callback(move |next: Option<PriceTargetFanActive>| {
        if !controlled {
            let mut signal = internal_active;
            signal.set(next.clone());
        }
        if let Some(handler) = on_active_change {
            handler.call(next);
        }
    });

    // Sanitize the history: positive finite prices on ascending unique ISO dates.
    let mut hist: Vec<f64> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    let mut previous = f64::NEG_INFINITY;
    for point in history.iter() {
        let Some(time) = parse_iso_ms(&point.date) else {
            continue;
        };
        if time <= previous || !point.price.is_finite() || point.price <= 0.0 {
            continue;
        }
        previous = time;
        times.push(time);
        hist.push(point.price);
    }
    let valid_targets: Vec<PriceTarget> = targets
        .iter()
        .filter(|target| target.price.is_finite() && target.price > 0.0)
        .cloned()
        .collect();
    let targets = valid_targets;

    let target_index = match &requested {
        Some(PriceTargetFanActive::Target { key }) => {
            targets.iter().position(|target| &target.key == key)
        }
        _ => None,
    };
    let scrub = match &requested {
        Some(PriceTargetFanActive::History { date }) => {
            history.iter().position(|point| &point.date == date)
        }
        _ => None,
    };
    let hot_t = target_index.filter(|index| *index < targets.len());
    let scrub = scrub.filter(|index| *index < hist.len());

    let y_lo = hist
        .iter()
        .chain(std::iter::once(&current))
        .chain(targets.iter().map(|target| &target.price))
        .copied()
        .fold(f64::INFINITY, f64::min);
    let y_hi = hist
        .iter()
        .chain(std::iter::once(&current))
        .chain(targets.iter().map(|target| &target.price))
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let y_pad = ((y_hi - y_lo) * 0.06).max(0.5);
    let y_min = if y_lo.is_finite() { y_lo - y_pad } else { 0.0 };
    let y_max = if y_hi.is_finite() { y_hi + y_pad } else { 1.0 };
    let y = |value: f64| PAD_T + (1.0 - (value - y_min) / (y_max - y_min)) * (H - PAD_T - PAD_B);
    let pct = |price: f64| ((price - current) / current) * 100.0;
    let fmt = |price: f64| format!("${price:.2}");

    let hist_w = (W - PAD_L - PAD_R) * 0.56;
    let now_x = PAD_L + hist_w;
    let end_x = W - PAD_R;
    let now_y = y(current);
    let first = times.first().copied().unwrap_or(0.0);
    let last = times.last().copied().unwrap_or(first);
    let hx: Vec<f64> = times
        .iter()
        .map(|time| {
            if last == first {
                now_x - 12.0
            } else {
                PAD_L + ((time - first) / (last - first)) * (hist_w - 12.0)
            }
        })
        .collect();
    let mut line = String::new();
    for (index, value) in hist.iter().enumerate() {
        let command = if index == 0 { "M" } else { "L" };
        line.push_str(&format!("{command}{:.1},{:.1} ", hx[index], y(*value)));
    }
    if !hist.is_empty() {
        line.push_str(&format!("L{now_x:.1},{now_y:.1}"));
    }
    let projections: Vec<Projection> = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let ty = y(target.price);
            let cx = now_x + (end_x - now_x) * 0.5;
            let cy = now_y + (ty - now_y) * 0.15;
            Projection {
                key: target.key.clone(),
                price: target.price,
                analysts: target.analysts,
                color: target
                    .color
                    .clone()
                    .unwrap_or_else(|| [UP, MID, LOW][index].to_string()),
                ty,
                d: format!("M{now_x},{now_y} Q{cx},{cy} {end_x},{ty}"),
                cx,
                cy,
            }
        })
        .collect();

    // Overlay: a hovered target wins over the scrub.
    let mean = targets.get(1).cloned();
    let head_color = hot_t
        .and_then(|index| projections.get(index))
        .map(|projection| projection.color.clone());
    let head = match hot_t.and_then(|index| projections.get(index)) {
        Some(projection) => (projection.price, head_color.clone(), projection.key.clone()),
        None => match &mean {
            Some(mean) => (mean.price, mean.color.clone(), mean.key.clone()),
            None => (current, None, String::new()),
        },
    };
    let head_pct = pct(head.0);
    let head_ink = ink(head
        .1
        .as_deref()
        .unwrap_or(if head_pct >= 0.0 { UP } else { LOW }));
    let head_pct_text = format!(
        "{}{:.1}%",
        if head_pct >= 0.0 { "+" } else { "−" },
        head_pct.abs()
    );
    let overlay: Option<Overlay> = if let Some(index) = hot_t {
        projections.get(index).map(|projection| {
            let up = projection.price >= current;
            let horizon = dates
                .horizon
                .as_deref()
                .map(|horizon| format!(" · {horizon}"))
                .unwrap_or_default();
            Overlay {
                px: end_x,
                py: projection.ty,
                title: format!("{} target{horizon}", projection.key),
                metrics: vec![
                    ("Price".to_string(), fmt(projection.price)),
                    ("Analysts".to_string(), projection.analysts.to_string()),
                ],
                accent: Some((
                    "vs now".to_string(),
                    format!(
                        "{}{:.1}%",
                        if up { "+" } else { "−" },
                        pct(projection.price).abs()
                    ),
                    projection.color.clone(),
                )),
            }
        })
    } else {
        scrub.map(|index| Overlay {
            px: hx[index],
            py: y(hist[index]),
            title: fmt_scrub_date(times[index]),
            metrics: vec![("Price".to_string(), fmt(hist[index]))],
            accent: None,
        })
    };

    // Round-numbered gridlines, at most four.
    let span = y_max - y_min;
    let mag = 10f64.powf((span / 3.2).log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0]
        .iter()
        .map(|m| m * mag)
        .find(|s| span / s <= 4.2)
        .unwrap_or(10.0 * mag);
    let mut grid_vals: Vec<f64> = Vec::new();
    let mut value = (y_min / step).ceil() * step;
    while value <= y_max {
        grid_vals.push(value);
        value += step;
    }

    let mean_label = mean
        .as_ref()
        .map(|mean| fmt(mean.price))
        .unwrap_or_else(|| fmt(current));
    let svg_label = format!("{label}: mean {mean_label}, now {}", fmt(current));
    let date_axis = {
        let mut labels: Vec<(f64, String, &'static str)> = Vec::new();
        if !history.is_empty() {
            let start = dates
                .start
                .clone()
                .unwrap_or_else(|| fmt_axis_date(&history[0].date));
            labels.push((hx[0], start, "start"));
        }
        if history.len() > 2 {
            let mid_index = history.len() / 2;
            let mid = dates
                .mid
                .clone()
                .unwrap_or_else(|| fmt_axis_date(&history[mid_index].date));
            labels.push((hx[mid_index], mid, "middle"));
        }
        labels.push((now_x, "Now".to_string(), "middle"));
        labels.push((
            end_x,
            dates
                .horizon
                .clone()
                .unwrap_or_else(|| "Target".to_string()),
            "middle",
        ));
        labels
    };

    let overlay_style = overlay
        .as_ref()
        .map(|overlay| {
            format!(
                "left:{:.2}%;top:{:.2}%;",
                overlay.px / W * 100.0,
                overlay.py / H * 100.0,
            )
        })
        .unwrap_or_default();
    let svg_width = use_signal(|| 0.0f64);
    let svg_id = crate::common::use_unique_id("fan-svg");
    let svg_id_for_effect = svg_id.clone();
    use_effect(move || {
        let selector = format!("[data-beui-fan=\"{svg_id_for_effect}\"]");
        let script = format!(
            "(function(){{var el=document.querySelector({sel});if(!el)return;function send(){{dioxus.send({{left:0,top:0,width:el.clientWidth,height:el.clientHeight}});}}send();var ro=new ResizeObserver(send);ro.observe(el);}})();",
            sel = beui_core::platform::js_string(&selector),
        );
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(rect) = eval.recv::<beui_core::platform::Rect>().await {
                if (svg_width)() != rect.width {
                    let mut signal = svg_width;
                    signal.set(rect.width);
                }
            }
        });
    });

    let last_index = hist.len().saturating_sub(1);
    let cursor_index = scrub.unwrap_or(last_index);
    let cursor_date = history
        .get(cursor_index)
        .map(|point| point.date.clone())
        .unwrap_or_default();
    let radius = if svg_width() > 0.0 {
        svg_width() / W
    } else {
        1.0
    };
    let geo_now_x = now_x;
    let geo_now_y = now_y;
    let geo_end_x = end_x;
    let projections_for_move = projections.clone();
    let hx_for_move = hx.clone();
    let history_for_move = history.clone();
    let history_for_keys = history_for_move.clone();
    let set_active_move = set_active;
    let reduce_move = reduce;
    let _ = can_hover;

    rsx! {
        style { dangerous_inner_html: "{RING_KEYFRAMES}" }
        div {
            class: cn!("w-[520px] max-w-full [--ink-l:0.5] dark:[--ink-l:1]", class),
            div { class: "mb-1 flex items-end justify-between px-1",
                div { class: "flex flex-wrap items-center gap-2",
                    span { class: "font-mono text-2xl font-semibold tabular-nums tracking-tight text-foreground",
                        "{fmt(head.0)}"
                    }
                    span {
                        class: "font-mono text-xs font-medium tabular-nums",
                        style: "color:{head_ink};",
                        "{head_pct_text}"
                    }
                    span { class: "text-xs text-muted-foreground", "{head.2} target" }
                }
            }
            div { class: "relative",
                svg {
                    "data-beui-fan": "{svg_id}",
                    view_box: "0 0 {W} {H}",
                    class: "block h-auto w-full cursor-crosshair",
                    role: "group",
                    "aria-label": "{svg_label}",
                    onpointermove: move |event: PointerEvent| {
                        let coords = event.data().element_coordinates();
                        let px = coords.x / radius.max(f64::EPSILON);
                        if px > geo_now_x + 6.0 {
                            let py = coords.y / radius.max(f64::EPSILON);
                            let t = if geo_end_x != geo_now_x {
                                ((px - geo_now_x) / (geo_end_x - geo_now_x)).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let ys: Vec<f64> = projections_for_move
                                .iter()
                                .map(|p| (1.0 - t) * (1.0 - t) * geo_now_y + 2.0 * (1.0 - t) * t * p.cy + t * t * p.ty)
                                .collect();
                            let mut nearest = 0usize;
                            let mut best = f64::INFINITY;
                            for (index, cy) in ys.iter().enumerate() {
                                let distance = (py - cy).abs();
                                if distance < best {
                                    best = distance;
                                    nearest = index;
                                }
                            }
                            let spread = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                                - ys.iter().copied().fold(f64::INFINITY, f64::min);
                            if spread < 24.0 {
                                set_active_move.call(None);
                            } else if let Some(projection) = projections_for_move.get(nearest) {
                                set_active_move.call(Some(PriceTargetFanActive::Target { key: projection.key.clone() }));
                            }
                            return;
                        }
                        if history_for_move.is_empty() || px > geo_now_x - 6.0 {
                            set_active_move.call(None);
                            return;
                        }
                        let mut nearest = 0usize;
                        for index in 1..history_for_move.len() {
                            if (hx_for_move[index] - px).abs() < (hx_for_move[nearest] - px).abs() {
                                nearest = index;
                            }
                        }
                        if let Some(point) = history_for_move.get(nearest) {
                            set_active_move.call(Some(PriceTargetFanActive::History { date: point.date.clone() }));
                        }
                    },
                    onpointerleave: {
                        let set = set_active_move;
                        move |event: PointerEvent| {
                            if event.pointer_type() != "touch" {
                                set.call(None);
                            }
                        }
                    },
                    defs {
                        linearGradient {
                            id: "fan-fade",
                            gradient_units: "userSpaceOnUse",
                            x1: "0", y1: "{PAD_T}", x2: "0", y2: "{H - PAD_B}",
                            stop { offset: "0", "stop-color": "var(--border-strong)", "stop-opacity": "0" }
                            stop { offset: "0.45", "stop-color": "var(--border-strong)", "stop-opacity": "1" }
                            stop { offset: "1", "stop-color": "var(--border-strong)", "stop-opacity": "1" }
                        }
                    }
                    g {
                        for grid in grid_vals.iter() {
                            g { key: "{grid}",
                                line {
                                    x1: "{PAD_L}", y1: "{y(*grid)}", x2: "{end_x}", y2: "{y(*grid)}",
                                    stroke: "var(--border)", "stroke-dasharray": "2 5",
                                }
                                text {
                                    x: "{PAD_L - 8.0}", y: "{y(*grid) + 3.0}", "text-anchor": "end",
                                    font_size: "9", fill: "var(--muted-foreground)",
                                    class: "font-mono tabular-nums",
                                    {grid_label(grid_vals.iter().position(|v| v == grid).unwrap_or(0), *grid, step)}
                                }
                            }
                        }
                        for (x, text, anchor) in date_axis.iter() {
                            text {
                                key: "{x}",
                                x: "{x}", y: "{H - 8.0}", "text-anchor": "{anchor}",
                                font_size: "9", fill: "var(--muted-foreground)", class: "font-mono",
                                "{text}"
                            }
                        }
                        line {
                            x1: "{now_x}", y1: "{PAD_T}", x2: "{now_x}", y2: "{H - PAD_B}",
                            stroke: "url(#fan-fade)", "stroke-width": "1", "stroke-dasharray": "3 3",
                        }
                    }
                    g { role: "slider", tabindex: "0", "aria-label": "Price history",
                        "aria-valuemin": "0", "aria-valuemax": "{last_index}", "aria-valuenow": "{cursor_index}",
                        "aria-valuetext": "{cursor_date}",
                        onkeydown: {
                            let history = history_for_keys.clone();
                            let set = set_active_move;
                            let len = history.len();
                            move |event: KeyboardEvent| {
                                if event.key() == Key::Escape {
                                    event.prevent_default();
                                    set.call(None);
                                    return;
                                }
                                if !matches!(event.key(), Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End) {
                                    return;
                                }
                                event.prevent_default();
                                let current = scrub.unwrap_or(len.saturating_sub(1));
                                let next = if event.key() == Key::Home {
                                    0
                                } else if event.key() == Key::End {
                                    len.saturating_sub(1)
                                } else if event.key() == Key::ArrowRight {
                                    (current + 1).min(len.saturating_sub(1))
                                } else {
                                    current.saturating_sub(1)
                                };
                                if let Some(point) = history.get(next) {
                                    set.call(Some(PriceTargetFanActive::History { date: point.date.clone() }));
                                }
                            }
                        },
                        path {
                            "data-price-history": "true",
                            d: "{line}", fill: "none", stroke: "var(--foreground)",
                            "stroke-width": "1.5", "stroke-linecap": "round",
                        }
                    }
                    g {
                        for (index, projection) in projections.iter().enumerate() {
                            {
                                let on = hot_t == Some(index);
                                let dim = hot_t.is_some() && !on;
                                let opacity = if dim { 0.3 } else { 1.0 };
                                let width = if on { 2.2 } else { 1.4 };
                                let dash_opacity = if on { 1.0 } else { 0.75 };
                                let dot_r = if on { 4.5 } else { 3.2 };
                                let key = projection.key.clone();
                                let color = projection.color.clone();
                                let arc = projection.d.clone();
                                let set = set_active_move;
                                rsx! {
                                    g {
                                        key: "{projection.key}",
                                        role: "button",
                                        tabindex: "0",
                                        "aria-label": "{projection.key} target {fmt(projection.price)}, {projection.analysts} analysts",
                                        "aria-pressed": if on { "true" } else { "false" },
                                        style: "opacity:{opacity};",
                                        onfocus: {
                                            let key = key.clone();
                                            move |_| set.call(Some(PriceTargetFanActive::Target { key: key.clone() }))
                                        },
                                        onblur: move |_| set.call(None),
                                        onkeydown: {
                                            let key = key.clone();
                                            move |event: KeyboardEvent| {
                                                if event.key() == Key::Escape {
                                                    event.prevent_default();
                                                    set.call(None);
                                                }
                                                if event.key() == Key::Enter || event.key() == Key::Character(" ".to_string()) {
                                                    event.prevent_default();
                                                    set.call(if on { None } else { Some(PriceTargetFanActive::Target { key: key.clone() }) });
                                                }
                                            }
                                        },
                                        path { d: "{arc}", fill: "none", stroke: "transparent", "stroke-width": "16" }
                                        path {
                                            d: "{arc}", fill: "none", stroke: "{color}",
                                            "stroke-width": "{width}", "stroke-opacity": "{dash_opacity}",
                                            "stroke-dasharray": "2 4", "stroke-linecap": "round",
                                        }
                                        circle {
                                            cx: "{end_x}", cy: "{projection.ty}", fill: "var(--background)",
                                            stroke: "{color}", "stroke-width": "1.6", r: "{dot_r}",
                                        }
                                        text {
                                            x: "{end_x + 10.0}", y: "{projection.ty + 4.0}", font_size: "11",
                                            font_weight: "600", fill: "{ink(&color)}",
                                            class: "font-mono tabular-nums",
                                            "{projection.price}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    circle {
                        cx: "{now_x}", cy: "{now_y}", r: "3.2", fill: "var(--foreground)",
                    }
                    if !reduce_move {
                        circle {
                            cx: "{now_x}", cy: "{now_y}", r: "3.2", fill: "none",
                            stroke: "var(--foreground)", "stroke-width": "1",
                            style: "transform-origin:{now_x}px {now_y}px;animation:beui-fan-ring 2.2s ease-out 1.6s infinite;",
                        }
                    }
                    if let Some(index) = scrub {
                        g { "pointer-events": "none",
                            line {
                                x1: "{hx[index]}", y1: "{PAD_T}", x2: "{hx[index]}", y2: "{H - PAD_B}",
                                stroke: "url(#fan-fade)", "stroke-width": "1",
                            }
                            circle {
                                cx: "{hx[index]}", cy: "{y(hist[index])}", r: "3.2",
                                fill: "var(--foreground)", stroke: "var(--background)", "stroke-width": "1.5",
                            }
                        }
                    }
                }
                if let Some(overlay) = overlay {
                    div {
                        role: "tooltip",
                        class: "pointer-events-none absolute z-10 w-[148px] -translate-y-1/2 rounded-xl border border-border bg-popover px-3 py-2 text-popover-foreground shadow-md",
                        style: "{overlay_style}",
                        span { class: "block text-[10px] text-muted-foreground", "{overlay.title}" }
                        span { class: "mt-1.5 flex flex-col gap-1",
                            for (metric_label, metric_value) in overlay.metrics.iter() {
                                span { key: "{metric_label}", class: "flex items-center justify-between gap-3 text-xs",
                                    span { class: "font-medium text-foreground", "{metric_label}" }
                                    span { class: "font-mono tabular-nums text-foreground", "{metric_value}" }
                                }
                            }
                        }
                        if let Some((label, value, color)) = overlay.accent {
                            span { class: "mt-1.5 flex items-center justify-between gap-3 border-t border-border pt-1.5 text-xs",
                                span { class: "font-medium text-foreground", "{label}" }
                                span { class: "font-mono tabular-nums", style: "color:{ink(&color)};", "{value}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

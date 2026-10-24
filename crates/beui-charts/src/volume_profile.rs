//! `charts/volume-profile/` — executed volume by price range.
//!
//! `buildVolumeProfile` is ported: sorting by `priceLow`, validation, the POC
//! (equal maxima favor the lower price), and the value-area growth from the POC
//! using the larger adjacent volume (ties favor the nearer row, then the higher
//! price; the POC is always included, with a dedicated `target == 1` path). The
//! plot renders real price geometry including unequal bins and gaps.
//!
//! Differences from the source: the source throws on invalid bins; the
//! port silently skips them. The bar entrance/spring and the motion tooltip are
//! replaced by resolved bars + a lane-local bubble; `NumberTicker` is not ported;
//! inspection reads the hidden range's `input` event for pointer and keyboard.

use beui::prelude::*;

/// `VolumeProfileBin`.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeProfileBin {
    pub id: String,
    pub price_low: f64,
    pub price_high: f64,
    pub volume: f64,
}

impl VolumeProfileBin {
    pub fn new(id: impl Into<String>, price_low: f64, price_high: f64, volume: f64) -> Self {
        Self {
            id: id.into(),
            price_low,
            price_high,
            volume,
        }
    }
}

/// The resolved value area.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeValueArea {
    pub price_low: f64,
    pub price_high: f64,
    pub volume: f64,
    pub share: f64,
    pub target: f64,
}

/// `VolumeProfileRow`.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeProfileRow {
    pub id: String,
    pub price_low: f64,
    pub price_high: f64,
    pub volume: f64,
    pub price: f64,
    pub top: f64,
    pub height: f64,
    pub center: f64,
    pub proportion: f64,
    pub share: f64,
    pub is_poc: bool,
    pub in_value_area: bool,
}

/// `buildVolumeProfile` result.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeProfileBuild {
    pub rows: Vec<VolumeProfileRow>,
    pub poc: Option<usize>,
    pub value_area: Option<VolumeValueArea>,
    pub total_volume: f64,
    pub max_volume: f64,
    pub price_low: f64,
    pub price_high: f64,
}

/// `buildVolumeProfile`.
pub fn build_volume_profile(data: &[VolumeProfileBin], target: Option<f64>) -> VolumeProfileBuild {
    let target = target.filter(|target| target.is_finite() && *target > 0.0 && *target <= 1.0);
    let mut bins: Vec<VolumeProfileBin> = data.to_vec();
    bins.sort_by(|a, b| a.price_low.total_cmp(&b.price_low));
    let mut total_volume = 0.0f64;
    let mut max_volume = 0.0f64;
    let mut poc_index: i64 = -1;
    let mut valid: Vec<VolumeProfileBin> = Vec::new();
    for (index, bin) in bins.iter().enumerate() {
        let valid_bin = bin.price_low.is_finite()
            && bin.price_high.is_finite()
            && bin.price_high > bin.price_low
            && bin.volume.is_finite()
            && bin.volume >= 0.0
            && (index == 0 || bin.price_low >= bins[index - 1].price_high);
        if !valid_bin {
            continue;
        }
        total_volume += bin.volume;
        if bin.volume > max_volume {
            max_volume = bin.volume;
            poc_index = valid.len() as i64;
        }
        valid.push(bin.clone());
    }
    let price_low = valid.first().map(|bin| bin.price_low).unwrap_or(0.0);
    let price_high = valid.last().map(|bin| bin.price_high).unwrap_or(1.0);
    let span = if (price_high - price_low).abs() > f64::EPSILON {
        price_high - price_low
    } else {
        1.0
    };
    let mut low_index = poc_index;
    let mut high_index = poc_index;
    let mut area_volume = if poc_index >= 0 {
        valid[poc_index as usize].volume
    } else {
        0.0
    };
    if target == Some(1.0) && poc_index >= 0 {
        low_index = 0;
        high_index = valid.len() as i64 - 1;
        area_volume = total_volume;
    } else if let (Some(target), true) = (target, poc_index >= 0) {
        let target_volume = total_volume * target;
        while area_volume < target_volume {
            let below = (low_index > 0).then(|| valid[(low_index - 1) as usize].clone());
            let above = ((high_index as usize) + 1 < valid.len())
                .then(|| valid[(high_index + 1) as usize].clone());
            if below.is_none() && above.is_none() {
                break;
            }
            let take_above = match (&below, &above) {
                (None, Some(_)) => true,
                (Some(below), Some(above)) => {
                    above.volume > below.volume
                        || (above.volume == below.volume
                            && high_index + 1 - poc_index <= poc_index - (low_index - 1))
                }
                _ => false,
            };
            let next = if take_above { above } else { below };
            let Some(next) = next else { break };
            if area_volume + next.volume > target_volume {
                break;
            }
            area_volume += next.volume;
            if take_above {
                high_index += 1;
            } else {
                low_index -= 1;
            }
        }
    }
    let value_area = match (target, poc_index >= 0) {
        (Some(target), true) => Some(VolumeValueArea {
            price_low: valid[low_index as usize].price_low,
            price_high: valid[high_index as usize].price_high,
            volume: area_volume,
            share: if total_volume != 0.0 {
                area_volume / total_volume
            } else {
                0.0
            },
            target,
        }),
        _ => None,
    };
    let rows: Vec<VolumeProfileRow> = valid
        .iter()
        .enumerate()
        .map(|(index, bin)| {
            let price = bin.price_low + (bin.price_high - bin.price_low) / 2.0;
            VolumeProfileRow {
                id: bin.id.clone(),
                price_low: bin.price_low,
                price_high: bin.price_high,
                volume: bin.volume,
                price,
                top: (price_high - bin.price_high) / span,
                height: (bin.price_high - bin.price_low) / span,
                center: (price_high - price) / span,
                proportion: if max_volume != 0.0 {
                    bin.volume / max_volume
                } else {
                    0.0
                },
                share: if total_volume != 0.0 {
                    bin.volume / total_volume
                } else {
                    0.0
                },
                is_poc: index as i64 == poc_index,
                in_value_area: value_area.is_some()
                    && index as i64 >= low_index
                    && index as i64 <= high_index,
            }
        })
        .collect();
    VolumeProfileBuild {
        poc: (poc_index >= 0).then_some(poc_index as usize),
        rows,
        value_area,
        total_volume,
        max_volume,
        price_low,
        price_high,
    }
}

#[derive(Clone, Copy)]
struct VolumeContext {
    build: Signal<VolumeProfileBuild>,
    label: Signal<String>,
    unit: Signal<String>,
    color: Signal<String>,
    poc_color: Signal<String>,
    format_price: Option<Callback<f64, String>>,
    format_volume: Option<Callback<f64, String>>,
    active: Signal<Option<String>>,
    controlled: bool,
    on_active_change: Option<EventHandler<Option<String>>>,
    tooltip_open: Signal<bool>,
    tooltip_id: Signal<String>,
}

impl VolumeContext {
    fn price(&self, value: f64) -> String {
        match self.format_price {
            Some(callback) => callback.call(value),
            None => crate::donut_chart::default_format(value),
        }
    }

    fn volume(&self, value: f64) -> String {
        match self.format_volume {
            Some(callback) => callback.call(value),
            None => crate::donut_chart::default_format(value),
        }
    }

    fn active_index(&self) -> Option<usize> {
        let id = (self.active)()?;
        (self.build)().rows.iter().position(|row| row.id == id)
    }

    fn set_active(&self, id: Option<String>) {
        if !self.controlled {
            let mut active = self.active;
            active.set(id.clone());
        }
        if let Some(handler) = self.on_active_change {
            handler.call(id);
        }
    }

    fn describe(&self, row: &VolumeProfileRow) -> String {
        let mut parts = vec![
            format!(
                "{} to {}",
                self.price(row.price_low),
                self.price(row.price_high)
            ),
            format!("{} {}", self.volume(row.volume), (self.unit)()),
            format!("{:.1}% of total volume", row.share * 100.0),
        ];
        if row.is_poc {
            parts.push("Point of control".to_string());
        } else if row.in_value_area {
            parts.push("Inside value area".to_string());
        }
        parts.join(". ")
    }
}

fn share_text(share: f64) -> String {
    format!("{:.1}%", share * 100.0)
}

/// `VolumeProfile`.
#[component]
pub fn VolumeProfile(
    data: Vec<VolumeProfileBin>,
    #[props(default = "Volume profile".to_string())] label: String,
    #[props(default = "units".to_string())] unit: String,
    #[props(default)] format_price: Option<Callback<f64, String>>,
    #[props(default)] format_volume: Option<Callback<f64, String>>,
    #[props(default = Some(0.7))] value_area: Option<f64>,
    #[props(default = "#6366f1".to_string())] color: String,
    #[props(default = "#f59e0b".to_string())] poc_color: String,
    #[props(default)] active_id: Option<String>,
    #[props(default)] default_active_id: Option<String>,
    #[props(default)] on_active_change: Option<EventHandler<Option<String>>>,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let build = build_volume_profile(&data, value_area);
    let build_signal = use_signal(|| build);
    let label_signal = use_signal(|| label.clone());
    let unit_signal = use_signal(|| unit.clone());
    let color_signal = use_signal(|| color.clone());
    let poc_signal = use_signal(|| poc_color.clone());
    let mut internal_active = use_signal(|| default_active_id.clone());
    let tooltip_open = use_signal(|| false);
    let tooltip_id_value = crate::common::use_unique_id("volume-tip");
    let tooltip_id = use_signal(|| tooltip_id_value.clone());
    let controlled = active_id.is_some();
    use_effect(use_reactive((&active_id,), move |(active_id,)| {
        if let Some(active_id) = active_id {
            internal_active.set(Some(active_id));
        }
    }));

    let context = VolumeContext {
        build: build_signal,
        label: label_signal,
        unit: unit_signal,
        color: color_signal,
        poc_color: poc_signal,
        format_price,
        format_volume,
        active: internal_active,
        controlled,
        on_active_change,
        tooltip_open,
        tooltip_id,
    };
    use_context_provider(|| context);

    let rows = (context.build)().rows;
    rsx! {
        section {
            "aria-label": "{label}",
            "data-slot": "volume-profile",
            class: cn!("min-w-0 w-full space-y-5", class),
            ..attributes,
            if let Some(children) = children {
                {children}
            } else {
                VolumeProfilePlot {}
                VolumeProfileTooltip {}
                VolumeProfileSummary {}
            }
            div { class: "sr-only",
                table {
                    caption { "{label} data" }
                    thead {
                        tr {
                            th { scope: "col", "Price range" }
                            th { scope: "col", "Volume ({unit})" }
                            th { scope: "col", "Share" }
                            th { scope: "col", "Level" }
                        }
                    }
                    tbody {
                        for row in rows.iter() {
                            tr { key: "{row.id}",
                                th { scope: "row", "{context.price(row.price_low)} to {context.price(row.price_high)}" }
                                td { "{context.volume(row.volume)}" }
                                td { "{share_text(row.share)}" }
                                td {
                                    if row.is_poc { "Point of control" } else if row.in_value_area { "Inside value area" } else if (context.build)().value_area.is_some() { "Outside value area" } else { "—" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `VolumeProfilePlot`.
#[component]
pub fn VolumeProfilePlot(
    #[props(default)] class: String,
    #[props(default)] bar_class_name: String,
    #[props(default = true)] show_axes: bool,
) -> Element {
    let context = use_context::<VolumeContext>();
    let reduce = use_reduced_motion()();
    let build = (context.build)();
    let rows = build.rows.clone();
    if rows.is_empty() {
        return rsx! {
            p { class: cn!("py-12 text-center text-sm text-muted-foreground", class), "No volume data" }
        };
    }
    let active_index = context.active_index();
    let index = active_index.unwrap_or(0);
    let row = rows[index].clone();
    let value_area_on = build.value_area.is_some();
    let color = (context.color)();
    let poc_color = (context.poc_color)();
    let price_low = build.price_low;
    let price_high = build.price_high;
    let span = price_high - price_low;
    let max_volume = build.max_volume;
    let label = (context.label)();
    let valuetext = context.describe(&row);
    let tooltip_open = (context.tooltip_open)();
    let tooltip_id = (context.tooltip_id)();
    let row_count = rows.len();
    let _ = reduce;

    let axis_positions: [f64; 5] = [1.0, 0.75, 0.5, 0.25, 0.0];
    let bar_class = cn!("h-full w-full rounded-r-sm", bar_class_name);

    rsx! {
        div { "data-slot": "volume-profile-plot", class: cn!("min-w-0", if show_axes { "pb-6" } else { "" }),
            div { class: cn!("flex h-80 min-w-0 gap-3", class),
                if show_axes {
                    div {
                        "aria-hidden": "true",
                        class: "relative shrink-0 text-right font-mono text-[10px] tabular-nums text-muted-foreground sm:text-xs",
                        for position in axis_positions.iter() {
                            {
                                let axis_top = (1.0 - *position) * 100.0;
                                rsx! {
                                    span { key: "{position}",
                                        span { class: "invisible block", "{context.price(price_low + span * position)}" }
                                        span {
                                            class: "absolute right-0 -translate-y-1/2",
                                            style: "top:{axis_top}%;",
                                            "{context.price(price_low + span * position)}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "relative min-w-0 flex-1 rounded-sm has-focus-visible:outline-2 has-focus-visible:outline-offset-4 has-focus-visible:outline-ring",
                    div { "aria-hidden": "true", class: "pointer-events-none absolute inset-0",
                        for position in [0.0, 0.25, 0.5, 0.75, 1.0] {
                            {
                                let grid_left = position * 100.0;
                                rsx! {
                                    span {
                                        key: "grid{position}",
                                        class: "absolute inset-y-0 border-l border-border/60",
                                        style: "left:{grid_left}%;",
                                    }
                                }
                            }
                        }
                        for (bin_index, item) in rows.iter().enumerate() {
                            {
                                let is_active = active_index == Some(bin_index);
                                let bar_opacity = if item.is_poc || item.in_value_area || !value_area_on { 0.9 } else { 0.3 };
                                let bar_color = if item.is_poc { poc_color.clone() } else { color.clone() };
                                let bar_style = format!(
                                    "background-color:{bar_color};transform:scaleX({});transform-origin:left;opacity:{bar_opacity};transition:transform 300ms,opacity 180ms;",
                                    item.proportion,
                                );
                                let highlight = if tooltip_open && is_active { 1.0 } else { 0.0 };
                                let bin_style = format!("top:{}%;height:{}%;", item.top * 100.0, item.height * 100.0);
                                rsx! {
                                    div {
                                        key: "{item.id}",
                                        "data-volume-bin": "{item.id}",
                                        "data-poc": if item.is_poc { Some("true") } else { None },
                                        "data-value-area": if item.in_value_area { Some("true") } else { None },
                                        class: "absolute inset-x-0 overflow-hidden py-px",
                                        style: "{bin_style}",
                                        div { class: "{bar_class}", style: "{bar_style}" }
                                        div { class: "absolute inset-0 bg-foreground/5", style: "opacity:{highlight};transition:opacity 100ms;" }
                                        if item.is_poc {
                                            span {
                                                class: "absolute inset-x-0 top-1/2 border-t border-dashed",
                                                style: "border-color:{poc_color};",
                                                span { class: "absolute right-0 -translate-y-1/2 rounded bg-background px-1.5 py-0.5 font-mono text-[10px] font-medium text-foreground",
                                                    "POC"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "{row_count - 1}",
                        step: "1",
                        value: "{index}",
                        "aria-label": "{label}: inspect price level",
                        "aria-orientation": "vertical",
                        "aria-valuetext": "{valuetext}",
                        "aria-describedby": if tooltip_open && active_index.is_some() { Some(tooltip_id.clone()) } else { None },
                        class: "absolute inset-0 m-0 h-full w-full cursor-crosshair opacity-0",
                        style: "writing-mode:vertical-lr;direction:rtl;touch-action:pan-x;",
                        onfocus: {
                            let mut open = context.tooltip_open;
                            let row_id = row.id.clone();
                            move |_| {
                                context.set_active(Some(row_id.clone()));
                                open.set(true);
                            }
                        },
                        onblur: {
                            let mut open = context.tooltip_open;
                            move |_| open.set(false)
                        },
                        oninput: {
                            let rows = rows.clone();
                            move |event| {
                                if let Ok(next) = event.value().parse::<usize>() {
                                    if let Some(row) = rows.get(next) {
                                        context.set_active(Some(row.id.clone()));
                                        let mut open = context.tooltip_open;
                                        open.set(true);
                                    }
                                }
                            }
                        },
                        onkeydown: {
                            let mut open = context.tooltip_open;
                            move |event: KeyboardEvent| {
                                if event.key() == Key::Escape {
                                    open.set(false);
                                } else if matches!(
                                    event.key(),
                                    Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End
                                ) {
                                    open.set(true);
                                }
                            }
                        },
                    }
                    if tooltip_open && active_index.is_some() {
                        {
                            let top = row.center * 100.0;
                            let note = if row.is_poc { Some("Point of control") } else if row.in_value_area { Some("Inside value area") } else { None };
                            let unit = (context.unit)();
                            let range = format!("{} – {}", context.price(row.price_low), context.price(row.price_high));
                            let volume = format!("{} {}", context.volume(row.volume), unit);
                            let share = share_text(row.share);
                            rsx! {
                                div {
                                    id: "{tooltip_id}",
                                    role: "tooltip",
                                    class: "pointer-events-none absolute left-full top-0 z-10 ml-2 w-max max-w-56 -translate-y-1/2 rounded-xl border border-border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md",
                                    style: "top:{top}%;",
                                    span { class: "flex flex-col gap-1",
                                        span { class: "text-muted-foreground", "{range}" }
                                        span { class: "font-mono font-medium tabular-nums", "{volume}" }
                                        span { class: "text-muted-foreground", "{share} of total volume" }
                                        if let Some(note) = note {
                                            span { "{note}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if show_axes {
                        div {
                            "aria-hidden": "true",
                            class: "absolute inset-x-0 top-full mt-2 flex justify-between font-mono text-[10px] tabular-nums text-muted-foreground sm:text-xs",
                            span { "0" }
                            span { "{context.volume(max_volume / 2.0)}" }
                            span { "{context.volume(max_volume)}" }
                        }
                    }
                }
            }
        }
    }
}

/// `VolumeProfileTooltip` — a marker; the plot renders the bubble.
#[component]
pub fn VolumeProfileTooltip(#[props(default)] class: String) -> Element {
    let _ = class;
    rsx! {}
}

/// `VolumeProfileSummary`.
#[component]
pub fn VolumeProfileSummary(#[props(default)] class: String) -> Element {
    let context = use_context::<VolumeContext>();
    let build = (context.build)();
    let rows = build.rows.clone();
    if rows.is_empty() {
        return rsx! {};
    }
    let unit = (context.unit)();
    let color = (context.color)();
    let poc_color = (context.poc_color)();
    let poc_price = build
        .poc
        .and_then(|index| rows.get(index))
        .map(|row| context.price(row.price))
        .unwrap_or_else(|| "—".to_string());
    let value_area = build.value_area.clone();
    let target_text = value_area
        .as_ref()
        .map(|area| format!("{:.0}", area.target * 100.0))
        .unwrap_or_default();
    let share_text_value = value_area
        .as_ref()
        .map(|area| format!("{:.1}% of volume", area.share * 100.0))
        .unwrap_or_default();
    rsx! {
        dl {
            class: cn!(
                "grid grid-cols-2 gap-x-4 gap-y-4 border-t border-border pt-4 text-xs sm:grid-cols-3",
                class,
            ),
            div { class: "space-y-1.5",
                dt { class: "text-muted-foreground", "Total volume" }
                dd { class: "font-mono font-medium tabular-nums", "{context.volume(build.total_volume)} {unit}" }
            }
            div { class: "space-y-1.5",
                dt { class: "flex items-center gap-1.5 text-muted-foreground",
                    span {
                        "aria-hidden": "true",
                        class: "size-1.5 rounded-full",
                        style: "background-color:{poc_color};",
                    }
                    "Point of control"
                }
                dd { class: "font-mono font-medium tabular-nums", "{poc_price}" }
            }
            if let Some(area) = value_area {
                div { class: "col-span-2 space-y-1.5 sm:col-span-1",
                    dt { class: "flex items-center gap-1.5 text-muted-foreground",
                        span {
                            "aria-hidden": "true",
                            class: "size-1.5 rounded-full",
                            style: "background-color:{color};",
                        }
                        "Value area · {target_text}% target"
                    }
                    dd { class: "font-mono font-medium tabular-nums",
                        "{context.price(area.price_low)} – {context.price(area.price_high)}"
                    }
                    dd { class: "text-muted-foreground", "{share_text_value}" }
                }
            }
        }
    }
}

//! `charts/donut-chart/` — a ring of rounded segments with a linked center
//! readout and legend.
//!
//! `buildDonut` is ported verbatim (arc geometry, cap inset, the two-half-arc
//! single-segment case). The model is a context shared by the plot, center and
//! legend, so hovering a legend row pops its segment exactly like the source.
//!
//! Differences from the source:
//! - `DigitSwap` is not ported; the center value is a formatted `<p>` that swaps
//!   in place (the caption slides via a CSS transition).
//! - the center's `children` render function is not ported (Dioxus takes an
//!   `Element`, not a `(segment, total) => Element`); pass `children` to replace
//!   the default content.
//! - the ring's entrance and focus emphasis are CSS transitions; the per-segment
//!   stagger is applied on first paint only.

use std::f64::consts::PI;

use beui::prelude::*;

/// `DONUT_SIZE`.
pub const DONUT_SIZE: f64 = 200.0;
const CENTER: f64 = DONUT_SIZE / 2.0;

const RING_ORIGIN: &str = "transform-box:view-box;transform-origin:100px 100px;";

/// `DonutSegment`.
#[derive(Clone, Debug, PartialEq)]
pub struct DonutSegment {
    pub id: String,
    pub name: String,
    pub color: String,
    pub value: f64,
}

impl DonutSegment {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        color: impl Into<String>,
        value: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color: color.into(),
            value,
        }
    }
}

/// `PlacedSegment`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedSegment {
    pub id: String,
    pub name: String,
    pub color: String,
    pub value: f64,
    pub share: f64,
    pub path: String,
}

fn point(radius: f64, degrees: f64) -> (f64, f64) {
    let radians = degrees * PI / 180.0;
    (
        CENTER + radius * radians.sin(),
        CENTER - radius * radians.cos(),
    )
}

/// `buildDonut`.
pub fn build_donut(
    segments: &[DonutSegment],
    radius: f64,
    thickness: f64,
    gap: f64,
) -> (Vec<PlacedSegment>, f64) {
    let valid: Vec<&DonutSegment> = segments
        .iter()
        .filter(|segment| segment.value.is_finite() && segment.value > 0.0)
        .collect();
    let total: f64 = valid.iter().map(|segment| segment.value).sum();
    let cap_degrees = (thickness / 2.0 / radius) * (180.0 / PI);
    let single = valid.len() == 1;
    let mut cursor = 0.0;
    let placed = valid
        .iter()
        .map(|segment| {
            let share = if total != 0.0 { segment.value / total } else { 0.0 };
            let sweep = share * 360.0;
            let inset = if single { 0.0 } else { gap / 2.0 + cap_degrees };
            let start = cursor + inset;
            let end = (start + 0.01).max(cursor + sweep - inset);
            cursor += sweep;
            let path = if single {
                let (x1, y1) = point(radius, 0.0);
                let (x2, y2) = point(radius, 180.0);
                format!(
                    "M {x1} {y1} A {radius} {radius} 0 1 1 {x2} {y2} A {radius} {radius} 0 1 1 {x1} {y1}"
                )
            } else {
                let (x1, y1) = point(radius, start);
                let (x2, y2) = point(radius, end);
                let large = if end - start > 180.0 { 1 } else { 0 };
                format!("M {x1} {y1} A {radius} {radius} 0 {large} 1 {x2} {y2}")
            };
            PlacedSegment {
                id: segment.id.clone(),
                name: segment.name.clone(),
                color: segment.color.clone(),
                value: segment.value,
                share,
                path,
            }
        })
        .collect();
    (placed, total)
}

/// `defaultFormat` — `toLocaleString("en-US")` without the locale dependency.
pub fn default_format(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let text = format!("{rounded:.3}");
    let negative = text.starts_with('-');
    let text = text.trim_start_matches('-');
    let (integer, fraction) = text.split_once('.').unwrap_or((text, ""));
    let fraction = fraction.trim_end_matches('0');
    let mut grouped = String::new();
    for (index, ch) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let mut out = if fraction.is_empty() {
        grouped
    } else {
        format!("{grouped}.{fraction}")
    };
    if negative && out != "0" {
        out.insert(0, '-');
    }
    out
}

#[derive(Clone, Copy)]
struct DonutContext {
    placed: Signal<Vec<PlacedSegment>>,
    total: Signal<f64>,
    radius: Signal<f64>,
    thickness: Signal<f64>,
    active: Signal<Option<String>>,
    on_active_change: Option<EventHandler<Option<String>>>,
    controlled: bool,
    on_select: Option<EventHandler<String>>,
    selected_id: Signal<Option<String>>,
    format_value: Option<Callback<f64, String>>,
    label: Signal<String>,
}

impl DonutContext {
    fn format(&self, value: f64) -> String {
        match self.format_value {
            Some(callback) => callback.call(value),
            None => default_format(value),
        }
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
}

/// `DonutChart`.
#[component]
pub fn DonutChart(
    segments: Vec<DonutSegment>,
    #[props(default = 22.0)] thickness: f64,
    #[props(default = 3.0)] gap: f64,
    #[props(default)] active_id: Option<String>,
    #[props(default)] default_active_id: Option<String>,
    #[props(default)] on_active_change: Option<EventHandler<Option<String>>>,
    #[props(default)] on_select: Option<EventHandler<String>>,
    #[props(default)] selected_id: Option<String>,
    #[props(default)] format_value: Option<Callback<f64, String>>,
    #[props(default = "Donut chart".to_string())] label: String,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let radius = 100.0 - thickness / 2.0 - 8.0;
    let (placed, total) = build_donut(&segments, radius, thickness, gap);
    let placed_signal = use_signal(|| placed);
    let total_signal = use_signal(|| total);
    let radius_signal = use_signal(|| radius);
    let thickness_signal = use_signal(|| thickness);
    let label_signal = use_signal(|| label.clone());
    let mut internal_active = use_signal(|| default_active_id.clone());
    let selected_signal = use_signal(|| selected_id.clone());
    let controlled = active_id.is_some();

    use_effect(use_reactive((&active_id,), move |(active_id,)| {
        if let Some(active_id) = active_id {
            internal_active.set(Some(active_id));
        }
    }));

    let context = DonutContext {
        placed: placed_signal,
        total: total_signal,
        radius: radius_signal,
        thickness: thickness_signal,
        active: internal_active,
        on_active_change,
        controlled,
        on_select,
        selected_id: selected_signal,
        format_value,
        label: label_signal,
    };
    use_context_provider(|| context);

    rsx! {
        div { class: cn!("w-full", class),
            if let Some(children) = children {
                {children}
            } else {
                div { class: "grid items-center gap-6 sm:grid-cols-[minmax(0,16rem)_1fr]",
                    DonutChartPlot { DonutChartCenter {} }
                    DonutChartLegend {}
                }
            }
        }
    }
}

/// `DonutChartPlot`.
#[component]
pub fn DonutChartPlot(
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let context = use_context::<DonutContext>();
    let reduce = use_reduced_motion()();
    let can_hover = beui_core::use_hover_capable();
    let mut in_view = use_signal(|| reduce);
    let mut entered = use_signal(|| reduce);

    // `useInView(ref, { once: true, amount: 0.4 })` — install an
    // IntersectionObserver and report the first intersection.
    let plot_id = crate::common::use_unique_id("donut-plot");
    let plot_id_for_effect = plot_id.clone();
    use_effect(move || {
        if in_view() {
            return;
        }
        let selector = format!("[data-beui-donut-plot=\"{plot_id_for_effect}\"]");
        let script = format!(
            "(function(){{var el=document.querySelector({sel});if(!el)return;var io=new IntersectionObserver(function(entries){{entries.forEach(function(e){{if(e.isIntersecting){{dioxus.send(true);io.disconnect();}}}});}},{{threshold:0.4}});io.observe(el);}})();",
            sel = beui_core::platform::js_string(&selector),
        );
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            if eval.recv::<bool>().await.is_ok() {
                in_view.set(true);
            }
        });
    });
    // The entrance stagger only applies to the first sweep.
    use_effect(move || {
        let Some(mut eval) = beui_core::platform::eval("setTimeout(()=>dioxus.send(true),900);")
        else {
            return;
        };
        spawn(async move {
            if eval.recv::<bool>().await.is_ok() {
                entered.set(true);
            }
        });
    });

    let placed = (context.placed)();
    let active = (context.active)();
    let shown = in_view() || reduce;
    let radius = (context.radius)();
    let thickness = (context.thickness)();
    let label = (context.label)();
    let _ = can_hover;

    let (rotate, scale, opacity) = if reduce || shown {
        (0.0, 1.0, 1.0)
    } else {
        (-90.0, 0.88, 0.0)
    };
    let group_style = format!(
        "{RING_ORIGIN}transform:rotate({rotate}deg) scale({scale});opacity:{opacity};transition:transform 900ms cubic-bezier(0.16,1,0.3,1),opacity 900ms cubic-bezier(0.16,1,0.3,1);"
    );

    rsx! {
        div {
            "data-beui-donut-plot": "{plot_id}",
            class: cn!("relative mx-auto aspect-square w-full max-w-64", class),
            svg {
                view_box: "0 0 200 200",
                role: "img",
                "aria-label": "{label}",
                class: "size-full overflow-visible",
                circle {
                    cx: "100",
                    cy: "100",
                    r: "{radius}",
                    fill: "none",
                    "stroke-width": "{thickness}",
                    class: "stroke-muted",
                    opacity: "0.6",
                }
                g {
                    style: "{group_style}",
                    for (index, segment) in placed.iter().enumerate() {
                        {
                            let focused = active.as_deref() == Some(segment.id.as_str());
                            let dimmed = active.is_some() && !focused;
                            let delay = if entered() { 0.0 } else { 0.15 + index as f64 * 0.06 };
                            let path_style = format!(
                                "{RING_ORIGIN}opacity:{};transform:scale({});transition:opacity 400ms cubic-bezier(0.16,1,0.3,1) {delay}s,transform 300ms cubic-bezier(0.34,1.3,0.64,1);",
                                if dimmed { 0.3 } else { 1.0 },
                                if focused { 1.05 } else { 1.0 },
                            );
                            rsx! {
                                path {
                                    key: "{segment.id}",
                                    d: "{segment.path}",
                                    fill: "none",
                                    stroke: "{segment.color}",
                                    "stroke-width": "{thickness}",
                                    "stroke-linecap": "round",
                                    "pointer-events": "none",
                                    "aria-hidden": "true",
                                    style: "{path_style}",
                                }
                            }
                        }
                    }
                }
                g {
                    for segment in placed.iter() {
                        path {
                            key: "{segment.id}",
                            d: "{segment.path}",
                            fill: "none",
                            stroke: "transparent",
                            "stroke-width": "{thickness + 8.0}",
                            "stroke-linecap": "round",
                            "pointer-events": if shown { "stroke" } else { "none" },
                            class: "cursor-pointer",
                            onpointerenter: {
                                let id = segment.id.clone();
                                move |_| {
                                    if can_hover() {
                                        context.set_active(Some(id.clone()));
                                    }
                                }
                            },
                            onpointerleave: move |_| {
                                if can_hover() {
                                    context.set_active(None);
                                }
                            },
                        }
                    }
                }
            }
            if let Some(children) = children {
                div { class: "pointer-events-none absolute inset-0 flex items-center justify-center text-center",
                    {children}
                }
            }
        }
    }
}

/// `DonutChartCenter`.
#[component]
pub fn DonutChartCenter(
    #[props(default = "Total".to_string())] total_label: String,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let context = use_context::<DonutContext>();
    let placed = (context.placed)();
    let active = (context.active)();
    let segment = active
        .as_ref()
        .and_then(|id| placed.iter().find(|item| &item.id == id))
        .map(ToOwned::to_owned);
    let value = segment
        .as_ref()
        .map(|segment| segment.value)
        .unwrap_or((context.total)());
    let formatted = context.format(value);
    let is_total = segment.is_none();
    let share_text = segment
        .as_ref()
        .map(|segment| format!("{:.1}", segment.share * 100.0))
        .unwrap_or_default();

    rsx! {
        div { "aria-live": "polite", class: cn!("relative max-w-[56%]", class),
            if let Some(children) = children {
                {children}
            } else {
                p { class: "text-2xl font-medium tracking-tight tabular-nums", "{formatted}" }
                div { class: "relative mt-1 h-4 overflow-hidden",
                    if let Some(segment) = segment {
                        div { class: "flex items-center justify-center gap-1.5 whitespace-nowrap text-xs leading-4 text-muted-foreground",
                            span {
                                "aria-hidden": "true",
                                class: "size-2 shrink-0 rounded-full",
                                style: "background-color:{segment.color};",
                            }
                            span { class: "truncate", "{segment.name} · {share_text}%" }
                        }
                    } else {
                        div { class: "flex items-center justify-center text-xs leading-4 text-muted-foreground",
                            "{total_label}"
                        }
                    }
                }
            }
            if is_total {
                span { class: "sr-only", "Total" }
            }
        }
    }
}

/// `DonutChartLegend`.
#[component]
pub fn DonutChartLegend(
    #[props(default)] render_detail: Option<Callback<PlacedSegment, Element>>,
    #[props(default)] render_value: Option<Callback<PlacedSegment, Element>>,
    #[props(default)] class: String,
) -> Element {
    let context = use_context::<DonutContext>();
    let placed = (context.placed)();
    let active = (context.active)();
    let can_hover = beui_core::use_hover_capable();
    let len = placed.len();

    rsx! {
        ul { class: cn!("space-y-1", class),
            for (index, segment) in placed.iter().enumerate() {
                {
                    let focused = active.as_deref() == Some(segment.id.as_str());
                    let selected = (context.selected_id)();
                    let share = format!("{:.1}", segment.share * 100.0);
                    let formatted = context.format(segment.value);
                    let detail = render_detail.map(|callback| callback.call(segment.clone()));
                    let custom_value = render_value.map(|callback| callback.call(segment.clone()));
                    rsx! {
                        li { key: "{segment.id}",
                            button {
                                r#type: "button",
                                "data-donut-legend": "true",
                                "aria-pressed": if context.on_select.is_some() { Some(if selected.as_deref() == Some(segment.id.as_str()) { "true" } else { "false" }) } else { None },
                                onclick: {
                                    let id = segment.id.clone();
                                    move |_| {
                                        if let Some(handler) = context.on_select {
                                            handler.call(id.clone());
                                        }
                                    }
                                },
                                onpointerenter: {
                                    let id = segment.id.clone();
                                    move |_| {
                                        if can_hover() {
                                            context.set_active(Some(id.clone()));
                                        }
                                    }
                                },
                                onpointerleave: move |_| {
                                    if can_hover() {
                                        context.set_active(None);
                                    }
                                },
                                onfocus: {
                                    let id = segment.id.clone();
                                    move |_| context.set_active(Some(id.clone()))
                                },
                                onblur: move |_| context.set_active(None),
                                onkeydown: {
                                    move |event: KeyboardEvent| {
                                        let step = if event.key() == Key::ArrowDown {
                                            1isize
                                        } else if event.key() == Key::ArrowUp {
                                            -1
                                        } else {
                                            0
                                        };
                                        if step == 0 {
                                            return;
                                        }
                                        event.prevent_default();
                                        let next = ((index as isize + step).rem_euclid(len as isize)) as usize;
                                        beui_core::platform::fire(&format!(
                                            "(function(){{var rows=document.querySelectorAll('[data-donut-legend]');if(rows[{}])rows[{}].focus();}})();",
                                            next, next,
                                        ));
                                    }
                                },
                                class: cn!(
                                    "w-full rounded-xl px-3 py-2 text-left outline-none transition-colors duration-150",
                                    "focus-visible:ring-2 focus-visible:ring-ring",
                                    if focused { "bg-muted" } else { "hover:bg-muted/60" },
                                    if selected.as_deref() == Some(segment.id.as_str()) { "ring-1 ring-border" } else { "" },
                                ),
                                span { class: "flex items-center gap-3",
                                    span {
                                        "aria-hidden": "true",
                                        class: "size-2.5 shrink-0 rounded-full",
                                        style: "background-color:{segment.color};",
                                    }
                                    span { class: "min-w-0 flex-1 truncate text-sm font-medium", "{segment.name}" }
                                    span { class: "shrink-0 text-right text-sm tabular-nums",
                                        if let Some(custom_value) = custom_value {
                                            {custom_value}
                                        } else {
                                            span { class: "font-medium", "{share}%" }
                                            span { class: "ml-2 text-muted-foreground", "{formatted}" }
                                        }
                                    }
                                }
                                if let Some(detail) = detail {
                                    span { class: "mt-0.5 block pl-5.5 text-xs text-muted-foreground", {detail} }
                                }
                                span {
                                    "aria-hidden": "true",
                                    class: "mt-2 ml-5.5 block h-1 overflow-hidden rounded-full bg-muted",
                                    span {
                                        class: "block h-full origin-left rounded-full",
                                        style: "background-color:{segment.color};transform:scaleX({segment.share});",
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

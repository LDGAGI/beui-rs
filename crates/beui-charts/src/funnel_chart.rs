//! `charts/funnel-chart/` — ordered conversion stages.
//!
//! `buildFunnel` (validity/dedupe, proportion, conversion, step conversion,
//! change) and `funnelPath` (shared curved boundaries between stages) are
//! ported verbatim. The model is a context shared by the plot and summary.
//!
//! Differences from the source: the source interpolates each stage's `d`
//! through motion on direction/proportion changes; the port renders the final
//! path. `NumberTicker` is not ported, and the hover tooltip is a lane-local
//! bubble rather than motion's `Tooltip`.

use beui::prelude::*;

/// `colors`.
pub const FUNNEL_COLORS: [&str; 5] = ["#8b5cf6", "#7774ef", "#548ee4", "#2ca6bc", "#14b8a6"];

/// `FunnelStage`.
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelStage {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
}

impl FunnelStage {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            color: None,
        }
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// A resolved funnel row.
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelRow {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
    pub proportion: f64,
    pub conversion: Option<f64>,
    pub step_conversion: Option<f64>,
    pub change: Option<f64>,
}

/// `buildFunnel` result.
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelBuild {
    pub rows: Vec<FunnelRow>,
    pub first: f64,
    pub last: f64,
    pub conversion: Option<f64>,
}

/// `buildFunnel`.
pub fn build_funnel(stages: &[FunnelStage]) -> FunnelBuild {
    let mut seen: Vec<String> = Vec::new();
    let mut valid: Vec<&FunnelStage> = Vec::new();
    for stage in stages {
        if seen.contains(&stage.id) || !stage.value.is_finite() || stage.value < 0.0 {
            continue;
        }
        seen.push(stage.id.clone());
        valid.push(stage);
    }
    let first = valid.first().map(|stage| stage.value).unwrap_or(0.0);
    let last = valid.last().map(|stage| stage.value).unwrap_or(0.0);
    let maximum = valid.iter().fold(0.0f64, |max, stage| max.max(stage.value));
    let rows = valid
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let previous = index.checked_sub(1).map(|i| valid[i].value);
            FunnelRow {
                id: stage.id.clone(),
                label: stage.label.clone(),
                value: stage.value,
                color: stage.color.clone(),
                proportion: if maximum > 0.0 {
                    stage.value / maximum
                } else {
                    0.0
                },
                conversion: if first > 0.0 {
                    Some((stage.value / first) * 100.0)
                } else {
                    None
                },
                step_conversion: previous
                    .filter(|previous| *previous > 0.0)
                    .map(|previous| (stage.value / previous) * 100.0),
                change: previous.map(|previous| stage.value - previous),
            }
        })
        .collect();
    FunnelBuild {
        rows,
        first,
        last,
        conversion: if first > 0.0 {
            Some((last / first) * 100.0)
        } else {
            None
        },
    }
}

/// `FunnelChartProps["direction"]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FunnelDirection {
    #[default]
    Vertical,
    Horizontal,
}

/// `funnelPath`.
pub fn funnel_path(proportions: &[f64], index: usize, direction: FunnelDirection) -> String {
    let value = proportions[index];
    let top = if index == 0 {
        value
    } else {
        (proportions[index - 1] + value) / 2.0
    };
    let bottom = if index == proportions.len() - 1 {
        value
    } else {
        (value + proportions[index + 1]) / 2.0
    };
    let len = proportions.len() as f64;
    let start = index as f64 / len;
    let end = (index as f64 + 1.0) / len;
    let middle = (start + end) / 2.0;
    let quarter = (end - start) / 4.0;
    let point = |cross: f64, along: f64| match direction {
        FunnelDirection::Vertical => format!("{},{}", cross * 1000.0, along * 500.0),
        FunnelDirection::Horizontal => format!("{},{}", along * 1000.0, cross * 500.0),
    };
    let left = |width: f64| 0.5 - width * 0.46;
    let right = |width: f64| 0.5 + width * 0.46;
    format!(
        "M{} L{} C{} {} {} C{} {} {} L{} C{} {} {} C{} {} {} Z",
        point(left(top), start),
        point(right(top), start),
        point(right(top), start + quarter),
        point(right(value), middle - quarter),
        point(right(value), middle),
        point(right(value), middle + quarter),
        point(right(bottom), end - quarter),
        point(right(bottom), end),
        point(left(bottom), end),
        point(left(bottom), end - quarter),
        point(left(value), middle + quarter),
        point(left(value), middle),
        point(left(value), middle - quarter),
        point(left(top), start + quarter),
        point(left(top), start),
    )
}

fn percentage(value: Option<f64>) -> String {
    match value {
        None => "—".to_string(),
        Some(value) => format!("{value:.1}%"),
    }
}

#[derive(Clone, Copy)]
struct FunnelContext {
    rows: Signal<Vec<FunnelRow>>,
    first: Signal<f64>,
    last: Signal<f64>,
    conversion: Signal<Option<f64>>,
    direction: FunnelDirection,
    unit: Signal<String>,
    format_value: Option<Callback<f64, String>>,
    hovered: Signal<Option<usize>>,
}

impl FunnelContext {
    fn format(&self, value: f64) -> String {
        match self.format_value {
            Some(callback) => callback.call(value),
            None => crate::donut_chart::default_format(value),
        }
    }
}

/// `FunnelChart`.
#[component]
pub fn FunnelChart(
    stages: Vec<FunnelStage>,
    #[props(default)] direction: FunnelDirection,
    #[props(default = "people".to_string())] unit: String,
    #[props(default = "Conversion funnel".to_string())] label: String,
    #[props(default)] format_value: Option<Callback<f64, String>>,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let build = build_funnel(&stages);
    let rows_signal = use_signal(|| build.rows);
    let first_signal = use_signal(|| build.first);
    let last_signal = use_signal(|| build.last);
    let conversion_signal = use_signal(|| build.conversion);
    let unit_signal = use_signal(|| unit.clone());
    let hovered = use_signal(|| None::<usize>);

    let context = FunnelContext {
        rows: rows_signal,
        first: first_signal,
        last: last_signal,
        conversion: conversion_signal,
        direction,
        unit: unit_signal,
        format_value,
        hovered,
    };
    use_context_provider(|| context);

    rsx! {
        section {
            "aria-label": "{label}",
            class: cn!("w-full space-y-6", class),
            if let Some(children) = children {
                {children}
            } else {
                FunnelChartPlot {}
                FunnelChartSummary {}
            }
        }
    }
}

/// `FunnelChartPlot`.
#[component]
pub fn FunnelChartPlot(#[props(default)] class: String) -> Element {
    let context = use_context::<FunnelContext>();
    let rows = (context.rows)();
    if rows.is_empty() {
        return rsx! {
            p { class: cn!("py-16 text-center text-sm text-muted-foreground", class), "No funnel data" }
        };
    }
    let horizontal = context.direction == FunnelDirection::Horizontal;
    let proportions: Vec<f64> = rows.iter().map(|row| row.proportion).collect();
    let count = rows.len();
    let height = if horizontal {
        300.0
    } else {
        (320.0f64).max(count as f64 * 72.0)
    };
    let min_width = if horizontal {
        count as f64 * 110.0
    } else {
        240.0
    };
    let unit = (context.unit)();
    let grid_template = format!("repeat({count}, 1fr)");
    let grid_style = if horizontal {
        format!("grid-template-columns:{grid_template}")
    } else {
        format!("grid-template-rows:{grid_template}")
    };
    let hovered = (context.hovered)();

    rsx! {
        div { class: cn!("space-y-5", class),
            div { class: "overflow-x-auto",
                div { class: "relative", style: "height:{height}px;min-width:{min_width}px",
                    svg {
                        "aria-hidden": "true",
                        view_box: "0 0 1000 500",
                        preserve_aspect_ratio: "none",
                        class: "absolute inset-0 h-full w-full",
                        for (index, row) in rows.iter().enumerate() {
                            path {
                                key: "{row.id}",
                                fill: "{row.color.clone().unwrap_or_else(|| FUNNEL_COLORS[index % FUNNEL_COLORS.len()].to_string())}",
                                d: "{funnel_path(&proportions, index, context.direction)}",
                            }
                        }
                    }
                    ol {
                        "aria-label": "Funnel stages",
                        class: "absolute inset-0 grid",
                        style: "{grid_style}",
                        for (index, row) in rows.iter().enumerate() {
                            {
                                let aria_label = format!(
                                    "{}: {} {}, {} of starting total",
                                    row.label,
                                    context.format(row.value),
                                    unit,
                                    percentage(row.conversion),
                                );
                                let is_hovered = hovered == Some(index);
                                let change = row.change.unwrap_or(0.0);
                                let change_word = if change > 0.0 { "gained" } else { "dropped" };
                                rsx! {
                                    li { key: "{row.id}", class: "relative min-h-0 min-w-0",
                                        button {
                                            r#type: "button",
                                            class: "flex h-full w-full items-center justify-center outline-none transition-colors hover:bg-foreground/5 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
                                            "aria-label": "{aria_label}",
                                            onpointerenter: {
                                                let mut hovered = context.hovered;
                                                move |_| hovered.set(Some(index))
                                            },
                                            onpointerleave: {
                                                let mut hovered = context.hovered;
                                                move |_| hovered.set(None)
                                            },
                                            onfocus: {
                                                let mut hovered = context.hovered;
                                                move |_| hovered.set(Some(index))
                                            },
                                            onblur: {
                                                let mut hovered = context.hovered;
                                                move |_| hovered.set(None)
                                            },
                                            span { class: "rounded bg-background/85 px-2 py-1 font-mono text-xs font-medium tabular-nums",
                                                "{context.format(row.value)}"
                                            }
                                        }
                                        if is_hovered {
                                            div {
                                                role: "tooltip",
                                                class: "pointer-events-none absolute left-1/2 top-1/2 z-10 flex w-max max-w-56 -translate-x-1/2 -translate-y-1/2 flex-col gap-1.5 rounded-xl border border-border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md",
                                                span { class: "text-muted-foreground", "{row.label}" }
                                                span { class: "font-mono", "{context.format(row.value)} {unit}" }
                                                if index > 0 {
                                                    span { "{percentage(row.step_conversion)} from previous stage" }
                                                    span { class: "text-muted-foreground",
                                                        "{context.format(change.abs())} {change_word}"
                                                    }
                                                }
                                                span { class: "text-muted-foreground",
                                                    "{percentage(row.conversion)} of starting total"
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
            div { class: "flex flex-wrap justify-center gap-x-6 gap-y-3",
                for (index, row) in rows.iter().enumerate() {
                    {
                        let legend_color = row.color.clone().unwrap_or_else(|| FUNNEL_COLORS[index % FUNNEL_COLORS.len()].to_string());
                        rsx! {
                    span { key: "{row.id}", class: "inline-flex items-center gap-2 text-xs",
                        span {
                            "aria-hidden": "true",
                            class: "size-2 shrink-0 rounded-full",
                            style: "background-color:{legend_color};",
                        }
                        span { "{row.label}" }
                        span { class: "font-mono text-[10px] text-muted-foreground",
                            "{percentage(row.conversion)}"
                        }
                    }
                        }
                    }
                }
            }
        }
    }
}

/// `FunnelChartSummary`.
#[component]
pub fn FunnelChartSummary(#[props(default)] class: String) -> Element {
    let context = use_context::<FunnelContext>();
    let rows = (context.rows)();
    if rows.is_empty() {
        return rsx! {};
    }
    let unit = (context.unit)();
    let first = (context.first)();
    let last = (context.last)();
    let conversion = (context.conversion)();
    rsx! {
        div {
            class: cn!(
                "flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4 text-xs",
                class,
            ),
            span { class: "text-muted-foreground",
                "{context.format(first)} → {context.format(last)} {unit}"
            }
            span { class: "flex items-center gap-2",
                span { class: "text-muted-foreground", "Overall conversion" }
                span { class: "font-mono font-medium tabular-nums", "{percentage(conversion)}" }
            }
        }
    }
}

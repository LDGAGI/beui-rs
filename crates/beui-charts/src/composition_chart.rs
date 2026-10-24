//! `charts/composition-chart/` — normalized stacked shares over time.
//!
//! `buildComposition` (dedupe, pre-scale by the period max, shares and offsets)
//! and `compositionArea` (separate polygons for contiguous valid runs) are
//! ported as pure functions. Bar/area views, the active period, highlight
//! (hover → focus → pinned) and the tooltip live in a shared context.
//!
//! Differences from the source: the inspection cursor reads the hidden
//! range's `input` event for pointer and keyboard rather than pointer-x
//! hit-testing; bars render their resolved geometry (no per-rect spring);
//! `NumberTicker` is not ported, values print directly; the tooltip is anchored
//! over the active column rather than following the cursor.

use beui::prelude::*;

use crate::common::use_unique_id;
use crate::donut_chart::default_format;

/// `CompositionSeries`.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionSeries {
    pub id: String,
    pub name: String,
    pub color: String,
    /// Nonnegative values aligned with periods; `None` leaves a gap.
    pub values: Vec<Option<f64>>,
}

impl CompositionSeries {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        color: impl Into<String>,
        values: Vec<Option<f64>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color: color.into(),
            values,
        }
    }
}

/// `CompositionSegment`.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionSegment {
    pub id: String,
    pub name: String,
    pub color: String,
    pub value: Option<f64>,
    pub share: f64,
    pub offset: f64,
}

/// `CompositionColumn`.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionColumn {
    pub id: String,
    pub valid: bool,
    pub segments: Vec<CompositionSegment>,
}

/// `buildComposition` result.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionBuild {
    pub rows: Vec<CompositionSeries>,
    pub columns: Vec<CompositionColumn>,
}

/// `buildComposition`.
pub fn build_composition(series: &[CompositionSeries], periods: &[String]) -> CompositionBuild {
    let mut seen = Vec::new();
    let mut rows = Vec::new();
    for row in series {
        if seen.contains(&row.id) {
            continue;
        }
        seen.push(row.id.clone());
        rows.push(row.clone());
    }
    let mut period_ids: Vec<String> = Vec::new();
    let mut columns = Vec::new();
    for (index, id) in periods.iter().enumerate() {
        if period_ids.contains(id) {
            continue;
        }
        period_ids.push(id.clone());
        let values: Vec<Option<f64>> = rows
            .iter()
            .map(|row| row.values.get(index).copied().flatten())
            .collect();
        let valid = !values.is_empty()
            && values
                .iter()
                .all(|value| value.is_some_and(|value| value.is_finite() && value >= 0.0));
        let max = if valid {
            values
                .iter()
                .filter_map(|value| *value)
                .fold(0.0f64, f64::max)
                .max(0.0)
        } else {
            0.0
        };
        let scaled_total: f64 = if max > 0.0 {
            values.iter().map(|value| value.unwrap_or(0.0) / max).sum()
        } else {
            0.0
        };
        let mut offset = 0.0;
        let mut segments = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let value = values[i];
            let share = if scaled_total > 0.0 {
                (value.unwrap_or(0.0) / max / scaled_total) * 100.0
            } else {
                0.0
            };
            segments.push(CompositionSegment {
                id: row.id.clone(),
                name: row.name.clone(),
                color: row.color.clone(),
                value,
                share,
                offset,
            });
            offset += share;
        }
        columns.push(CompositionColumn {
            id: id.clone(),
            valid: valid && scaled_total > 0.0,
            segments,
        });
    }
    CompositionBuild { rows, columns }
}

/// `compositionArea`.
pub fn composition_area(columns: &[CompositionColumn], row: usize) -> String {
    let mut paths: Vec<String> = Vec::new();
    let mut run: Vec<usize> = Vec::new();
    let flush = |run: &mut Vec<usize>, paths: &mut Vec<String>| {
        if run.is_empty() {
            return;
        }
        let len = columns.len();
        if run.len() == 1 {
            let index = run[0];
            let segment = &columns[index].segments[row];
            let left = (index as f64 / len as f64) * 100.0;
            let right = ((index + 1) as f64 / len as f64) * 100.0;
            paths.push(format!(
                "M{left},{} L{left},{} L{right},{} L{right},{} Z",
                100.0 - segment.offset,
                100.0 - segment.offset - segment.share,
                100.0 - segment.offset - segment.share,
                100.0 - segment.offset,
            ));
        } else {
            let top: Vec<String> = run
                .iter()
                .map(|index| {
                    let segment = &columns[*index].segments[row];
                    format!(
                        "{},{}",
                        ((*index as f64 + 0.5) / len as f64) * 100.0,
                        100.0 - segment.offset - segment.share
                    )
                })
                .collect();
            let bottom: Vec<String> = run
                .iter()
                .rev()
                .map(|index| {
                    let segment = &columns[*index].segments[row];
                    format!(
                        "{},{}",
                        ((*index as f64 + 0.5) / len as f64) * 100.0,
                        100.0 - segment.offset
                    )
                })
                .collect();
            paths.push(format!("M{} L{} Z", top.join(" L"), bottom.join(" L")));
        }
        run.clear();
    };
    for (index, column) in columns.iter().enumerate() {
        if column.valid {
            run.push(index);
        } else {
            flush(&mut run, &mut paths);
        }
    }
    flush(&mut run, &mut paths);
    paths.join(" ")
}

/// `CompositionChartProps["view"]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompositionView {
    #[default]
    Bar,
    Area,
}

#[derive(Clone, Copy)]
struct CompositionContext {
    build: Signal<CompositionBuild>,
    selected: Signal<Option<String>>,
    controlled: bool,
    view: CompositionView,
    format_value: Option<Callback<f64, String>>,
    hovered: Signal<Option<String>>,
    focused: Signal<Option<String>>,
    pinned: Signal<Option<String>>,
    on_period_change: Option<EventHandler<String>>,
}

impl CompositionContext {
    fn format(&self, value: f64) -> String {
        match self.format_value {
            Some(callback) => callback.call(value),
            None => default_format(value),
        }
    }

    fn index(&self) -> usize {
        let columns = (self.build)().columns;
        let selected = (self.selected)();
        columns
            .iter()
            .position(|column| Some(&column.id) == selected.as_ref())
            .unwrap_or_else(|| columns.len().saturating_sub(1))
    }

    fn column(&self) -> Option<CompositionColumn> {
        let columns = (self.build)().columns;
        let index = self.index();
        columns.get(index).cloned()
    }

    fn highlight(&self) -> Option<String> {
        let rows = (self.build)().rows;
        let valid = |id: &String| rows.iter().any(|row| &row.id == id);
        [self.hovered, self.focused, self.pinned]
            .into_iter()
            .filter_map(|signal| signal())
            .find(valid)
    }

    fn select(&self, id: String) {
        let columns = (self.build)().columns;
        let current = columns.get(self.index()).map(|column| column.id.clone());
        if !self.controlled {
            let mut selected = self.selected;
            selected.set(Some(id.clone()));
        }
        if Some(&id) != current.as_ref() {
            if let Some(handler) = self.on_period_change {
                handler.call(id);
            }
        }
    }
}

/// `CompositionChart`.
#[component]
pub fn CompositionChart(
    series: Vec<CompositionSeries>,
    periods: Vec<String>,
    #[props(default)] view: CompositionView,
    #[props(default)] period: Option<String>,
    #[props(default)] default_period: Option<String>,
    #[props(default)] on_period_change: Option<EventHandler<String>>,
    #[props(default)] format_value: Option<Callback<f64, String>>,
    #[props(default = "Composition over time".to_string())] label: String,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let build = build_composition(&series, &periods);
    let build_signal = use_signal(|| build);
    let mut internal_selected = use_signal(|| default_period.clone());
    let hovered = use_signal(|| None::<String>);
    let focused = use_signal(|| None::<String>);
    let pinned = use_signal(|| None::<String>);
    let controlled = period.is_some();

    use_effect(use_reactive((&period,), move |(period,)| {
        if let Some(period) = period {
            internal_selected.set(Some(period));
        }
    }));

    let context = CompositionContext {
        build: build_signal,
        selected: internal_selected,
        controlled,
        view,
        format_value,
        hovered,
        focused,
        pinned,
        on_period_change,
    };
    use_context_provider(|| context);

    rsx! {
        section {
            "aria-label": "{label}",
            class: cn!("@container w-full space-y-4", class),
            if let Some(children) = children {
                {children}
            } else {
                div { class: "grid gap-4",
                    CompositionChartPlot {}
                    CompositionChartLegend {}
                }
            }
        }
    }
}

/// `CompositionChartPlot`.
#[component]
pub fn CompositionChartPlot(#[props(default)] class: String) -> Element {
    let context = use_context::<CompositionContext>();
    let tooltip_id = use_unique_id("composition-tip");
    let tooltip_open = use_signal(|| false);
    let build = (context.build)();
    let columns = build.columns.clone();
    if columns.is_empty() || build.rows.is_empty() {
        return rsx! {
            p { class: cn!("flex min-h-64 items-center justify-center text-sm text-muted-foreground", class),
                "No composition data"
            }
        };
    }
    let index = context.index();
    let column = context.column();
    let highlight = context.highlight();
    let column_count = columns.len();

    // Grid lines at 0/25/50/75/100.
    let grid: Vec<u32> = vec![0, 25, 50, 75, 100];
    let cursor_x = ((index as f64 + 0.5) / column_count as f64) * 100.0;
    let header_left = match &column {
        Some(column) => {
            if column.valid {
                column.id.clone()
            } else {
                format!("{} · No data", column.id)
            }
        }
        None => String::new(),
    };
    let last_id = columns
        .last()
        .map(|column| column.id.clone())
        .unwrap_or_default();

    rsx! {
        div { class: cn!("min-w-0 space-y-3", class),
            div { class: "flex justify-between text-[11px] text-muted-foreground",
                span { "{header_left}" }
                span { class: "font-mono", "100%" }
            }
            div {
                class: "relative h-64 has-focus-visible:outline-2 has-focus-visible:outline-offset-4 has-focus-visible:outline-ring sm:h-80",
                onpointerleave: {
                    let mut open = tooltip_open;
                    move |_| open.set(false)
                },
                svg {
                    "aria-hidden": "true",
                    view_box: "0 0 100 100",
                    preserve_aspect_ratio: "none",
                    class: "size-full overflow-visible",
                    for y in grid.iter() {
                        line {
                            key: "{y}",
                            x1: "0",
                            x2: "100",
                            y1: "{y}",
                            y2: "{y}",
                            stroke: "currentColor",
                            "stroke-width": "1",
                            "vector-effect": "non-scaling-stroke",
                            class: "text-border",
                        }
                    }
                    if context.view == CompositionView::Area {
                        for (r, row) in build.rows.iter().enumerate() {
                            {
                                let opacity = if highlight.as_deref().is_some_and(|id| id != row.id) { 0.18 } else { 0.9 };
                                let path = composition_area(&columns, r);
                                rsx! {
                                    path {
                                        key: "{row.id}",
                                        d: "{path}",
                                        fill: "{row.color}",
                                        style: "opacity:{opacity};transition:opacity 180ms;",
                                    }
                                }
                            }
                        }
                    } else {
                        for (i, col) in columns.iter().enumerate() {
                            if col.valid {
                                for segment in col.segments.iter() {
                                    {
                                        let opacity = if highlight.as_deref().is_some_and(|id| id != segment.id) { 0.18 } else { 0.9 };
                                        let x = (i as f64 / column_count as f64) * 100.0 + 10.0 / column_count as f64;
                                        let width = 80.0 / column_count as f64;
                                        let y = 100.0 - segment.offset - segment.share;
                                        rsx! {
                                            rect {
                                                key: "{col.id}/{segment.id}",
                                                x: "{x}",
                                                y: "{y}",
                                                width: "{width}",
                                                height: "{segment.share}",
                                                fill: "{segment.color}",
                                                style: "opacity:{opacity};transition:opacity 180ms;",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    line {
                        x1: "0",
                        x2: "0",
                        y1: "0",
                        y2: "100",
                        stroke: "currentColor",
                        "stroke-width": "1",
                        "vector-effect": "non-scaling-stroke",
                        stroke_dasharray: "3 3",
                        class: "text-foreground/70",
                        style: "transform:translateX({cursor_x}px);",
                    }
                }
                input {
                    r#type: "range",
                    "aria-label": "Inspect period",
                    min: "0",
                    max: "{column_count - 1}",
                    step: "1",
                    value: "{index}",
                    "aria-valuetext": "{header_left}",
                    "aria-describedby": if tooltip_open() { Some(tooltip_id.clone()) } else { None },
                    class: "absolute inset-0 h-full w-full cursor-crosshair opacity-0",
                    onfocus: {
                        let mut open = tooltip_open;
                        move |_| open.set(true)
                    },
                    onblur: {
                        let mut open = tooltip_open;
                        move |_| open.set(false)
                    },
                    oninput: {
                        let columns = columns.clone();
                        move |event| {
                            if let Ok(index) = event.value().parse::<usize>() {
                                if let Some(column) = columns.get(index) {
                                    context.select(column.id.clone());
                                    let mut open = tooltip_open;
                                    open.set(true);
                                }
                            }
                        }
                    },
                    onkeydown: {
                        let mut open = tooltip_open;
                        move |event: KeyboardEvent| {
                            if event.key() == Key::Escape {
                                open.set(false);
                            }
                        }
                    },
                }
                if tooltip_open() {
                    CompositionTooltip { id: tooltip_id.clone(), cursor_x }
                }
            }
            div { class: "flex justify-between gap-4 text-[11px] text-muted-foreground",
                span { "{columns[0].id}" }
                span { "{last_id}" }
            }
        }
    }
}

/// `CompositionTooltipContent`, anchored over the active column.
#[component]
fn CompositionTooltip(id: String, cursor_x: f64) -> Element {
    let context = use_context::<CompositionContext>();
    let column = context.column();
    let format = |value: f64| context.format(value);
    let Some(column) = column else {
        return rsx! {};
    };
    rsx! {
        div {
            id: "{id}",
            role: "tooltip",
            class: "pointer-events-none absolute top-1 z-10 block w-72 max-w-full -translate-x-1/2 -translate-y-full rounded-xl border border-border bg-popover px-3 py-2 text-popover-foreground shadow-md",
            style: "left:{cursor_x}%",
            span { class: "mb-2 block text-xs font-medium", "{column.id}" }
            if column.valid {
                span { class: "grid gap-2",
                    for row in column.segments.iter() {
                        span { key: "{row.id}", class: "flex items-center gap-2 text-[11px]",
                            span {
                                "aria-hidden": "true",
                                class: "size-1.5 shrink-0 rounded-full",
                                style: "background-color:{row.color};",
                            }
                            span { class: "min-w-0 flex-1 truncate", "{row.name}" }
                            span { class: "max-w-28 shrink-0 truncate text-muted-foreground tabular-nums",
                                "{format(row.value.unwrap_or(0.0))}"
                            }
                            {
                                let share_text = format!("{:.1}%", row.share);
                                rsx! {
                                    span { class: "w-12 shrink-0 text-right font-mono tabular-nums", "{share_text}" }
                                }
                            }
                        }
                    }
                }
            } else {
                span { class: "block text-xs text-muted-foreground", "No complete data for this period." }
            }
        }
    }
}

/// `CompositionChartLegend`.
#[component]
pub fn CompositionChartLegend(#[props(default)] class: String) -> Element {
    let context = use_context::<CompositionContext>();
    let can_hover = beui_core::use_hover_capable();
    let column = context.column();
    let Some(column) = column else {
        return rsx! {};
    };
    let highlight = context.highlight();
    let pinned = (context.pinned)();
    let hovered_signal = context.hovered;
    let focused_signal = context.focused;
    let pinned_signal = context.pinned;

    rsx! {
        div {
            class: cn!(
                "grid min-w-0 grid-cols-2 gap-x-4 gap-y-1 @min-[480px]:grid-cols-3 @min-[900px]:grid-cols-6",
                class,
            ),
            for row in column.segments.iter() {
                {
                    let is_pinned = pinned.as_deref() == Some(row.id.as_str());
                    let dimmed = highlight.as_deref().is_some_and(|id| id != row.id);
                    let share_text = if column.valid { format!("{:.1}%", row.share) } else { "—".to_string() };
                    let value_title = match row.value {
                        Some(value) => context.format(value),
                        None => "Missing value".to_string(),
                    };
                    let id = row.id.clone();
                    rsx! {
                        button {
                            key: "{row.id}",
                            r#type: "button",
                            "aria-label": "Highlight {row.name}",
                            "aria-pressed": if is_pinned { "true" } else { "false" },
                            onclick: {
                                let id = id.clone();
                                move |_| {
                                    let mut signal = pinned_signal;
                                    signal.set(if is_pinned { None } else { Some(id.clone()) });
                                }
                            },
                            onpointerenter: {
                                let id = id.clone();
                                move |_| {
                                    if can_hover() {
                                        let mut signal = hovered_signal;
                                        signal.set(Some(id.clone()));
                                    }
                                }
                            },
                            onpointerleave: {
                                move |_| {
                                    let mut signal = hovered_signal;
                                    signal.set(None);
                                }
                            },
                            onfocus: {
                                let id = id.clone();
                                move |_| {
                                    let mut signal = focused_signal;
                                    signal.set(Some(id.clone()));
                                }
                            },
                            onblur: {
                                move |_| {
                                    let mut signal = focused_signal;
                                    signal.set(None);
                                }
                            },
                            onkeydown: {
                                move |event: KeyboardEvent| {
                                    if event.key() == Key::Escape {
                                        for mut signal in [pinned_signal, hovered_signal, focused_signal] {
                                            signal.set(None);
                                        }
                                    }
                                }
                            },
                            class: cn!(
                                "flex min-h-12 min-w-0 items-center gap-2.5 rounded-md px-2 py-1.5 text-left text-xs transition-opacity duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                                if is_pinned { "bg-muted/60" } else { "" },
                                if dimmed { "opacity-40" } else { "" },
                            ),
                            span {
                                "aria-hidden": "true",
                                class: "h-6 w-1 shrink-0 rounded-full",
                                style: "background-color:{row.color};",
                            }
                            span { class: "grid min-w-0 gap-0.5",
                                span { class: "truncate text-muted-foreground", "{row.name}" }
                                span { title: "{value_title}", class: "shrink-0 font-mono tabular-nums", "{share_text}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

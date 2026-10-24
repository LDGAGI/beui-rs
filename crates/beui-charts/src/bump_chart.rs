//! `charts/bump-chart/` — rankings over time.
//!
//! `buildBumpChart` (dedupe, rank filtering, numeric rank distances with a
//! capped plot height) and `bumpPath` (a broken path for missing periods, cubic
//! mid-point segments) are ported as pure functions. Hover/selection live in a
//! shared context; the plot renders paths, rank gridlines, period labels, an
//! inspecting point per (series, period) and the exact-ranks table.
//!
//! Differences from the source: the animated `useBumpGeometry` motion
//! values are not ported — points and paths render at their target coordinates,
//! so there is no spring between rank changes; the tooltip is rendered inside
//! the point rather than through motion's `Tooltip`.

use beui::prelude::*;

/// `BUMP_COLORS`.
pub const BUMP_COLORS: [&str; 6] = [
    "#8b5cf6", "#0d9488", "#f59e0b", "#3b82f6", "#f43f5e", "#a3a33a",
];

/// `PLOT`.
pub const PLOT_WIDTH: f64 = 560.0;
pub const PLOT_LEFT: f64 = 32.0;
pub const PLOT_RIGHT: f64 = 100.0;
pub const PLOT_TOP: f64 = 28.0;
pub const PLOT_BOTTOM: f64 = 36.0;
pub const PLOT_ROW_HEIGHT: f64 = 44.0;

/// `BumpSeries`.
#[derive(Clone, Debug, PartialEq)]
pub struct BumpSeries {
    pub id: String,
    pub name: String,
    /// One rank per period, starting at 1. `None` leaves a gap in the path.
    pub ranks: Vec<Option<i64>>,
    pub color: Option<String>,
}

impl BumpSeries {
    pub fn new(id: impl Into<String>, name: impl Into<String>, ranks: Vec<Option<i64>>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            ranks,
            color: None,
        }
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// A resolved series row.
#[derive(Clone, Debug, PartialEq)]
pub struct BumpRow {
    pub id: String,
    pub name: String,
    pub color: String,
    pub ranks: Vec<Option<i64>>,
}

/// `buildBumpChart` result.
#[derive(Clone, Debug, PartialEq)]
pub struct BumpModel {
    pub rows: Vec<BumpRow>,
    pub ranks: Vec<i64>,
    pub max_rank: i64,
    pub height: f64,
    pub plot_height: f64,
    pub period_count: usize,
}

impl BumpModel {
    pub fn x(&self, index: usize) -> f64 {
        if self.period_count <= 1 {
            (PLOT_LEFT + PLOT_WIDTH - PLOT_RIGHT) / 2.0
        } else {
            PLOT_LEFT
                + (index as f64 / (self.period_count - 1) as f64)
                    * (PLOT_WIDTH - PLOT_LEFT - PLOT_RIGHT)
        }
    }

    pub fn y(&self, rank: i64) -> f64 {
        if self.max_rank == 1 {
            PLOT_TOP + self.plot_height / 2.0
        } else {
            PLOT_TOP + ((rank - 1) as f64 / (self.max_rank - 1) as f64) * self.plot_height
        }
    }
}

/// `buildBumpChart`.
pub fn build_bump_chart(series: &[BumpSeries], period_count: usize) -> BumpModel {
    let mut seen = Vec::new();
    let mut rows = Vec::new();
    for (index, row) in series.iter().enumerate() {
        if seen.contains(&row.id) {
            continue;
        }
        seen.push(row.id.clone());
        let color = row
            .color
            .clone()
            .unwrap_or_else(|| BUMP_COLORS[index % BUMP_COLORS.len()].to_string());
        let ranks: Vec<Option<i64>> = (0..period_count)
            .map(|i| row.ranks.get(i).copied().flatten().filter(|rank| *rank > 0))
            .collect();
        rows.push(BumpRow {
            id: row.id.clone(),
            name: row.name.clone(),
            color,
            ranks,
        });
    }
    let mut ranks: Vec<i64> = rows
        .iter()
        .flat_map(|row| row.ranks.iter().filter_map(|rank| *rank))
        .collect();
    ranks.sort_unstable();
    ranks.dedup();
    let max_rank = ranks.last().copied().unwrap_or(1);
    let plot_height = (max_rank - 1).clamp(1, 8) as f64 * PLOT_ROW_HEIGHT;
    let height = PLOT_TOP + plot_height + PLOT_BOTTOM;
    BumpModel {
        rows,
        ranks,
        max_rank,
        height,
        plot_height,
        period_count,
    }
}

/// `bumpPath` — missing periods break the path.
pub fn bump_path(
    ranks: &[Option<i64>],
    x: impl Fn(usize) -> f64,
    y: impl Fn(i64) -> f64,
) -> String {
    let mut path = String::new();
    let mut previous: Option<(f64, f64)> = None;
    for (index, rank) in ranks.iter().enumerate() {
        let Some(rank) = rank else {
            previous = None;
            continue;
        };
        let point = (x(index), y(*rank));
        if let Some((px, py)) = previous {
            let middle = (px + point.0) / 2.0;
            path.push_str(&format!(
                " C {middle} {py}, {middle} {}, {} {}",
                point.1, point.0, point.1
            ));
        } else {
            path.push_str(&format!(" M {} {}", point.0, point.1));
        }
        previous = Some(point);
    }
    path.trim().to_string()
}

#[derive(Clone, Copy)]
struct BumpContext {
    model: Signal<BumpModel>,
    periods: Signal<Vec<String>>,
    label: Signal<String>,
    active: Signal<Option<String>>,
    controlled: bool,
    on_active_change: Option<EventHandler<Option<String>>>,
    hovered: Signal<Option<String>>,
    focused: Signal<Option<String>>,
    tip: Signal<Option<(String, usize)>>,
}

impl BumpContext {
    fn valid(&self, id: Option<&String>) -> bool {
        match id {
            Some(id) => (self.model)().rows.iter().any(|row| &row.id == id),
            None => false,
        }
    }

    fn pinned(&self) -> Option<String> {
        (self.active)().filter(|id| self.valid(Some(id)))
    }

    fn highlighted(&self) -> Option<String> {
        let hovered = (self.hovered)();
        if self.valid(hovered.as_ref()) {
            return hovered;
        }
        let focused = (self.focused)();
        if self.valid(focused.as_ref()) {
            return focused;
        }
        self.pinned()
    }

    fn select(&self, id: Option<String>) {
        if !self.controlled {
            let mut active = self.active;
            active.set(id.clone());
        }
        if let Some(handler) = self.on_active_change {
            handler.call(id);
        }
    }
}

/// `BumpChart`.
#[component]
pub fn BumpChart(
    series: Vec<BumpSeries>,
    periods: Vec<String>,
    #[props(default)] active: Option<String>,
    #[props(default)] default_active: Option<String>,
    #[props(default)] on_active_change: Option<EventHandler<Option<String>>>,
    #[props(default = "Rankings over time".to_string())] label: String,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let model = build_bump_chart(&series, periods.len());
    let model_signal = use_signal(|| model);
    let periods_signal = use_signal(|| periods.clone());
    let label_signal = use_signal(|| label.clone());
    let mut internal_active = use_signal(|| default_active.clone());
    let hovered = use_signal(|| None::<String>);
    let focused = use_signal(|| None::<String>);
    let tip = use_signal(|| None::<(String, usize)>);
    let controlled = active.is_some();

    use_effect(use_reactive((&active,), move |(active,)| {
        if let Some(active) = active {
            internal_active.set(Some(active));
        }
    }));

    let context = BumpContext {
        model: model_signal,
        periods: periods_signal,
        label: label_signal,
        active: internal_active,
        controlled,
        on_active_change,
        hovered,
        focused,
        tip,
    };
    use_context_provider(|| context);

    rsx! {
        section {
            "aria-label": "{label}",
            class: cn!("w-full space-y-5", class),
            if let Some(children) = children {
                {children}
            } else {
                BumpChartPlot {}
                BumpChartLegend {}
            }
        }
    }
}

/// `BumpChartPlot`.
#[component]
pub fn BumpChartPlot(#[props(default)] class: String) -> Element {
    let context = use_context::<BumpContext>();
    let model = (context.model)();
    let periods = (context.periods)();
    let label = (context.label)();
    let highlighted = context.highlighted();

    if periods.is_empty() || model.ranks.is_empty() {
        return rsx! {
            div { class: cn!("py-16 text-center text-sm text-muted-foreground", class),
                "No rankings yet"
            }
        };
    }

    let width = PLOT_WIDTH.max(periods.len() as f64 * 64.0);
    let height = model.height;
    let grid_ranks = model.ranks.clone();
    let rows = model.rows.clone();
    let model_for_paths = model.clone();
    let can_hover = beui_core::use_hover_capable();

    rsx! {
        section {
            "aria-label": "{label} plot, scroll horizontally for more periods",
            tabindex: "0",
            class: cn!("overflow-x-auto rounded-lg focus-visible:outline-2 focus-visible:outline-ring", class),
            div { class: "relative", style: "min-width:{width}px",
                svg {
                    role: "img",
                    "aria-label": "{label}",
                    view_box: "0 0 {PLOT_WIDTH} {height}",
                    class: "block w-full overflow-visible",
                    title { "{label}" }
                    desc {
                        "Ranks run from best at the top to lowest at the bottom. Use the legend to highlight or pin a series. Exact values follow in a table."
                    }
                    for rank in grid_ranks.iter() {
                        line {
                            key: "{rank}",
                            x1: "{PLOT_LEFT}",
                            x2: "{PLOT_WIDTH - PLOT_RIGHT}",
                            y1: "{model_for_paths.y(*rank)}",
                            y2: "{model_for_paths.y(*rank)}",
                            class: "stroke-border",
                            "stroke-dasharray": "2 5",
                        }
                    }
                    for (index, period) in periods.iter().enumerate() {
                        text {
                            key: "{period}",
                            x: "{model_for_paths.x(index)}",
                            y: "{height - 8.0}",
                            "text-anchor": "middle",
                            class: "fill-muted-foreground font-mono text-[10px]",
                            "{period}"
                        }
                    }
                    for row in rows.iter() {
                        {
                            let dimmed = highlighted.as_deref().is_some_and(|id| id != row.id);
                            let path = bump_path(
                                &row.ranks,
                                |i| model_for_paths.x(i),
                                |rank| model_for_paths.y(rank),
                            );
                            let last_point = row
                                .ranks
                                .iter()
                                .enumerate()
                                .filter_map(|(index, rank)| rank.map(|rank| (index, rank)))
                                .next_back();
                            let group_opacity = if dimmed { 0.22 } else { 1.0 };
                            let highlight_opacity = if highlighted.as_deref() == Some(row.id.as_str()) { 0.16 } else { 0.0 };
                            let label_text = if row.name.chars().count() > 12 {
                                let truncated: String = row.name.chars().take(11).collect();
                                format!("{truncated}…")
                            } else {
                                row.name.clone()
                            };
                            let hovered_signal = context.hovered;
                            let id_hover = row.id.clone();
                            rsx! {
                                g {
                                    key: "{row.id}",
                                    style: "opacity:{group_opacity};transition:opacity 180ms;",
                                    path {
                                        d: "{path}",
                                        stroke: "{row.color}",
                                        "stroke-width": "2",
                                        fill: "none",
                                    }
                                    path {
                                        d: "{path}",
                                        stroke: "{row.color}",
                                        "stroke-width": "5",
                                        fill: "none",
                                        style: "opacity:{highlight_opacity};transition:opacity 180ms;",
                                    }
                                    path {
                                        d: "{path}",
                                        stroke: "transparent",
                                        "stroke-width": "16",
                                        fill: "none",
                                        onpointerenter: move |_| {
                                            if can_hover() {
                                                let mut signal = hovered_signal;
                                                signal.set(Some(id_hover.clone()));
                                            }
                                        },
                                        onpointerleave: move |_| {
                                            let mut signal = hovered_signal;
                                            signal.set(None);
                                        },
                                    }
                                    if let Some((index, rank)) = last_point {
                                        g { transform: "translate({model_for_paths.x(index)} {model_for_paths.y(rank)})",
                                            text {
                                                x: "18",
                                                "dominant-baseline": "central",
                                                fill: "{row.color}",
                                                class: "text-[10px] font-medium",
                                                "{label_text}"
                                                title { "{row.name}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "pointer-events-none absolute inset-0 overflow-clip",
                    for (series_index, row) in rows.iter().enumerate() {
                        for (index, rank) in row.ranks.iter().enumerate() {
                            if let Some(rank) = rank {
                                BumpChartPoint {
                                    key: "{row.id}-{periods[index]}",
                                    series_id: row.id.clone(),
                                    period: periods[index].clone(),
                                    series_index,
                                    period_index: index,
                                    rank: *rank,
                                }
                            }
                        }
                    }
                }
            }
            table { class: "sr-only",
                caption { "{label} — exact ranks" }
                thead {
                    tr {
                        th { scope: "col", "Series" }
                        for period in periods.iter() {
                            th { key: "{period}", scope: "col", "{period}" }
                        }
                    }
                }
                tbody {
                    for row in rows.iter() {
                        tr { key: "{row.id}",
                            th { scope: "row", "{row.name}" }
                            for (index, rank) in row.ranks.iter().enumerate() {
                                td { key: "{periods[index]}",
                                    {rank.map(|rank| rank.to_string()).unwrap_or_else(|| "No data".to_string())}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `BumpChartPoint`.
#[component]
fn BumpChartPoint(
    series_id: String,
    period: String,
    series_index: usize,
    period_index: usize,
    rank: i64,
) -> Element {
    let context = use_context::<BumpContext>();
    let can_hover = beui_core::use_hover_capable();
    let _ = series_index;
    let model = (context.model)();
    let periods = (context.periods)();
    let row = model.rows.iter().find(|row| row.id == series_id).cloned();
    let Some(row) = row else {
        return rsx! {};
    };
    let highlighted = context.highlighted();
    let pinned = context.pinned();
    let active = highlighted.as_deref() == Some(row.id.as_str());
    let dimmed = highlighted.is_some() && !active;
    let is_pinned = pinned.as_deref() == Some(row.id.as_str());
    let previous = period_index
        .checked_sub(1)
        .and_then(|i| row.ranks.get(i).copied().flatten());
    let change = previous.map(|previous| previous - rank);
    let change_label = match change {
        None => "No previous rank".to_string(),
        Some(0) => "No change".to_string(),
        Some(change) => format!(
            "{} {} {}",
            if change > 0 { "Up" } else { "Down" },
            change.abs(),
            if change.abs() == 1 { "place" } else { "places" }
        ),
    };
    let x = model.x(period_index);
    let y = model.y(rank);
    let transform = format!(
        "translate({}%, {}%)",
        (x / PLOT_WIDTH) * 100.0,
        (y / model.height) * 100.0
    );
    let tip = (context.tip)();
    let show_tip = tip.as_ref() == Some(&(row.id.clone(), period_index));
    let previous_period = period_index
        .checked_sub(1)
        .and_then(|i| periods.get(i).cloned());
    let dot_opacity = if active { 0.12 } else { 0.0 };
    let point_opacity = if dimmed { 0.22 } else { 1.0 };
    let period_for_tip = period.clone();
    let row_for_tip = row.clone();

    rsx! {
        div {
            class: "pointer-events-none absolute inset-0",
            style: "transform:{transform}",
            div {
                class: "pointer-events-auto absolute -translate-x-1/2 -translate-y-1/2",
                onpointerenter: {
                    let id = row.id.clone();
                    let mut tip = context.tip;
                    move |_| {
                        if can_hover() {
                            let mut hovered = context.hovered;
                            hovered.set(Some(id.clone()));
                            tip.set(Some((id.clone(), period_index)));
                        }
                    }
                },
                onpointerleave: {
                    let mut tip = context.tip;
                    move |_| {
                        let mut hovered = context.hovered;
                        hovered.set(None);
                        tip.set(None);
                    }
                },
                div { class: "relative",
                    button {
                        r#type: "button",
                        "aria-label": "{row.name}, {period}: rank {rank}",
                        "aria-pressed": if is_pinned { "true" } else { "false" },
                        onfocus: {
                            let id = row.id.clone();
                            let mut tip = context.tip;
                            move |_| {
                                let mut focused = context.focused;
                                focused.set(Some(id.clone()));
                                tip.set(Some((id.clone(), period_index)));
                            }
                        },
                        onblur: {
                            let mut tip = context.tip;
                            move |_| {
                                let mut focused = context.focused;
                                focused.set(None);
                                tip.set(None);
                            }
                        },
                        onclick: {
                            let id = row.id.clone();
                            move |_| {
                                context.select(if is_pinned { None } else { Some(id.clone()) });
                            }
                        },
                        onkeydown: {
                            let mut tip = context.tip;
                            move |event: KeyboardEvent| {
                                if event.key() == Key::Escape {
                                    context.select(None);
                                    let mut hovered = context.hovered;
                                    hovered.set(None);
                                    let mut focused = context.focused;
                                    focused.set(None);
                                    tip.set(None);
                                }
                            }
                        },
                        class: "relative flex size-7 items-center justify-center rounded-full focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                        style: "color:{row.color};opacity:{point_opacity};transition:opacity 180ms;",
                        span {
                            "aria-hidden": "true",
                            class: "absolute inset-0 rounded-full bg-current",
                            style: "opacity:{dot_opacity};transition:opacity 150ms;",
                        }
                        span {
                            class: "relative flex size-5 items-center justify-center rounded-full border-[1.5px] border-current bg-background font-mono text-[9px] font-medium",
                            "{rank}"
                        }
                    }
                    if show_tip {
                        div {
                            role: "tooltip",
                            class: "pointer-events-none absolute bottom-full left-1/2 z-20 mb-2 min-w-[156px] -translate-x-1/2 rounded-xl border border-border bg-popover px-3 py-2 text-left shadow-md",
                            span { class: "flex items-center justify-between gap-4 text-[10px] text-muted-foreground",
                                span { "{row_for_tip.name}" }
                                span { "{period_for_tip}" }
                            }
                            span { class: "mt-2 flex items-baseline justify-between gap-4",
                                span {
                                    class: "font-mono text-lg font-medium",
                                    style: "color:{row_for_tip.color};",
                                    "#{rank}"
                                }
                                span { class: "text-xs", "{change_label}" }
                            }
                            if let (Some(previous), Some(previous_period)) = (previous, previous_period) {
                                span { class: "mt-1 block text-[10px] text-muted-foreground",
                                    "Previously #{previous} in {previous_period}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `BumpChartLegend`.
#[component]
pub fn BumpChartLegend(#[props(default)] class: String) -> Element {
    let context = use_context::<BumpContext>();
    let can_hover = beui_core::use_hover_capable();
    let model = (context.model)();
    let highlighted = context.highlighted();
    let pinned = context.pinned();
    let hovered_signal = context.hovered;
    let focused_signal = context.focused;

    rsx! {
        div { class: cn!("grid grid-cols-2 gap-x-5 gap-y-1 sm:grid-cols-3", class),
            for row in model.rows.iter() {
                {
                    let first = row.ranks.first().copied().flatten();
                    let last = row.ranks.last().copied().flatten();
                    let gain = match (first, last) { (Some(first), Some(last)) => Some(first - last), _ => None };
                    let is_pinned = pinned.as_deref() == Some(row.id.as_str());
                    let dimmed = highlighted.as_deref().is_some_and(|id| id != row.id);
                    let last_text = match last { Some(last) => format!("#{last}"), None => "—".to_string() };
                    let gain_text = match gain {
                        None | Some(0) => "—".to_string(),
                        Some(gain) => format!("{}{}", if gain > 0 { "↑" } else { "↓" }, gain.abs()),
                    };
                    let gain_title = match gain {
                        None => "Change unavailable".to_string(),
                        Some(0) => "Unchanged".to_string(),
                        Some(gain) => format!("{} {} places", if gain > 0 { "Up" } else { "Down" }, gain.abs()),
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
                                move |_| context.select(if is_pinned { None } else { Some(id.clone()) })
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
                                        context.select(None);
                                        let mut hovered = hovered_signal;
                                        hovered.set(None);
                                        let mut focused = focused_signal;
                                        focused.set(None);
                                    }
                                }
                            },
                            class: cn!(
                                "flex min-w-0 items-center gap-2 rounded-md px-2 py-2.5 text-xs text-foreground transition-opacity duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                                if dimmed { "opacity-35" } else { "" },
                                if is_pinned { "bg-muted/60" } else { "" },
                            ),
                            span {
                                "aria-hidden": "true",
                                class: "size-2 shrink-0 rounded-full",
                                style: "background-color:{row.color};",
                            }
                            span { class: "truncate", "{row.name}" }
                            span { class: "ml-auto font-mono tabular-nums", "{last_text}" }
                            span {
                                class: "min-w-5 text-right font-mono text-[10px] tabular-nums text-muted-foreground",
                                title: "{gain_title}",
                                "{gain_text}"
                            }
                        }
                    }
                }
            }
        }
    }
}

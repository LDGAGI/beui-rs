//! `charts/heat-calendar/` — weeks of activity as a single-hue grid.
//!
//! Date arithmetic (UTC epoch days), the five intensity buckets, the month
//! labels, the hover ripple and the anchor→pointer span selection with a live
//! total are ported. State lives in a shared context.
//!
//! Differences from the source: the source uses JS `Date`/`Intl`; the port
//! works in UTC epoch days with hand-rolled civil-date math and deterministic
//! English labels. The entrance wave and ripple are CSS transitions rather than
//! motion springs, `NumberTicker` is not ported, and the tooltip is a lane-local
//! bubble anchored at the cell. `endDate` takes an epoch day.

use beui::prelude::*;

/// `HeatCalendarCell` — zero-based week/day (Monday first).
pub type HeatCalendarCell = (usize, usize);

/// `HeatCalendarSelection`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeatCalendarSelection {
    pub start: HeatCalendarCell,
    pub end: Option<HeatCalendarCell>,
}

/// `STEPS`.
pub const STEPS: [u32; 5] = [0, 24, 46, 70, 94];

/// `EMPTY`.
pub const EMPTY: &str = "color-mix(in srgb, var(--foreground) 6%, transparent)";

/// `CELL`.
pub const CELL: f64 = 16.0;
/// `GAP`.
pub const GAP: f64 = 4.0;
/// `PITCH`.
pub const PITCH: f64 = CELL + GAP;
/// `MONTH_ROW`.
pub const MONTH_ROW: f64 = 12.0;
/// `LIFT`.
pub const LIFT: [f64; 3] = [1.3, 1.08, 1.03];

/// `daysFromCivil` (Howard Hinnant) — days since 1970-01-01.
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `civilFromDays`.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn weekday_index(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

fn monday_of(day: i64) -> i64 {
    day - weekday_index(day)
}

fn month_abbr(month: u32) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month.saturating_sub(1)).min(11) as usize]
}

fn weekday_abbr(index: i64) -> &'static str {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][index.rem_euclid(7) as usize]
}

/// `fmtDay` — `"Mon, Mar 4"`.
pub fn fmt_day(day: i64) -> String {
    let (_, month, date) = civil_from_days(day);
    format!(
        "{}, {} {}",
        weekday_abbr(weekday_index(day)),
        month_abbr(month),
        date
    )
}

/// `fmtMonth` — `"Mar"`.
pub fn fmt_month(day: i64) -> String {
    let (_, month, _) = civil_from_days(day);
    month_abbr(month).to_string()
}

/// `fmtRange` — `"Mar 4"`.
pub fn fmt_range(day: i64) -> String {
    let (_, month, date) = civil_from_days(day);
    format!("{} {}", month_abbr(month), date)
}

/// A month label placed at the first column of a month.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonthLabel {
    pub week: usize,
    pub month: u32,
    pub label: Option<String>,
}

/// The tooltip's resolved data.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatTooltip {
    pub days: i64,
    pub total: f64,
    pub count: f64,
    pub start_day: i64,
    pub end_day: i64,
    pub day: i64,
}

#[derive(Clone, Copy)]
struct HeatContext {
    values: Signal<Vec<Vec<f64>>>,
    weeks: usize,
    max_count: f64,
    unit: Signal<String>,
    color: Signal<String>,
    end_day: Signal<Option<i64>>,
    start_day: Signal<Option<i64>>,
    hover: Signal<Option<HeatCalendarCell>>,
    selection: Signal<Option<HeatCalendarSelection>>,
    controlled: bool,
    on_selection_change: Option<EventHandler<Option<HeatCalendarSelection>>>,
    step: Signal<Option<usize>>,
    settled: Signal<bool>,
    mounted: Signal<bool>,
    tooltip_id: Signal<String>,
}

impl HeatContext {
    fn level(&self, w: usize, d: usize) -> f64 {
        (self.values)()
            .get(w)
            .and_then(|week| week.get(d))
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    }

    fn bucket(&self, value: f64) -> usize {
        ((value * 5.0).floor() as usize).min(4)
    }

    fn fill(&self, bucket: usize) -> String {
        if bucket == 0 {
            EMPTY.to_string()
        } else {
            format!(
                "color-mix(in srgb, {} {}%, transparent)",
                (self.color)(),
                STEPS[bucket.min(4)]
            )
        }
    }

    fn count(&self, value: f64) -> f64 {
        (value * self.max_count).round()
    }

    fn date_of(&self, w: usize, d: usize) -> Option<i64> {
        (self.start_day)().map(|start| start + (w * 7 + d) as i64)
    }

    fn future(&self, w: usize, d: usize) -> bool {
        match ((self.end_day)(), self.date_of(w, d)) {
            (Some(end), Some(date)) => date > end,
            _ => false,
        }
    }

    fn valid_cell(&self, cell: HeatCalendarCell) -> bool {
        let (w, d) = cell;
        w < self.weeks && d < 7 && !self.future(w, d)
    }

    fn selection(&self) -> Option<HeatCalendarSelection> {
        let requested = (self.selection)()?;
        if !self.valid_cell(requested.start) {
            return None;
        }
        if let Some(end) = requested.end {
            if !self.valid_cell(end) {
                return None;
            }
        }
        Some(requested)
    }

    fn pinned(&self) -> Option<HeatCalendarCell> {
        self.selection().map(|selection| selection.start)
    }

    fn span_end(&self) -> Option<HeatCalendarCell> {
        self.selection().and_then(|selection| selection.end)
    }

    fn idx(cell: HeatCalendarCell) -> i64 {
        (cell.0 * 7 + cell.1) as i64
    }

    fn span(&self) -> Option<(i64, i64)> {
        let pinned = self.pinned()?;
        let span_to = self.span_end().or((self.hover)()).or(Some(pinned))?;
        let a = Self::idx(pinned);
        let b = Self::idx(span_to);
        Some((a.min(b), a.max(b)))
    }

    fn span_total(&self) -> f64 {
        let Some((lo, hi)) = self.span() else {
            return 0.0;
        };
        let mut total = 0.0;
        for i in lo..=hi {
            let w = (i / 7) as usize;
            let d = (i % 7) as usize;
            if self.future(w, d) {
                break;
            }
            total += self.count(self.level(w, d));
        }
        total
    }

    fn clear(&self) {
        self.set_selection(None);
    }

    fn set_selection(&self, next: Option<HeatCalendarSelection>) {
        if !self.controlled {
            let mut selection = self.selection;
            selection.set(next.clone());
        }
        if let Some(handler) = self.on_selection_change {
            handler.call(next);
        }
    }

    fn select(&self, cell: HeatCalendarCell) {
        if self.span_end().is_some() {
            self.clear();
        } else if let Some(pinned) = self.pinned() {
            if Self::idx(pinned) == Self::idx(cell) {
                self.set_selection(None);
            } else {
                self.set_selection(Some(HeatCalendarSelection {
                    start: pinned,
                    end: Some(cell),
                }));
            }
        } else {
            self.set_selection(Some(HeatCalendarSelection {
                start: cell,
                end: None,
            }));
        }
    }

    /// The cell the grid reacts to.
    fn hot(&self) -> Option<HeatCalendarCell> {
        (self.hover)().or(self.span_end()).or(self.pinned())
    }

    /// The cell the tooltip hangs from.
    fn tip(&self) -> Option<HeatCalendarCell> {
        self.span_end().or((self.hover)()).or(self.pinned())
    }

    fn tooltip(&self) -> Option<HeatTooltip> {
        let tip = self.tip()?;
        let day = self.date_of(tip.0, tip.1)?;
        let span = self.span();
        let (start_day, end_day, days) = match ((self.start_day)(), span) {
            (Some(start), Some((lo, hi))) => (start + lo, start + hi, hi - lo + 1),
            _ => (day, day, 1),
        };
        Some(HeatTooltip {
            days,
            total: self.span_total(),
            count: self.count(self.level(tip.0, tip.1)),
            start_day,
            end_day,
            day,
        })
    }

    fn hot_month(&self) -> Option<u32> {
        let hot = self.hot()?;
        let day = self.date_of(hot.0, hot.1)?;
        Some(civil_from_days(day).1)
    }

    fn cols(&self) -> Vec<MonthLabel> {
        let start = (self.start_day)();
        let mut list: Vec<MonthLabel> = (0..self.weeks)
            .map(|w| {
                let day = start.map(|start| start + (w * 7) as i64);
                let month = day.map(|day| civil_from_days(day).1).unwrap_or(0);
                let fresh = match (start, day) {
                    (Some(start), Some(_)) => {
                        w == 0 || civil_from_days(start + ((w - 1) * 7) as i64).1 != month
                    }
                    _ => false,
                };
                MonthLabel {
                    week: w,
                    month,
                    label: if fresh && day.is_some() {
                        day.map(fmt_month)
                    } else {
                        None
                    },
                }
            })
            .collect();
        if list.get(1).and_then(|c| c.label.as_ref()).is_some()
            || list.get(2).and_then(|c| c.label.as_ref()).is_some()
        {
            if let Some(first) = list.first_mut() {
                first.label = None;
            }
        }
        list
    }

    fn tip_position(&self) -> (f64, f64) {
        match self.tip() {
            Some((w, d)) => (
                w as f64 * PITCH + CELL / 2.0,
                MONTH_ROW + GAP + d as f64 * PITCH,
            ),
            None => (0.0, 0.0),
        }
    }
}

/// `HeatCalendar`.
#[component]
pub fn HeatCalendar(
    #[props(default = "commits".to_string())] unit: String,
    #[props(default = 16)] weeks: usize,
    #[props(default = 14.0)] max_count: f64,
    #[props(default)] values: Vec<Vec<f64>>,
    /// Last UTC day of the grid, as an epoch day (`days_from_civil`).
    #[props(default)]
    end_date: Option<i64>,
    #[props(default = "var(--accent)".to_string())] color: String,
    #[props(default)] selection: Option<HeatCalendarSelection>,
    #[props(default)] default_selection: Option<HeatCalendarSelection>,
    #[props(default)] on_selection_change: Option<EventHandler<Option<HeatCalendarSelection>>>,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let values_signal = use_signal(|| values.clone());
    let unit_signal = use_signal(|| unit.clone());
    let color_signal = use_signal(|| color.clone());
    let mut internal_selection = use_signal(|| default_selection.clone());
    let hover = use_signal(|| None::<HeatCalendarCell>);
    let step = use_signal(|| None::<usize>);
    let settled = use_signal(|| false);
    let mounted = use_signal(|| false);
    let today = use_signal(|| None::<i64>);
    let tooltip_id_value = crate::common::use_unique_id("heat-tip");
    let tooltip_id = use_signal(|| tooltip_id_value.clone());
    let controlled = selection.is_some();

    use_effect(use_reactive((&selection,), move |(selection,)| {
        if let Some(selection) = selection {
            internal_selection.set(Some(selection));
        }
    }));

    // "today" is read after mount, and the entrance wave settles after it lands.
    use_effect(move || {
        let mut settled = settled;
        let mut mounted = mounted;
        let mut today = today;
        spawn(async move {
            if let Some(mut eval) =
                beui_core::platform::eval("dioxus.send(Math.floor(Date.now()/86400000));")
            {
                if let Ok(day) = eval.recv::<i64>().await {
                    today.set(Some(day));
                }
            }
            let wait = (weeks + 7) * 18 + 500;
            if let Some(mut eval) =
                beui_core::platform::eval(&format!("setTimeout(()=>dioxus.send(true),{wait});"))
            {
                if eval.recv::<bool>().await.is_ok() {
                    settled.set(true);
                    mounted.set(true);
                }
            }
        });
    });

    let end_day = use_signal(|| end_date);
    let start_day = use_signal(|| None::<i64>);
    // Keep `end_day`/`start_day` in sync with the props and the fetched today.
    use_effect(move || {
        let end = end_date.or(today());
        let start = end.map(|end| monday_of(end) - ((weeks as i64) - 1) * 7);
        let mut end_signal = end_day;
        if (end_signal)() != end {
            end_signal.set(end);
        }
        let mut start_signal = start_day;
        if (start_signal)() != start {
            start_signal.set(start);
        }
    });

    let context = HeatContext {
        values: values_signal,
        weeks,
        max_count,
        unit: unit_signal,
        color: color_signal,
        end_day,
        start_day,
        hover,
        selection: internal_selection,
        controlled,
        on_selection_change,
        step,
        settled,
        mounted,
        tooltip_id,
    };
    use_context_provider(|| context);

    rsx! {
        div { class: cn!("w-fit max-w-full", class),
            if let Some(children) = children {
                {children}
            } else {
                HeatCalendarGrid { HeatCalendarTooltip {} }
                HeatCalendarLegend {}
            }
        }
    }
}

/// `HeatCalendarGrid`.
#[component]
pub fn HeatCalendarGrid(
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let context = use_context::<HeatContext>();
    let reduce = use_reduced_motion()();
    let can_hover = beui_core::use_hover_capable();
    let weeks = context.weeks;
    let cols = context.cols();
    let hot = context.hot();
    let hot_month = context.hot_month();
    let span = context.span();
    let pinned = context.pinned();
    let step = (context.step)();
    let settled = (context.settled)();
    let mounted = (context.mounted)();
    let hovered_signal = context.hover;
    let unit = (context.unit)();

    let grid_style = format!(
        "width:{}px;max-width:100%;grid-template-columns:repeat({weeks},minmax(0,1fr));grid-template-rows:{MONTH_ROW}px repeat(7,auto);gap:{GAP}px;",
        weeks as f64 * PITCH - GAP,
    );

    rsx! {
        div {
            class: cn!("relative grid", class),
            style: "{grid_style}",
            onpointerleave: move |_| {
                let mut hover = hovered_signal;
                hover.set(None);
            },
            for col in cols.iter() {
                if let Some(label) = col.label.clone() {
                    span {
                        key: "m{col.week}",
                        class: cn!(
                            "whitespace-nowrap text-[10px] leading-none transition-colors duration-200",
                            if hot_month == Some(col.month) { "text-foreground" } else { "text-muted-foreground" },
                        ),
                        style: "grid-column:{col.week + 1};grid-row:1;",
                        "{label}"
                    }
                }
            }
            for col in cols.iter() {
                for d in 0..7usize {
                    if !context.future(col.week, d) {
                        {
                            let w = col.week;
                            let value = context.level(w, d);
                            let bucket = context.bucket(value);
                            let index = (w * 7 + d) as i64;
                            let on = hot == Some((w, d));
                            let is_end = match span {
                                Some((lo, hi)) => index == lo || index == hi,
                                None => pinned == Some((w, d)),
                            };
                            let dim = (step.is_some() && step != Some(bucket))
                                || span.map(|(lo, hi)| index < lo || index > hi).unwrap_or(false);
                            let dist = match hot {
                                Some((hw, hd)) => (hw as i64 - w as i64).abs().max((hd as i64 - d as i64).abs()),
                                None => 9,
                            };
                            let lift = if reduce || !can_hover() {
                                1.0
                            } else if dist == 0 {
                                LIFT[0]
                            } else if (dist as usize) < LIFT.len() {
                                LIFT[dist as usize]
                            } else {
                                1.0
                            };
                            let delay = if settled || reduce {
                                (dist.min(3)) as f64 * 0.03
                            } else {
                                (w + d) as f64 * 0.018
                            };
                            let cell_count = context.count(value);
                            let date = context.date_of(w, d);
                            let aria = match date {
                                Some(date) => format!("{} {} on {}", cell_count, unit, fmt_day(date)),
                                None => format!("{cell_count} {unit}"),
                            };
                            let inner_style = format!(
                                "background:{};box-shadow:{};opacity:{};transform:scale({lift});transition:transform 220ms cubic-bezier(0.34,1.3,0.64,1) {delay}s,opacity 220ms {delay}s;",
                                context.fill(bucket),
                                if is_end {
                                    "0 0 0 2px var(--background), 0 0 0 3.5px var(--foreground)".to_string()
                                } else {
                                    "none".to_string()
                                },
                                if mounted || reduce { 1.0 } else { 0.0 },
                            );
                            let ring = on && !is_end;
                            let tooltip_id = (context.tooltip_id)();
                            let wrapper_style = format!(
                                "grid-column:{};grid-row:{};opacity:{};z-index:{};",
                                w + 1,
                                d + 2,
                                if dim { 0.25 } else { 1.0 },
                                if lift > 1.0 { LIFT.len() as i64 - dist } else { 0 },
                            );
                            rsx! {
                                span {
                                    key: "{w}-{d}",
                                    class: "relative block aspect-square w-full transition-opacity duration-200",
                                    style: "{wrapper_style}",
                                    button {
                                        r#type: "button",
                                        "data-heat-cell": "{w}-{d}",
                                        "aria-label": "{aria}",
                                        "aria-pressed": if is_end { "true" } else { "false" },
                                        "aria-describedby": if on { Some(tooltip_id.clone()) } else { None },
                                        class: "absolute -inset-0.5 block rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-ring active:scale-90",
                                        onpointerenter: {
                                            let mut hover = hovered_signal;
                                            move |_| hover.set(Some((w, d)))
                                        },
                                        onfocus: {
                                            let mut hover = hovered_signal;
                                            move |_| hover.set(Some((w, d)))
                                        },
                                        onblur: {
                                            let mut hover = hovered_signal;
                                            move |_| hover.set(None)
                                        },
                                        onclick: move |_| context.select((w, d)),
                                        onkeydown: move |event: KeyboardEvent| {
                                            if event.key() == Key::Escape {
                                                context.clear();
                                            }
                                        },
                                        span {
                                            class: "pointer-events-none absolute inset-0.5 block rounded-[4px]",
                                            style: "{inner_style}",
                                            if ring {
                                                span {
                                                    class: "pointer-events-none absolute inset-0 block border-[1.5px]",
                                                    style: "border-radius:inherit;border-color:color-mix(in srgb, var(--foreground) 55%, transparent);",
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
            if let Some(children) = children {
                {children}
            }
        }
    }
}

/// `HeatCalendarTooltip`.
#[component]
pub fn HeatCalendarTooltip(
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let context = use_context::<HeatContext>();
    let tooltip = context.tooltip();
    let Some(tooltip) = tooltip else {
        return rsx! {};
    };
    let (x, y) = context.tip_position();
    let unit = (context.unit)();
    let value = if tooltip.days > 1 {
        tooltip.total
    } else {
        tooltip.count
    };
    let range = if tooltip.days > 1 {
        format!(
            "{} – {}",
            fmt_range(tooltip.start_day),
            fmt_range(tooltip.end_day)
        )
    } else {
        fmt_day(tooltip.day)
    };
    rsx! {
        div {
            id: "{context.tooltip_id}",
            role: "tooltip",
            class: cn!("pointer-events-none absolute z-10 flex flex-wrap items-center gap-1.5 rounded-lg border border-border bg-popover px-2 py-1 text-xs text-popover-foreground shadow-md", class),
            style: "left:{x}px;top:{y}px;transform:translate(-50%,-100%);",
            if let Some(children) = children {
                {children}
            } else {
                span { class: "inline-flex items-center gap-1 font-mono tabular-nums",
                    "{crate::donut_chart::default_format(value)} {unit}"
                }
                span { class: "text-muted-foreground", "{range}" }
                if tooltip.days > 1 {
                    span { class: "text-muted-foreground", "{tooltip.days} days" }
                }
            }
        }
    }
}

/// `HeatCalendarLegend`.
#[component]
pub fn HeatCalendarLegend(#[props(default)] class: String) -> Element {
    let context = use_context::<HeatContext>();
    let reduce = use_reduced_motion()();
    let can_hover = beui_core::use_hover_capable();
    let start = (context.start_day)();
    let end = (context.end_day)();
    let step = (context.step)();
    let step_signal = context.step;
    let range = match (start, end) {
        (Some(start), Some(end)) => format!("{} – {}", fmt_range(start), fmt_range(end)),
        _ => "\u{00a0}".to_string(),
    };
    rsx! {
        div { class: cn!("mt-3 flex flex-wrap items-center justify-between gap-3", class),
            span { class: "text-xs text-muted-foreground", "{range}" }
            span {
                class: "flex items-center gap-1",
                onpointerleave: {
                    let mut step = step_signal;
                    move |_| step.set(None)
                },
                span { class: "mr-0.5 text-xs text-muted-foreground", "less" }
                for (index, bucket) in STEPS.iter().enumerate() {
                    {
                        let swatch_bg = context.fill(index);
                        let swatch_scale = if !reduce && step == Some(index) { "scale(1.25)" } else { "none" };
                        rsx! {
                    button {
                        key: "{bucket}",
                        r#type: "button",
                        "aria-label": "Show activity level {index}",
                        "aria-pressed": if step == Some(index) { "true" } else { "false" },
                        class: "size-3 rounded-[3px] transition-transform duration-150",
                        style: "background:{swatch_bg};transform:{swatch_scale};",
                        onpointerenter: {
                            let mut step = step_signal;
                            move |_| {
                                if can_hover() {
                                    step.set(Some(index));
                                }
                            }
                        },
                        onfocus: {
                            let mut step = step_signal;
                            move |_| step.set(Some(index))
                        },
                        onblur: {
                            let mut step = step_signal;
                            move |_| step.set(None)
                        },
                        onclick: {
                            let mut step = step_signal;
                            move |_| step.set(if step() == Some(index) { None } else { Some(index) })
                        },
                    }
                        }
                    }
                }
                span { class: "ml-0.5 text-xs text-muted-foreground", "more" }
            }
        }
    }
}

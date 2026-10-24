//! `charts/returns-calendar/` — monthly returns as a years × months grid with a
//! compounded Year column.
//!
//! The tint (magnitude → strength of one up/down hue), the compounded totals,
//! the month-span and whole-year-span selection, the row/column dimming and the
//! tooltip read-outs are ported. State lives in a shared context.
//!
//! Differences from the source: the source animates the entrance wave and
//! the year replay through motion; the port keeps the entrance fade/cell lift as
//! CSS transitions and drops the replay pulse. `NumberTicker` is not ported, and
//! the tooltip is a lane-local bubble anchored to the tip cell.

use beui::prelude::*;

/// `UP`.
pub const UP: &str = "var(--success)";
/// `DOWN`.
pub const DOWN: &str = "var(--destructive)";
/// `MONTHS`.
pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
/// `YEAR`.
pub const YEAR: usize = 12;
/// `RING`.
pub const RING: &str = "inset 0 0 0 1.5px var(--foreground)";

/// `compound`.
pub fn compound(run: &[f64]) -> f64 {
    (run.iter()
        .fold(1.0, |acc, value| acc * (1.0 + value / 100.0))
        - 1.0)
        * 100.0
}

/// `signed`.
pub fn signed(value: f64, dp: usize) -> String {
    format!(
        "{}{:.*}",
        if value >= 0.0 { "+" } else { "−" },
        dp,
        value.abs()
    )
}

/// `ink`.
pub fn ink(color: &str) -> String {
    format!("oklch(from {color} min(l, var(--ink-l, 1)) c h)")
}

/// `tint`.
pub fn tint(value: f64, range: f64, on: bool) -> String {
    let strength =
        (((value.abs() / range).min(1.0) * 55.0) + if on { 24.0 } else { 8.0 }).round() as i64;
    format!(
        "color-mix(in srgb, {} {strength}%, transparent)",
        if value >= 0.0 { UP } else { DOWN }
    )
}

/// `ReturnsCalendarCell`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnsCalendarCell {
    pub y: usize,
    pub m: usize,
}

impl ReturnsCalendarCell {
    pub fn new(y: usize, m: usize) -> Self {
        Self { y, m }
    }
}

/// `ReturnsCalendarSelection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnsCalendarSelection {
    pub start: ReturnsCalendarCell,
    pub end: Option<ReturnsCalendarCell>,
}

fn same(cell: Option<ReturnsCalendarCell>, y: usize, m: usize) -> bool {
    cell == Some(ReturnsCalendarCell { y, m })
}

#[derive(Clone, Copy)]
struct ReturnsContext {
    years: Signal<Vec<i32>>,
    returns: Signal<Vec<Vec<f64>>>,
    hover: Signal<Option<ReturnsCalendarCell>>,
    selection: Signal<Option<ReturnsCalendarSelection>>,
    controlled: bool,
    on_selection_change: Option<EventHandler<Option<ReturnsCalendarSelection>>>,
    settled: Signal<bool>,
    mounted: Signal<bool>,
    tooltip_id: Signal<String>,
}

impl ReturnsContext {
    fn valid_cell(&self, cell: ReturnsCalendarCell) -> bool {
        cell.y < (self.years)().len() && cell.m <= YEAR
    }

    fn selection(&self) -> Option<ReturnsCalendarSelection> {
        let requested = (self.selection)()?;
        if !self.valid_cell(requested.start) {
            return None;
        }
        if let Some(end) = requested.end {
            if !self.valid_cell(end) || (requested.start.m == YEAR) != (end.m == YEAR) {
                return None;
            }
        }
        Some(requested)
    }

    fn pinned(&self) -> Option<ReturnsCalendarCell> {
        self.selection().map(|selection| selection.start)
    }

    fn span_end(&self) -> Option<ReturnsCalendarCell> {
        self.selection().and_then(|selection| selection.end)
    }

    fn rows(&self) -> Vec<Vec<f64>> {
        let returns = (self.returns)();
        (0..(self.years)().len())
            .map(|y| {
                (0..12)
                    .map(|m| {
                        returns
                            .get(y)
                            .and_then(|row| row.get(m))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect()
    }

    fn totals(&self) -> Vec<f64> {
        self.rows().iter().map(|row| compound(row)).collect()
    }

    fn flat(&self) -> Vec<f64> {
        self.rows().into_iter().flatten().collect()
    }

    fn value_at(&self, y: usize, m: usize) -> f64 {
        (self.returns)()
            .get(y)
            .and_then(|row| row.get(m))
            .copied()
            .unwrap_or(0.0)
    }

    fn clear(&self) {
        self.set_selection(None);
    }

    fn set_selection(&self, next: Option<ReturnsCalendarSelection>) {
        if !self.controlled {
            let mut selection = self.selection;
            selection.set(next);
        }
        if let Some(handler) = self.on_selection_change {
            handler.call(next);
        }
    }

    fn month_index(cell: ReturnsCalendarCell) -> i64 {
        (cell.y * 12 + cell.m) as i64
    }

    fn span(&self) -> Option<(i64, i64)> {
        let pinned = self.pinned()?;
        if pinned.m == YEAR {
            return None;
        }
        let span_to = self
            .span_end()
            .filter(|end| end.m != YEAR)
            .or_else(|| (self.hover)().filter(|hover| hover.m != YEAR));
        Some(match span_to {
            Some(end) => {
                let a = Self::month_index(pinned);
                let b = Self::month_index(end);
                (a.min(b), a.max(b))
            }
            None => (Self::month_index(pinned), Self::month_index(pinned)),
        })
    }

    fn year_span(&self) -> Option<(usize, usize)> {
        let pinned = self.pinned()?;
        if pinned.m != YEAR {
            return None;
        }
        let span_to = self
            .span_end()
            .filter(|end| end.m == YEAR)
            .or_else(|| (self.hover)().filter(|hover| hover.m == YEAR));
        let other = span_to.unwrap_or(pinned).y;
        Some((pinned.y.min(other), pinned.y.max(other)))
    }

    fn hot(&self) -> Option<ReturnsCalendarCell> {
        (self.hover)().or(self.span_end()).or(self.pinned())
    }

    fn tip(&self) -> Option<ReturnsCalendarCell> {
        self.span_end().or((self.hover)()).or(self.pinned())
    }

    fn span_value(&self) -> f64 {
        match self.span() {
            Some((lo, hi)) => compound(&self.flat()[lo as usize..=hi as usize]),
            None => 0.0,
        }
    }

    fn year_span_value(&self) -> f64 {
        match self.year_span() {
            Some((lo, hi)) => compound(&self.flat()[lo * 12..(hi + 1) * 12]),
            None => 0.0,
        }
    }

    fn tip_value(&self) -> f64 {
        let year_span = self.year_span();
        if year_span.is_some_and(|(lo, hi)| lo != hi) {
            return self.year_span_value();
        }
        if self.span().is_some_and(|(lo, hi)| lo != hi) {
            return self.span_value();
        }
        match self.tip() {
            Some(tip) if tip.m == YEAR => self.totals().get(tip.y).copied().unwrap_or(0.0),
            Some(tip) => self.value_at(tip.y, tip.m),
            None => 0.0,
        }
    }

    fn tip_label(&self) -> String {
        let years = (self.years)();
        if let Some((lo, hi)) = self.year_span().filter(|(lo, hi)| lo != hi) {
            return format!(
                "{} – {}",
                years.get(lo).copied().unwrap_or(0),
                years.get(hi).copied().unwrap_or(0)
            );
        }
        if let Some((lo, hi)) = self.span().filter(|(lo, hi)| lo != hi) {
            return format!(
                "{} {} – {} {}",
                MONTHS[(lo % 12) as usize],
                years.get((lo / 12) as usize).copied().unwrap_or(0),
                MONTHS[(hi % 12) as usize],
                years.get((hi / 12) as usize).copied().unwrap_or(0),
            );
        }
        match self.tip() {
            Some(tip) if tip.m == YEAR => format!("{}", years.get(tip.y).copied().unwrap_or(0)),
            Some(tip) => format!(
                "{} {}",
                MONTHS[tip.m.min(11)],
                years.get(tip.y).copied().unwrap_or(0)
            ),
            None => String::new(),
        }
    }

    fn tip_note(&self) -> Option<String> {
        if let Some((lo, hi)) = self.year_span().filter(|(lo, hi)| lo != hi) {
            return Some(format!("{} years", hi - lo + 1));
        }
        if let Some((lo, hi)) = self.span().filter(|(lo, hi)| lo != hi) {
            return Some(format!("{} months", hi - lo + 1));
        }
        self.tip()
            .filter(|tip| tip.m == YEAR)
            .map(|_| "for the year".to_string())
    }

    fn select(&self, cell: ReturnsCalendarCell) {
        let pinned = self.pinned();
        if self.span_end().is_some() || same(pinned, cell.y, cell.m) {
            self.clear();
        } else if let Some(pinned) = pinned {
            if (pinned.m == YEAR) == (cell.m == YEAR) {
                self.set_selection(Some(ReturnsCalendarSelection {
                    start: pinned,
                    end: Some(cell),
                }));
            } else {
                self.set_selection(Some(ReturnsCalendarSelection {
                    start: cell,
                    end: None,
                }));
            }
        } else {
            self.set_selection(Some(ReturnsCalendarSelection {
                start: cell,
                end: None,
            }));
        }
    }
}

/// `ReturnsCalendar`.
#[component]
pub fn ReturnsCalendar(
    #[props(default)] years: Vec<i32>,
    #[props(default)] returns: Vec<Vec<f64>>,
    #[props(default)] selection: Option<ReturnsCalendarSelection>,
    #[props(default)] default_selection: Option<ReturnsCalendarSelection>,
    #[props(default)] on_selection_change: Option<EventHandler<Option<ReturnsCalendarSelection>>>,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let years_signal = use_signal(|| years.clone());
    let returns_signal = use_signal(|| returns.clone());
    let mut internal_selection = use_signal(|| default_selection);
    let hover = use_signal(|| None::<ReturnsCalendarCell>);
    let settled = use_signal(|| false);
    let mounted = use_signal(|| false);
    let controlled = selection.is_some();
    use_effect(use_reactive((&selection,), move |(selection,)| {
        if let Some(selection) = selection {
            internal_selection.set(Some(selection));
        }
    }));
    let tooltip_id_value = crate::common::use_unique_id("returns-tip");
    let tooltip_id = use_signal(|| tooltip_id_value.clone());

    use_effect(move || {
        let mut settled = settled;
        let mut mounted = mounted;
        let count = years_signal().len();
        let wait = (count + 13) * 20 + 500;
        spawn(async move {
            let Some(mut eval) =
                beui_core::platform::eval(&format!("setTimeout(()=>dioxus.send(true),{wait});"))
            else {
                return;
            };
            if eval.recv::<bool>().await.is_ok() {
                settled.set(true);
                mounted.set(true);
            }
        });
    });

    let context = ReturnsContext {
        years: years_signal,
        returns: returns_signal,
        hover,
        selection: internal_selection,
        controlled,
        on_selection_change,
        settled,
        mounted,
        tooltip_id,
    };
    use_context_provider(|| context);

    rsx! {
        div { class: cn!("w-[480px] max-w-full [--ink-l:0.5] dark:[--ink-l:1]", class),
            if let Some(children) = children {
                {children}
            } else {
                ReturnsCalendarGrid { ReturnsCalendarTooltip {} }
            }
        }
    }
}

/// `ReturnsCalendarGrid`.
#[component]
pub fn ReturnsCalendarGrid(
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let context = use_context::<ReturnsContext>();
    let reduce = use_reduced_motion()();
    let can_hover = beui_core::use_hover_capable();
    let years = (context.years)();
    let totals = context.totals();
    let hot = context.hot();
    let span = context.span();
    let year_span = context.year_span();
    let pinned = context.pinned();
    let settled = (context.settled)();
    let mounted = (context.mounted)();
    let hovered = context.hover;
    let value_at = |y: usize, m: usize| context.value_at(y, m);

    rsx! {
        div {
            class: cn!("relative grid gap-1", class),
            style: "grid-template-columns:32px repeat(12,1fr) 52px;",
            onpointerleave: {
                let mut hover = hovered;
                move |_| hover.set(None)
            },
            span {}
            for m in 0..12usize {
                span {
                    key: "h{m}",
                    class: cn!(
                        "pb-1 text-center text-[10px] transition-colors duration-200",
                        if hot.is_some_and(|hot| hot.m == m) { "text-foreground" } else { "text-muted-foreground" },
                    ),
                    "{MONTHS[m].chars().next().unwrap_or(' ')}"
                }
            }
            span {
                class: cn!(
                    "pb-1 text-center text-[10px] transition-colors duration-200",
                    if hot.is_some_and(|hot| hot.m == YEAR) { "text-foreground" } else { "text-muted-foreground" },
                ),
                "Year"
            }
            for (y, year) in years.iter().enumerate() {
                div { key: "{year}", class: "contents",
                    span {
                        class: cn!(
                            "flex items-center justify-end pr-1.5 font-mono text-[10px] tabular-nums transition-colors duration-200",
                            if hot.is_some_and(|hot| hot.y == y) { "text-foreground" } else { "text-muted-foreground" },
                        ),
                        "’{short_year(*year)}"
                    }
                    for m in 0..12usize {
                        {
                            let value = value_at(y, m);
                            let index = (y * 12 + m) as i64;
                            let on = same(hot, y, m);
                            let is_end = match span {
                                Some((lo, hi)) => index == lo || index == hi,
                                None => same(pinned, y, m),
                            };
                            let dim = if let Some((lo, hi)) = span {
                                index < lo || index > hi
                            } else if let Some((lo, hi)) = year_span {
                                y < lo || y > hi
                            } else {
                                hot.is_some_and(|hot| !on && hot.y != y && hot.m != m)
                            };
                            let lift = if settled && on && can_hover() && !reduce { 1.15 } else { 1.0 };
                            let delay = if settled || reduce { 0.0 } else { (y + m) as f64 * 0.02 };
                            let cell_style = format!(
                                "background:{};color:color-mix(in srgb, var(--foreground) {}%, transparent);box-shadow:{};opacity:{};transform:scale({lift});transition:background 150ms,box-shadow 150ms,transform 200ms {delay}s,opacity 200ms {delay}s;",
                                tint(value, 8.0, on),
                                (85.0 + (value.abs() / 8.0).min(1.0) * 15.0).round() as i64,
                                if is_end { RING } else { "none" },
                                if mounted || reduce { 1.0 } else { 0.0 },
                            );
                            let cell_opacity = if dim { 0.35 } else { 1.0 };
                            let ring = on && !is_end;
                            let ring_color = if value >= 0.0 { UP } else { DOWN };
                            let tooltip_id = (context.tooltip_id)();
                            let is_tip = context.tip() == Some(ReturnsCalendarCell { y, m });
                            let tip_value = context.tip_value();
                            let tip_label = context.tip_label();
                            let tip_note = context.tip_note();
                            rsx! {
                                span {
                                    key: "{year}-{m}",
                                    class: "relative block aspect-square w-full transition-opacity duration-200",
                                    style: "opacity:{cell_opacity};",
                                    button {
                                        r#type: "button",
                                        "data-return-cell": "{y}-{m}",
                                        "aria-label": "{MONTHS[m]} {year} {signed(value, 1)}%",
                                        "aria-pressed": if is_end { "true" } else { "false" },
                                        "aria-describedby": if on { Some(tooltip_id.clone()) } else { None },
                                        class: "absolute -inset-0.5 block rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-ring active:scale-95",
                                        onpointerenter: {
                                            let mut hover = hovered;
                                            move |_| hover.set(Some(ReturnsCalendarCell { y, m }))
                                        },
                                        onfocus: {
                                            let mut hover = hovered;
                                            move |_| hover.set(Some(ReturnsCalendarCell { y, m }))
                                        },
                                        onblur: {
                                            let mut hover = hovered;
                                            move |_| hover.set(None)
                                        },
                                        onclick: move |_| context.select(ReturnsCalendarCell { y, m }),
                                        onkeydown: move |event: KeyboardEvent| {
                                            if event.key() == Key::Escape {
                                                context.clear();
                                            }
                                        },
                                        span {
                                            class: "pointer-events-none absolute inset-0.5 grid place-items-center rounded-[4px] font-mono text-[9px] font-medium tabular-nums",
                                            style: "{cell_style}",
                                            if ring {
                                                span {
                                                    class: "pointer-events-none absolute inset-0 border-[1.5px]",
                                                    style: "border-radius:inherit;border-color:{ring_color};",
                                                }
                                            }
                                            if value.abs() >= 4.0 { "{value.round() as i64}" } else { "" }
                                        }
                                    }
                                    if is_tip {
                                        ReturnsCalendarTooltipBubble { label: tip_label, value: tip_value, note: tip_note }
                                    }
                                }
                            }
                        }
                    }
                    {
                        let year_index = (y * 12) as i64;
                        let is_end = match year_span {
                            Some((lo, hi)) => y == lo || y == hi,
                            None => same(pinned, y, YEAR),
                        };
                        let dim = if let Some((lo, hi)) = span {
                            (y as i64) < lo / 12 || (y as i64) > hi / 12
                        } else if let Some((lo, hi)) = year_span {
                            y < lo || y > hi
                        } else {
                            hot.is_some_and(|hot| hot.y != y)
                        };
                        let total = totals.get(y).copied().unwrap_or(0.0);
                        let year_opacity = if dim { 0.35 } else { 1.0 };
                        let total_style = format!(
                            "background:{};color:{};box-shadow:{};opacity:{};transition:background 150ms,box-shadow 150ms,opacity 200ms;",
                            if same(hot, y, YEAR) { "var(--background)".to_string() } else { "color-mix(in srgb, var(--foreground) 6%, transparent)".to_string() },
                            ink(if total >= 0.0 { UP } else { DOWN }),
                            if is_end { RING } else if same(hot, y, YEAR) { "inset 0 0 0 1px var(--border-strong)" } else { "none" },
                            if mounted || reduce { 1.0 } else { 0.0 },
                        );
                        let tooltip_id = (context.tooltip_id)();
                        let is_tip = context.tip() == Some(ReturnsCalendarCell { y, m: YEAR });
                        let tip_value = context.tip_value();
                        let tip_label = context.tip_label();
                        let tip_note = context.tip_note();
                        let _ = year_index;
                        rsx! {
                            span {
                                key: "{year}-year",
                                class: "relative block transition-opacity duration-200",
                                style: "opacity:{year_opacity};",
                                button {
                                    r#type: "button",
                                    "data-return-cell": "{y}-{YEAR}",
                                    "aria-label": "{year} {signed(total, 1)}% for the year",
                                    "aria-pressed": if is_end { "true" } else { "false" },
                                    "aria-describedby": if same(hot, y, YEAR) { Some(tooltip_id.clone()) } else { None },
                                    class: "absolute -inset-0.5 block rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-ring active:scale-95",
                                    onpointerenter: {
                                        let mut hover = hovered;
                                        move |_| hover.set(Some(ReturnsCalendarCell { y, m: YEAR }))
                                    },
                                    onfocus: {
                                        let mut hover = hovered;
                                        move |_| hover.set(Some(ReturnsCalendarCell { y, m: YEAR }))
                                    },
                                    onblur: {
                                        let mut hover = hovered;
                                        move |_| hover.set(None)
                                    },
                                    onclick: move |_| context.select(ReturnsCalendarCell { y, m: YEAR }),
                                    onkeydown: move |event: KeyboardEvent| {
                                        if event.key() == Key::Escape {
                                            context.clear();
                                        }
                                    },
                                    span {
                                        class: "pointer-events-none absolute inset-0.5 grid place-items-center rounded-[4px] font-mono text-[11px] font-medium tabular-nums",
                                        style: "{total_style}",
                                        "{signed(total, 0)}%"
                                    }
                                }
                                if is_tip {
                                    ReturnsCalendarTooltipBubble { label: tip_label, value: tip_value, note: tip_note }
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

fn short_year(year: i32) -> String {
    let text = year.to_string();
    if text.len() >= 2 {
        text[text.len() - 2..].to_string()
    } else {
        text
    }
}

/// `ReturnsCalendarTooltip` — a no-op marker; the grid renders the bubble.
#[component]
pub fn ReturnsCalendarTooltip(#[props(default)] class: String) -> Element {
    let _ = class;
    rsx! {}
}

#[component]
fn ReturnsCalendarTooltipBubble(label: String, value: f64, note: Option<String>) -> Element {
    let value_color = ink(if value >= 0.0 { UP } else { DOWN });
    rsx! {
        div {
            role: "tooltip",
            class: "pointer-events-none absolute bottom-full left-1/2 z-20 mb-1 flex -translate-x-1/2 items-center gap-1.5 rounded-lg border border-border bg-popover px-2 py-1 font-mono text-xs text-popover-foreground shadow-md",
            span { class: "whitespace-nowrap text-muted-foreground", "{label}" }
            span {
                class: "inline-flex items-center tabular-nums",
                style: "color:{value_color};",
                "{signed(value, 1)}%"
            }
            if let Some(note) = note {
                span { class: "whitespace-nowrap text-muted-foreground", "{note}" }
            }
        }
    }
}

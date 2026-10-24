//! `motion/date-range-picker.tsx` — the range calendar.
//!
//! Calendar dates are plain `YYYY-MM-DD` strings (never a browser timezone),
//! and the civil-date maths is the same algorithm the browser's UTC `Date`
//! uses: days since 1970-01-01.
//!
//! The DOM, class strings and the day table match the source: a fixed 320px
//! panel, a `table-fixed` grid with the continuous range band, the month/year
//! choice views, arrow/Home/End/PageUp/PageDown navigation, and the trigger's
//! short-date summary. The panel is positioned by the component itself rather
//! than `MorphPopover`, and the source's per-action live-region announcements
//! are not ported (the footer summary is still `aria-live`).

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::hooks::{use_dismiss, use_reduced_motion, DismissOptions};
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::icons::{IconCalendarDays, IconChevronDown, IconChevronLeft, IconChevronRight};
use crate::motion::{spring, tween, Ease};

/// `DateRange`.
#[derive(Clone, Debug, PartialEq)]
pub struct DateRange {
    pub from: String,
    pub to: Option<String>,
}

/// Which surface the calendar body shows: days, months, or a 12-year page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CalView {
    Days,
    Months,
    Years,
}

// --- civil date maths -------------------------------------------------------

/// Days since 1970-01-01 for a civil date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn parse_date(date: &str) -> i64 {
    let parts: Vec<i64> = date.split('-').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 3 {
        return 0;
    }
    days_from_civil(parts[0], parts[1], parts[2])
}

pub fn iso_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn month_of(date: &str) -> String {
    format!("{}-01", &date[..7.min(date.len())])
}

fn shift_month(date: &str, offset: i64) -> String {
    let base = parse_date(&month_of(date));
    let (y, m, _) = civil_from_days(base);
    let total = y * 12 + (m - 1) + offset;
    let ny = total.div_euclid(12);
    let nm = total.rem_euclid(12) + 1;
    let (_, _, day) = civil_from_days(parse_date(date));
    let last = last_day_of(ny, nm);
    iso_date(days_from_civil(ny, nm, day.min(last)))
}

fn last_day_of(year: i64, month: i64) -> i64 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    days_from_civil(ny, nm, 1) - days_from_civil(year, month, 1)
}

/// Sunday = 0, matching the `en-US` week the source formats.
fn weekday_of(date: &str) -> i64 {
    (parse_date(date) + 4).rem_euclid(7)
}

fn ordered(a: &str, b: &str) -> DateRange {
    if a <= b {
        DateRange {
            from: a.to_string(),
            to: Some(b.to_string()),
        }
    } else {
        DateRange {
            from: b.to_string(),
            to: Some(a.to_string()),
        }
    }
}

/// The reference "today" for the today dot.
///
/// There is no cross-renderer clock here (the browser's `Date` lives behind
/// `platform`, and the desktop WebView behind `eval`), so the picker uses the
/// `max` bound the caller passes — which the preview sets to today — and falls
/// back to the first available date otherwise.
fn today(context: &RangeContext) -> String {
    context
        .max
        .clone()
        .or_else(|| context.min.clone())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

// --- date labels ------------------------------------------------------------

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTH_NAMES_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

fn parts(date: &str) -> (i64, i64, i64) {
    civil_from_days(parse_date(date))
}

/// `Oct 25` — the source's `month: "short", day: "numeric"` format.
fn short_no_year(date: &str) -> String {
    let (_, m, d) = parts(date);
    format!("{} {}", MONTH_NAMES_SHORT[(m - 1) as usize], d)
}

/// `Oct 25, 2026`.
fn short_with_year(date: &str) -> String {
    let (y, m, d) = parts(date);
    format!("{} {}, {}", MONTH_NAMES_SHORT[(m - 1) as usize], d, y)
}

/// `October 25, 2026` — used for the day button's accessible name.
fn long_date(date: &str) -> String {
    let (y, m, d) = parts(date);
    format!("{} {}, {}", MONTH_NAMES[(m - 1) as usize], d, y)
}

// --- context ----------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct RangeContext {
    value: Signal<Option<DateRange>>,
    month: Signal<String>,
    preview: Signal<Option<String>>,
    view: Signal<CalView>,
    year_page: Signal<i64>,
    cursor: Signal<Option<String>>,
    min: Option<String>,
    max: Option<String>,
    disabled: bool,
    open: Signal<bool>,
    close_on_select: bool,
    on_value_change: Option<EventHandler<Option<DateRange>>>,
}

impl RangeContext {
    fn unavailable(&self, date: &str) -> bool {
        self.disabled
            || self.min.as_ref().is_some_and(|min| date < min.as_str())
            || self.max.as_ref().is_some_and(|max| date > max.as_str())
    }

    fn shown(&self) -> Option<DateRange> {
        let value = (self.value)()?;
        if value.to.is_none() {
            if let Some(preview) = (self.preview)() {
                return Some(ordered(&value.from, &preview));
            }
        }
        Some(value)
    }

    fn select(&self, date: &str) {
        if self.unavailable(date) {
            return;
        }
        let next = match (self.value)() {
            Some(value) if value.to.is_none() => Some(ordered(&value.from, date)),
            _ => Some(DateRange {
                from: date.to_string(),
                to: None,
            }),
        };
        let mut value = self.value;
        value.set(next.clone());
        let mut preview = self.preview;
        preview.set(None);
        let mut cursor = self.cursor;
        cursor.set(Some(date.to_string()));
        if let Some(on_value_change) = self.on_value_change {
            on_value_change.call(next.clone());
        }
        if self.close_on_select && next.as_ref().is_some_and(|range| range.to.is_some()) {
            let mut open = self.open;
            open.set(false);
        }
    }

    /// The day that owns the roving tab stop in the current month.
    fn active_day(&self) -> Option<String> {
        let month = (self.month)();
        let (year, m, _) = parts(&month);
        let first = parse_date(&month);
        let count = last_day_of(year, m);
        let in_month = |date: &str| date.starts_with(&month[..7.min(month.len())]);
        (self.cursor)()
            .filter(|date| in_month(date) && !self.unavailable(date))
            .or_else(|| {
                (0..count)
                    .map(|offset| iso_date(first + offset))
                    .find(|date| !self.unavailable(date))
            })
    }
}

#[component]
pub fn DateRangePicker(
    #[props(default)] value: Option<DateRange>,
    #[props(default)] default_value: Option<DateRange>,
    #[props(default)] on_value_change: Option<EventHandler<Option<DateRange>>>,
    #[props(default)] default_month: Option<String>,
    #[props(default)] min: Option<String>,
    #[props(default)] max: Option<String>,
    #[props(default = "Reporting period".to_string())] label: String,
    #[props(default)] disabled: bool,
    #[props(default)] default_open: bool,
    #[props(default)] open: Option<bool>,
    #[props(default)] close_on_select: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let controlled = value.is_some();
    let initial_range = value.clone().or(default_value.clone());
    let initial_month = default_month
        .clone()
        .or_else(|| initial_range.as_ref().map(|r| r.from.clone()))
        .unwrap_or_else(|| "1970-01-01".to_string());
    let initial_cursor = initial_range
        .as_ref()
        .map(|range| range.from.clone())
        .unwrap_or_else(|| initial_month.clone());

    let internal = use_signal(|| initial_range.clone());
    let controlled_value = use_signal(|| value.clone());
    let month = use_signal(|| month_of(&initial_month));
    let preview = use_signal(|| None::<String>);
    let view = use_signal(|| CalView::Days);
    let year_page = use_signal(|| {
        let (year, _, _) = parts(&initial_month);
        year.div_euclid(12) * 12
    });
    let cursor = use_signal(|| Some(initial_cursor));
    let open_signal = use_signal(|| open.unwrap_or(default_open));

    use_effect(use_reactive!(|value| {
        if let Some(value) = value {
            let mut internal = internal;
            internal.set(Some(value));
        }
    }));
    use_effect(use_reactive!(|open| {
        if let Some(open) = open {
            let mut open_signal = open_signal;
            open_signal.set(open);
        }
    }));

    let context = RangeContext {
        value: if controlled {
            controlled_value
        } else {
            internal
        },
        month,
        preview,
        view,
        year_page,
        cursor,
        min: min.clone(),
        max: max.clone(),
        disabled,
        open: open_signal,
        close_on_select,
        on_value_change,
    };

    // Outside pointer / Escape. The scope root is the fieldset, so the trigger
    // and the panel are both "inside".
    let mut open_for_dismiss = open_signal;
    let dismiss = EventHandler::new(move |_| open_for_dismiss.set(false));
    let scope = use_dismiss(open_signal, dismiss, DismissOptions::default());

    use_context_provider(|| context.clone());

    rsx! {
        fieldset {
            disabled,
            "data-beui-scope": scope.attr(),
            class: cn!("relative m-0 flex w-fit min-w-0 flex-col gap-3 border-0 p-0", class),
            legend { class: "sr-only", "{label}" }
            {children}
        }
    }
}

#[component]
pub fn DateRangePickerTrigger(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let open = (context.open)();
    let summary = match (context.value)() {
        Some(range) => {
            let same_year = range
                .to
                .as_ref()
                .is_some_and(|to| range.from[..4.min(range.from.len())] == to[..4.min(to.len())]);
            let start = if same_year {
                short_no_year(&range.from)
            } else {
                short_with_year(&range.from)
            };
            let end = match range.to {
                Some(to) => short_with_year(&to),
                None => "Select end".to_string(),
            };
            format!("{start} – {end}")
        }
        None => "Select dates".to_string(),
    };

    rsx! {
        button {
            r#type: "button",
            "data-date-range-trigger": "true",
            "aria-haspopup": "dialog",
            "aria-expanded": if open { "true" } else { "false" },
            disabled: context.disabled,
            class: cn!(
                "inline-flex h-9 w-fit max-w-full items-center gap-2 rounded-xl border border-border bg-background px-3 text-[13px] font-medium text-foreground transition-colors hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40",
                class,
            ),
            onclick: move |_| {
                let mut open = context.open;
                open.toggle();
            },
            span { class: "shrink-0 text-muted-foreground", "aria-hidden": "true",
                IconCalendarDays { class: "size-[15px]" }
            }
            span { class: "min-w-0 truncate tabular-nums", "{summary}" }
            span {
                class: cn!(
                    "shrink-0 text-muted-foreground transition-transform",
                    if open { "rotate-180" } else { "" },
                ),
                "aria-hidden": "true",
                IconChevronDown { class: "size-[13px]" }
            }
        }
    }
}

#[component]
pub fn DateRangePickerContent(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_context::<RangeContext>();
    let panels: Vec<()> = if (context.open)() {
        vec![()]
    } else {
        Vec::new()
    };

    rsx! {
        AnimatePresence {
            for _ in panels {
                RangePanel { key: "{PANEL_KEY}", class: class.clone(), children: children.clone() }
            }
        }
    }
}

const PANEL_KEY: &str = "panel";

#[component]
fn RangePanel(#[props(default)] class: Option<String>, children: Element) -> Element {
    let reduce = use_reduced_motion()();
    let gate = use_gate();
    let config = if reduce {
        PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        )
    } else {
        PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0, scale: 0.96, y: -4.0 },
            motion_style! { opacity: 1.0, scale: 1.0, y: 0.0 },
            motion_style! { opacity: 0.0, scale: 0.96, y: -4.0 },
            spring(beui_core::ease::SPRING_PANEL),
            tween(140, Ease::Out),
        )
    };
    let style = use_presence_style(config).expect("static presence config");
    let css = presence_css(&style);

    rsx! {
        div {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            // `MorphPopoverContent` gives the source panel this width; without
            // it an absolutely positioned panel shrink-wraps to its contents,
            // and the calendar collapses as the selected date gets shorter.
            class: cn!(
                "absolute left-0 top-full z-50 mt-2 w-80 max-w-[calc(100vw-1.5rem)] origin-top rounded-2xl shadow-xl",
                class,
            ),
            {children}
        }
    }
}

const ICON_CLASS: &str = "inline-flex size-8 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-30";

#[component]
pub fn DateRangePickerCalendar(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: cn!(
                "flex w-80 max-w-full flex-col gap-3 rounded-2xl border border-border bg-background p-3.5",
                class,
            ),
            {children}
        }
    }
}

#[component]
pub fn DateRangePickerHeader(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let month = (context.month)();
    let (year, month_index, _) = parts(&month);
    let view = (context.view)();
    let min_year = context.min.as_ref().map(|min| parts(min).0).unwrap_or(0);
    let max_year = context.max.as_ref().map(|max| parts(max).0).unwrap_or(9999);
    let first_page = min_year;
    let last_page = (max_year - 11).max(first_page);
    let page = (context.year_page)().clamp(first_page, last_page);
    let page_end = (page + 11).min(max_year);

    let previous_disabled = match view {
        CalView::Days => context
            .min
            .as_ref()
            .is_some_and(|min| month <= month_of(min)),
        CalView::Months => year <= min_year,
        CalView::Years => page <= first_page,
    };
    let next_disabled = match view {
        CalView::Days => context
            .max
            .as_ref()
            .is_some_and(|max| month >= month_of(max)),
        CalView::Months => year >= max_year,
        CalView::Years => page >= last_page,
    };

    let open_months = {
        let context = context.clone();
        move |_| {
            let mut view = context.view;
            view.set(if (context.view)() == CalView::Months {
                CalView::Days
            } else {
                CalView::Months
            });
        }
    };
    let open_years = {
        let context = context.clone();
        move |_| {
            if (context.view)() == CalView::Years {
                let mut view = context.view;
                view.set(CalView::Days);
                return;
            }
            let (current_year, _, _) = parts(&(context.month)());
            let start = min_year + (current_year - min_year).div_euclid(12) * 12;
            let mut year_page = context.year_page;
            year_page.set(start.clamp(first_page, last_page));
            let mut view = context.view;
            view.set(CalView::Years);
        }
    };
    let page_back = {
        let context = context.clone();
        move |_| page_calendar(&context, -1)
    };
    let page_forward = {
        let context = context.clone();
        move |_| page_calendar(&context, 1)
    };

    rsx! {
        div { class: cn!("mb-2 flex items-center justify-between gap-1", class),
            div { class: "flex min-w-0 items-center gap-0.5",
                if view != CalView::Years {
                    button {
                        r#type: "button",
                        "aria-label": "Choose month",
                        "aria-expanded": if view == CalView::Months { "true" } else { "false" },
                        disabled: context.disabled,
                        class: "inline-flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-[13px] font-semibold transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40",
                        onclick: open_months,
                        "{MONTH_NAMES[(month_index - 1) as usize]}"
                        IconChevronDown {
                            class: cn!(
                                "size-3 text-muted-foreground transition-transform",
                                if view == CalView::Months { "rotate-180" } else { "" },
                            )
                        }
                    }
                }
                button {
                    r#type: "button",
                    "aria-label": "Choose year",
                    "aria-expanded": if view == CalView::Years { "true" } else { "false" },
                    disabled: context.disabled,
                    class: "inline-flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-[13px] font-medium tabular-nums text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40",
                    onclick: open_years,
                    if view == CalView::Years { "{page}–{page_end}" } else { "{year}" }
                    IconChevronDown {
                        class: cn!(
                            "size-3 text-muted-foreground transition-transform",
                            if view == CalView::Years { "rotate-180" } else { "" },
                        )
                    }
                }
            }
            div { class: "flex shrink-0 gap-0.5",
                button {
                    r#type: "button",
                    "aria-label": match view {
                        CalView::Days => "Previous month",
                        CalView::Months => "Previous year",
                        CalView::Years => "Previous years",
                    },
                    disabled: previous_disabled,
                    class: ICON_CLASS,
                    onclick: page_back,
                    IconChevronLeft { class: "size-4" }
                }
                button {
                    r#type: "button",
                    "aria-label": match view {
                        CalView::Days => "Next month",
                        CalView::Months => "Next year",
                        CalView::Years => "Next years",
                    },
                    disabled: next_disabled,
                    class: ICON_CLASS,
                    onclick: page_forward,
                    IconChevronRight { class: "size-4" }
                }
            }
        }
    }
}

/// Move the header's paging by `step` in whatever view is showing.
fn page_calendar(context: &RangeContext, step: i64) {
    match (context.view)() {
        CalView::Days => {
            let mut next = month_of(&shift_month(&(context.month)(), step));
            if let Some(min) = context.min.as_ref() {
                next = next.max(month_of(min));
            }
            if let Some(max) = context.max.as_ref() {
                next = next.min(month_of(max));
            }
            let mut month = context.month;
            month.set(next);
            let mut preview = context.preview;
            preview.set(None);
        }
        CalView::Months => {
            let mut next = month_of(&shift_month(&(context.month)(), step * 12));
            if let Some(min) = context.min.as_ref() {
                next = next.max(month_of(min));
            }
            if let Some(max) = context.max.as_ref() {
                next = next.min(month_of(max));
            }
            let mut month = context.month;
            month.set(next.clone());
            let mut cursor = context.cursor;
            cursor.set(Some(next));
        }
        CalView::Years => {
            let min_year = context.min.as_ref().map(|min| parts(min).0).unwrap_or(0);
            let max_year = context.max.as_ref().map(|max| parts(max).0).unwrap_or(9999);
            let first = min_year;
            let last = (max_year - 11).max(first);
            let mut page = context.year_page;
            page.set(((context.year_page)() + step * 12).clamp(first, last));
        }
    }
}

#[component]
pub fn DateRangePickerGrid(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let view = (context.view)();
    let month = (context.month)();
    let first = parse_date(&month);
    let (year, month_index, _) = parts(&month);
    let count = last_day_of(year, month_index);
    let lead = weekday_of(&month);
    let shown = context.shown();
    let value = (context.value)();
    let reference_today = today(&context);
    let active = context.active_day();

    let clear_preview = {
        let context = context.clone();
        move |_| {
            let mut preview = context.preview;
            preview.set(None);
        }
    };

    // The active day owns the roving tab stop. Keying the table by it remounts
    // the grid when the cursor moves, and the active button focuses itself on
    // mount — the same result as the source's `pendingFocus` layout effect,
    // without reaching into the DOM from Rust.
    let active_key = active.clone().unwrap_or_default();

    rsx! {
        div { class: cn!("overflow-hidden", class), onmouseleave: clear_preview,
            if view == CalView::Days {
                table {
                    key: "{active_key}",
                    role: "grid",
                    tabindex: if active.is_none() { "0" } else { "-1" },
                    class: "w-full table-fixed select-none border-separate border-spacing-x-0 border-spacing-y-1",
                    thead {
                        tr {
                            for day in WEEKDAYS.iter() {
                                th {
                                    key: "{day}",
                                    scope: "col",
                                    abbr: "{day}",
                                    class: "h-6 pb-1.5 text-center text-[11px] font-medium text-muted-foreground",
                                    "{day}"
                                }
                            }
                        }
                    }
                    tbody {
                        for row in 0..6 {
                            tr { key: "week-{row}",
                                for col in 0..7 {
                                    {
                                        let cell = row * 7 + col;
                                        let day_number = cell - lead;
                                        if day_number < 0 || day_number >= count {
                                            rsx! { td { key: "pad-{cell}", class: "relative h-8 p-0 text-center" } }
                                        } else {
                                            day_cell(
                                                &context,
                                                iso_date(first + day_number),
                                                col,
                                                &GridState {
                                                    shown: &shown,
                                                    value: &value,
                                                    today: &reference_today,
                                                    active: active.as_deref(),
                                                },
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                { choice_grid(&context, view, year, month_index, page_bounds(&context)) }
            }
        }
    }
}

fn page_bounds(context: &RangeContext) -> (i64, i64) {
    let min_year = context.min.as_ref().map(|min| parts(min).0).unwrap_or(0);
    let max_year = context.max.as_ref().map(|max| parts(max).0).unwrap_or(9999);
    let first = min_year;
    let last = (max_year - 11).max(first);
    let page = (context.year_page)().clamp(first, last);
    (page, (page + 11).min(max_year))
}

/// The values every cell in a month shares; bundled so `day_cell` stays within
/// clippy's argument limit.
struct GridState<'a> {
    shown: &'a Option<DateRange>,
    value: &'a Option<DateRange>,
    today: &'a str,
    active: Option<&'a str>,
}

fn day_cell(context: &RangeContext, date: String, day_index: i64, grid: &GridState<'_>) -> Element {
    let shown = grid.shown;
    let value = grid.value;
    let unavailable = context.unavailable(&date);
    let in_range = shown.as_ref().is_some_and(|range| {
        let to = range.to.clone().unwrap_or_else(|| range.from.clone());
        date.as_str() >= range.from.as_str() && date.as_str() <= to.as_str()
    });
    let endpoint = value
        .as_ref()
        .is_some_and(|range| date == range.from || range.to.as_deref() == Some(date.as_str()));
    let is_from = shown.as_ref().is_some_and(|range| range.from == date);
    let is_to = shown.as_ref().and_then(|range| range.to.clone()).as_deref() == Some(date.as_str());
    let no_to = shown.as_ref().is_none_or(|range| range.to.is_none());
    let is_today = date == grid.today;
    let rounded_left = in_range && (day_index == 0 || is_from);
    let rounded_right = in_range && (day_index == 6 || is_to || no_to);
    let fill_left = if endpoint && (is_from || day_index == 0) {
        "50%"
    } else {
        "0"
    };
    let fill_right = if endpoint && (is_to || no_to || day_index == 6) {
        "50%"
    } else {
        "0"
    };
    let aria_label = long_date(&date);
    let tabindex = if grid.active == Some(date.as_str()) {
        "0"
    } else {
        "-1"
    };

    let on_enter = {
        let context = context.clone();
        let date = date.clone();
        move |_| {
            if (context.value)().is_some_and(|range| range.to.is_none()) {
                let mut preview = context.preview;
                preview.set(Some(date.clone()));
            }
        }
    };
    let on_focus = {
        let context = context.clone();
        let date = date.clone();
        // Do not move the cursor here: focusing happens on mousedown, and a
        // cursor change remounts the keyed table, which would remove the button
        // before its click lands. Keyboard navigation sets the cursor itself.
        move |_| {
            if (context.value)().is_some_and(|range| range.to.is_none()) {
                let mut preview = context.preview;
                preview.set(Some(date.clone()));
            }
        }
    };
    let on_click = {
        let context = context.clone();
        let date = date.clone();
        move |_| context.select(&date)
    };
    let on_key = {
        let context = context.clone();
        let date = date.clone();
        move |event: KeyboardEvent| navigate_key(&context, &date, &event)
    };

    rsx! {
        td {
            key: "{date}",
            role: "gridcell",
            "aria-selected": if value.as_ref().is_some_and(|range| {
                let to = range.to.clone().unwrap_or_else(|| range.from.clone());
                date.as_str() >= range.from.as_str() && date.as_str() <= to.as_str()
            }) { "true" } else { "false" },
            class: cn!(
                "relative h-8 p-0 text-center",
                if rounded_left { "rounded-l-full" } else { "" },
                if rounded_right { "rounded-r-full" } else { "" },
            ),
            span {
                "aria-hidden": "true",
                class: cn!(
                    "pointer-events-none absolute inset-y-0 rounded-[inherit] bg-foreground/[0.08]",
                    if in_range { "opacity-100" } else { "opacity-0" },
                ),
                style: "left:{fill_left};right:{fill_right}",
            }
            button {
                r#type: "button",
                disabled: unavailable,
                "data-date": "{date}",
                "aria-label": "{aria_label}",
                "aria-current": if is_today { "date" } else { "" },
                tabindex: "{tabindex}",
                class: cn!(
                    "relative mx-auto flex h-8 w-full max-w-8 items-center justify-center rounded-full text-xs tabular-nums focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-30",
                    if endpoint {
                        "bg-foreground font-medium text-background"
                    } else {
                        ""
                    },
                    if !endpoint && !in_range && !unavailable {
                        "hover:bg-muted"
                    } else {
                        ""
                    },
                    if is_today && !endpoint {
                        "font-semibold after:absolute after:bottom-1 after:h-[3px] after:w-[3px] after:rounded-full after:bg-current"
                    } else {
                        ""
                    },
                ),
                onmounted: {
                    let is_active = grid.active == Some(date.as_str());
                    move |event: MountedEvent| {
                        if is_active {
                            spawn(async move {
                                let _ = event.set_focus(true).await;
                            });
                        }
                    }
                },
                onmouseenter: on_enter,
                onfocus: on_focus,
                onclick: on_click,
                onkeydown: on_key,
                "{parts(&date).2}"
            }
        }
    }
}

struct Choice {
    id: String,
    label: String,
    disabled: bool,
    selected: bool,
}

fn choice_grid(
    context: &RangeContext,
    view: CalView,
    year: i64,
    month_index: i64,
    (page, page_end): (i64, i64),
) -> Element {
    let choices: Vec<Choice> = match view {
        CalView::Months => (0..12)
            .map(|index| {
                let id = format!("{year:04}-{:02}-01", index + 1);
                Choice {
                    label: MONTH_NAMES[index as usize].to_string(),
                    disabled: context.min.as_ref().is_some_and(|min| id < month_of(min))
                        || context.max.as_ref().is_some_and(|max| id > month_of(max)),
                    selected: index + 1 == month_index,
                    id,
                }
            })
            .collect(),
        CalView::Years => (page..=page_end)
            .map(|choice_year| Choice {
                id: choice_year.to_string(),
                label: choice_year.to_string(),
                disabled: context
                    .min
                    .as_ref()
                    .is_some_and(|min| choice_year < parts(min).0)
                    || context
                        .max
                        .as_ref()
                        .is_some_and(|max| choice_year > parts(max).0),
                selected: choice_year == year,
            })
            .collect(),
        CalView::Days => Vec::new(),
    };
    let active_id = choices
        .iter()
        .find(|choice| choice.selected && !choice.disabled)
        .or_else(|| choices.iter().find(|choice| !choice.disabled))
        .map(|choice| choice.id.clone());

    rsx! {
        fieldset {
            class: "grid min-h-[15.25rem] grid-cols-3 content-start gap-2 border-0 p-0 pt-1",
            for choice in choices {
                {
                    let context = context.clone();
                    let id = choice.id.clone();
                    let selected = choice.selected;
                    let tabindex = if active_id.as_deref() == Some(choice.id.as_str()) { "0" } else { "-1" };
                    let onclick = move |_| {
                        let next_month = if view == CalView::Months {
                            id.clone()
                        } else {
                            let (current_year, current_month, _) = parts(&(context.month)());
                            let chosen_year: i64 = id.parse().unwrap_or(current_year);
                            format!("{chosen_year:04}-{current_month:02}-01")
                        };
                        let mut month = context.month;
                        month.set(month_of(&next_month));
                        let mut cursor = context.cursor;
                        cursor.set(Some(month_of(&next_month)));
                        let mut preview = context.preview;
                        preview.set(None);
                        let mut view = context.view;
                        view.set(CalView::Days);
                    };
                    rsx! {
                        button {
                            key: "{choice.id}",
                            r#type: "button",
                            disabled: choice.disabled,
                            "aria-pressed": if selected { "true" } else { "false" },
                            tabindex: "{tabindex}",
                            class: cn!(
                                "flex h-[3.25rem] items-center justify-center rounded-xl px-1 text-[13px] tabular-nums transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-25",
                                if selected {
                                    "bg-foreground font-medium text-background"
                                } else {
                                    "bg-muted/40 hover:bg-muted"
                                },
                            ),
                            onclick,
                            "{choice.label}"
                        }
                    }
                }
            }
        }
    }
}

fn navigate_key(context: &RangeContext, date: &str, event: &KeyboardEvent) {
    let key = event.key();
    let weekday = weekday_of(date);
    let (offset, skip_unavailable) = match key {
        Key::ArrowLeft => (-1, true),
        Key::ArrowRight => (1, true),
        Key::ArrowUp => (-7, true),
        Key::ArrowDown => (7, true),
        Key::Home => (-weekday, false),
        Key::End => (6 - weekday, false),
        Key::PageUp => (-1, true),
        Key::PageDown => (1, true),
        _ => return,
    };
    event.prevent_default();

    let next = if matches!(key, Key::PageUp | Key::PageDown) {
        let step = if key == Key::PageUp { -1 } else { 1 };
        let step = if event.modifiers().shift() {
            step * 12
        } else {
            step
        };
        // Keep the day of the month, and stop at the min/max month rather than
        // walking into a month where every day is disabled.
        let candidate = shift_month(date, step);
        let target_month = month_of(&candidate);
        if context
            .min
            .as_ref()
            .is_some_and(|min| target_month < month_of(min))
            || context
                .max
                .as_ref()
                .is_some_and(|max| target_month > month_of(max))
        {
            return;
        }
        candidate
    } else {
        let mut next = iso_date(parse_date(date) + offset);
        if skip_unavailable {
            let step = if offset < 0 { -1 } else { 1 };
            let mut attempts = 0;
            while context.unavailable(&next) && attempts < 366 {
                if context.min.as_ref().is_some_and(|min| next < *min)
                    || context.max.as_ref().is_some_and(|max| next > *max)
                {
                    return;
                }
                next = iso_date(parse_date(&next) + step);
                attempts += 1;
            }
        }
        if context.unavailable(&next) {
            return;
        }
        next
    };

    let mut month = context.month;
    month.set(month_of(&next));
    let mut cursor = context.cursor;
    cursor.set(Some(next.clone()));
    let mut preview = context.preview;
    if (context.value)().is_some_and(|range| range.to.is_none()) {
        preview.set(Some(next.clone()));
    } else {
        preview.set(None);
    }
}

#[component]
pub fn DateRangePickerFooter(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: cn!("mt-2.5 border-t border-border pt-2.5", class),
            {children}
        }
    }
}

#[component]
pub fn DateRangePickerSelection(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let text = match (context.value)() {
        Some(range) => match range.to {
            Some(to) => {
                let days = parse_date(&to) - parse_date(&range.from) + 1;
                format!("{days} {} selected", if days == 1 { "day" } else { "days" })
            }
            None => "Choose an end date".to_string(),
        },
        None => "Choose a start date".to_string(),
    };
    rsx! {
        p {
            "aria-live": "polite",
            class: cn!("text-xs text-muted-foreground", class),
            "{text}"
        }
    }
}

#[component]
pub fn DateRangePickerClear(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let empty = (context.value)().is_none();
    rsx! {
        button {
            r#type: "button",
            "aria-label": "Clear date range",
            disabled: empty || context.disabled,
            class: cn!(
                "rounded-md px-1.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-30",
                class,
            ),
            onclick: move |_| {
                let mut value = context.value;
                value.set(None);
                let mut preview = context.preview;
                preview.set(None);
                if let Some(on_value_change) = context.on_value_change {
                    on_value_change.call(None);
                }
            },
            "Clear"
        }
    }
}

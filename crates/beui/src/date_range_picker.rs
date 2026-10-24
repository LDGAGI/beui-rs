//! `motion/date-range-picker.tsx` — the range calendar.
//!
//! Calendar dates are plain `YYYY-MM-DD` strings (never a browser timezone),
//! and the civil-date maths is the same algorithm the browser's UTC `Date`
//! uses: days since 1970-01-01.
//!
//! Differences from the source : the month/year
//! choice views, arrow-key navigation inside the grid, the screen-reader
//! announcements and the `MorphPopover`-driven panel are not ported; the panel
//! is a presence-gated box anchored under the trigger. Selection, the ordered
//! range, min/max disabling and the summary/clear/footer parts are all here.

use dioxus::prelude::*;

use beui_core::cn;
use beui_core::hooks::use_reduced_motion;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::icons::{IconChevronDown, IconChevronLeft, IconChevronRight};
use crate::motion::{spring, tween, Ease};

/// `DateRange`.
#[derive(Clone, Debug, PartialEq)]
pub struct DateRange {
    pub from: String,
    pub to: Option<String>,
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

/// The reference "today" for the initial month and the today ring.
///
/// There is no cross-renderer clock here (the browser's `Date` lives behind
/// `platform`, and the desktop WebView behind `eval`), so the picker uses the
/// `max` bound the caller passes — which the preview sets to today — and falls
/// back to the first of the month otherwise.
fn today(context: &RangeContext) -> String {
    context
        .max
        .clone()
        .or_else(|| context.min.clone())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

// --- context ----------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct RangeContext {
    value: Signal<Option<DateRange>>,
    month: Signal<String>,
    preview: Signal<Option<String>>,
    min: Option<String>,
    max: Option<String>,
    disabled: bool,
    open: Signal<bool>,
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
        if let Some(on_value_change) = self.on_value_change {
            on_value_change.call(next);
        }
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

    let internal = use_signal(|| initial_range.clone());
    let controlled_value = use_signal(|| value.clone());
    let month = use_signal(|| month_of(&initial_month));
    let preview = use_signal(|| None::<String>);
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
        min: min.clone(),
        max: max.clone(),
        disabled,
        open: open_signal,
        on_value_change,
    };
    let _ = close_on_select;
    use_context_provider(|| context.clone());

    rsx! {
        fieldset {
            disabled,
            class: cn!("relative m-0 flex w-fit min-w-0 flex-col gap-3 border-0 p-0", class),
            legend { class: "sr-only", "{label}" }
            {children}
        }
    }
}

#[component]
pub fn DateRangePickerTrigger(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let label = (context.value)()
        .map(|range| match range.to {
            Some(to) => format!("{} – {}", range.from, to),
            None => range.from,
        })
        .unwrap_or_else(|| "Select dates".to_string());

    rsx! {
        button {
            r#type: "button",
            "data-date-range-trigger": "true",
            "aria-haspopup": "dialog",
            disabled: context.disabled,
            class: cn!(
                "inline-flex w-fit items-center gap-2 rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors hover:border-border-strong focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            onclick: move |_| {
                let mut open = context.open;
                open.toggle();
            },
            span { class: "tabular-nums", "{label}" }
            IconChevronDown { class: "size-4 text-muted-foreground" }
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
            class: cn!("absolute left-0 top-full z-50 mt-2 origin-top rounded-2xl shadow-xl", class),
            {children}
        }
    }
}

#[component]
pub fn DateRangePickerCalendar(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: cn!("flex w-72 flex-col gap-3 rounded-2xl border border-border bg-background p-3", class),
            {children}
        }
    }
}

#[component]
pub fn DateRangePickerHeader(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let month = (context.month)();
    let days = parse_date(&month);
    let (year, m, _) = civil_from_days(days);
    let at_min = context
        .min
        .as_ref()
        .is_some_and(|min| month_of(min) >= month);
    let at_max = context
        .max
        .as_ref()
        .is_some_and(|max| month_of(max) <= month);
    let month_prev = month.clone();
    let month_next = month.clone();

    rsx! {
        div { class: cn!("flex items-center justify-between gap-2", class),
            button {
                r#type: "button",
                "aria-label": "Previous month",
                disabled: at_min,
                class: "grid size-8 place-items-center rounded-lg text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-30",
                onclick: move |_| {
                    let mut signal = context.month;
                    signal.set(shift_month(&month_prev, -1));
                },
                IconChevronLeft { class: "size-4" }
            }
            span { class: "text-sm font-medium tabular-nums", "{MONTH_NAMES[(m - 1) as usize]} {year}" }
            button {
                r#type: "button",
                "aria-label": "Next month",
                disabled: at_max,
                class: "grid size-8 place-items-center rounded-lg text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-30",
                onclick: move |_| {
                    let mut signal = context.month;
                    signal.set(shift_month(&month_next, 1));
                },
                IconChevronRight { class: "size-4" }
            }
        }
    }
}

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
const WEEKDAYS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

#[component]
pub fn DateRangePickerGrid(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let month = (context.month)();
    let first = parse_date(&month);
    let (_, m, _) = civil_from_days(first);
    let year = civil_from_days(first).0;
    let count = last_day_of(year, m);
    // Sunday = 0, matching the `en-US` week the source formats.
    let lead = (first + 4).rem_euclid(7);
    let shown = context.shown();
    let reference_today = today(&context);
    let cells: Vec<i64> = (0..42).collect();

    rsx! {
        div { class: cn!("flex flex-col gap-1", class),
            div { class: "grid grid-cols-7 gap-1",
                for (i, day) in WEEKDAYS.iter().enumerate() {
                    span {
                        key: "{day}",
                        class: cn!("grid h-7 place-items-center text-xs text-muted-foreground", if i == 0 { "text-destructive/70" } else { "" }),
                        "{day}"
                    }
                }
            }
            div { class: "grid grid-cols-7 gap-1",
                for cell in cells {
                    {
                        let day_number = cell - lead;
                        if day_number < 0 || day_number >= count {
                            rsx! { span { key: "pad-{cell}", class: "size-8" } }
                        } else {
                            let context = context.clone();
                            let date = iso_date(first + day_number);
                            let unavailable = context.unavailable(&date);
                            let is_start = shown.as_ref().is_some_and(|r| r.from == date);
                            let is_end = shown.as_ref().and_then(|r| r.to.clone()).as_deref() == Some(date.as_str());
                            let in_range = shown.as_ref().is_some_and(|r| {
                                let to = r.to.clone().unwrap_or_else(|| r.from.clone());
                                date.as_str() >= r.from.as_str() && date.as_str() <= to.as_str()
                            });
                            let is_today = date == reference_today;
                            let selected = is_start || is_end;
                            let date_for_click = date.clone();
                            let date_hover = date.clone();
                            let date_leave = date.clone();
                            rsx! {
                                button {
                                    key: "{date}",
                                    r#type: "button",
                                    disabled: unavailable,
                                    "data-date": "{date}",
                                    "data-selected": selected.then_some("true"),
                                    "data-in-range": in_range.then_some("true"),
                                    class: cn!(
                                        "grid size-8 place-items-center rounded-full text-sm tabular-nums outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring",
                                        if selected {
                                            "bg-primary text-primary-foreground"
                                        } else if in_range {
                                            "bg-muted text-foreground"
                                        } else {
                                            "text-foreground hover:bg-muted"
                                        },
                                        if is_today && !selected { "font-semibold ring-1 ring-inset ring-border-strong" } else { "" },
                                        if unavailable { "cursor-not-allowed opacity-30" } else { "" },
                                    ),
                                    onmouseenter: move |_| {
                                        let value = (context.value)();
                                        if value.as_ref().is_some_and(|r| r.to.is_none()) {
                                            let mut preview = context.preview;
                                            preview.set(Some(date_hover.clone()));
                                        }
                                    },
                                    onmouseleave: move |_| {
                                        let _ = &date_leave;
                                        let mut preview = context.preview;
                                        preview.set(None);
                                    },
                                    onclick: move |_| context.select(&date_for_click),
                                    "{civil_from_days(parse_date(&date)).2}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DateRangePickerFooter(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div { class: cn!("flex items-center gap-2 border-t border-border pt-3", class), {children} }
    }
}

#[component]
pub fn DateRangePickerSelection(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    let text = match (context.value)() {
        Some(range) => match range.to {
            Some(to) => format!("{} → {}", range.from, to),
            None => format!("{} → pick an end date", range.from),
        },
        None => "Pick a start date".to_string(),
    };
    rsx! {
        p {
            "aria-live": "polite",
            class: cn!("text-sm tabular-nums text-muted-foreground", class),
            "{text}"
        }
    }
}

#[component]
pub fn DateRangePickerClear(#[props(default)] class: Option<String>) -> Element {
    let context = use_context::<RangeContext>();
    rsx! {
        button {
            r#type: "button",
            class: cn!(
                "rounded-lg border border-border px-2.5 py-1 text-xs text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            onclick: move |_| {
                let mut value = context.value;
                value.set(None);
                if let Some(on_value_change) = context.on_value_change {
                    on_value_change.call(None);
                }
            },
            "Clear"
        }
    }
}

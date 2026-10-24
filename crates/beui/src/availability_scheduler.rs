//! `motion/availability-scheduler/*` — the weekly availability editor.
//!
//! Seven day rows, each springing between its time ranges and "Unavailable",
//! with add/remove range controls, a time dropdown per field, and a copy menu
//! that clones one day's hours onto others.
//!
//! Deviations from the source, all forced by the platform:
//!
//! - **The copy menu renders in place.** The source's `MorphPopoverContent`
//!   portals to `document.body`; on this platform a portaled subtree is outside
//!   the mount root that Dioxus delegates bubbling events through, so its
//!   checkboxes and Apply button would never receive a click. The panel keeps
//!   the morph clip/scale entrance but
//!   is an `absolute` layer inside the row's action group, and it flips to the
//!   side with room using the wrapper's measured box.
//! - **`Tooltip` becomes a native `title`.** The source wraps every icon button
//!   in `Tooltip`; a tooltip overlay per button is the same portaled-overlay
//!   problem, and the accessible name is already on each button (the
//!   `AttachmentUpload` precedent).
//! - **`layout="position"` goes through `beui_core::flip`.** Every day row and
//!   every range row carries a scope whose state is the whole week (or the day),
//!   so a change anywhere re-commits all of them — which is what motion's
//!   layout animation does.
//! - **`AnimatePresence` is `beui_core::presence`**, with the reduced-motion
//!   branch collapsing the enter/exit springs to a zero-length tween.
//! - **The trigger is a plain `<button>` with a `scale` style**, not a
//!   `motion.button` with `whileTap`; the press spring is the same `SPRING_PRESS`
//!   (the repo's `IconButton` pattern from `AttachmentUpload`/`Table`).
//! - **`WeekAvailability` is a fixed seven-element array**, because Rust has no
//!   record type keyed by a string union; `get`/`with_day` are the accessors.
//! - **The step is clamped to at least one minute** (`buildOptions(0)` would
//!   spin forever in JS, so the source assumes a sane prop).

use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use dioxus_motion::prelude::{use_motion, AnimationManager, Duration, Time, TimeProvider};

use beui_core::ease::{tween_ease_out, SPRING_LAYOUT, SPRING_PANEL, SPRING_PRESS};
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};

use crate::motion::number;
use crate::prelude::*;
use crate::{Select, SelectContent, SelectItem, SelectTrigger, SelectValue};

// --- public types -----------------------------------------------------------

/// `DayKey`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum DayKey {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl DayKey {
    /// `WEEKDAYS`, in display order.
    pub const ALL: [DayKey; 7] = [
        DayKey::Mon,
        DayKey::Tue,
        DayKey::Wed,
        DayKey::Thu,
        DayKey::Fri,
        DayKey::Sat,
        DayKey::Sun,
    ];

    /// The lowercase key the source uses in range ids and panel keys.
    pub fn slug(self) -> &'static str {
        match self {
            DayKey::Mon => "mon",
            DayKey::Tue => "tue",
            DayKey::Wed => "wed",
            DayKey::Thu => "thu",
            DayKey::Fri => "fri",
            DayKey::Sat => "sat",
            DayKey::Sun => "sun",
        }
    }

    /// `WEEKDAYS[].label`.
    pub fn label(self) -> &'static str {
        match self {
            DayKey::Mon => "Monday",
            DayKey::Tue => "Tuesday",
            DayKey::Wed => "Wednesday",
            DayKey::Thu => "Thursday",
            DayKey::Fri => "Friday",
            DayKey::Sat => "Saturday",
            DayKey::Sun => "Sunday",
        }
    }

    fn index(self) -> usize {
        match self {
            DayKey::Mon => 0,
            DayKey::Tue => 1,
            DayKey::Wed => 2,
            DayKey::Thu => 3,
            DayKey::Fri => 4,
            DayKey::Sat => 5,
            DayKey::Sun => 6,
        }
    }
}

/// `TimeRange`.
#[derive(Clone, PartialEq, Debug)]
pub struct TimeRange {
    pub id: String,
    pub start: String,
    pub end: String,
}

impl TimeRange {
    pub fn new(id: impl Into<String>, start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            start: start.into(),
            end: end.into(),
        }
    }
}

/// `DayAvailability`.
#[derive(Clone, PartialEq, Debug)]
pub struct DayAvailability {
    pub enabled: bool,
    pub ranges: Vec<TimeRange>,
}

impl DayAvailability {
    pub fn unavailable() -> Self {
        Self {
            enabled: false,
            ranges: Vec::new(),
        }
    }

    pub fn day(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            enabled: true,
            ranges: vec![TimeRange::new("range-0", start, end)],
        }
    }
}

/// `WeekAvailability` — the seven days, indexed by [`DayKey`].
#[derive(Clone, PartialEq, Debug)]
pub struct WeekAvailability([DayAvailability; 7]);

impl WeekAvailability {
    pub fn new(days: [DayAvailability; 7]) -> Self {
        Self(days)
    }

    pub fn get(&self, day: DayKey) -> &DayAvailability {
        &self.0[day.index()]
    }

    /// The week with one day replaced.
    pub fn with_day(&self, day: DayKey, value: DayAvailability) -> Self {
        let mut next = self.0.clone();
        next[day.index()] = value;
        Self(next)
    }
}

impl Default for WeekAvailability {
    fn default() -> Self {
        default_week()
    }
}

/// `TimeOption`.
#[derive(Clone, PartialEq, Debug)]
pub struct TimeOption {
    pub value: String,
    pub label: String,
}

/// Which endpoint of a range just moved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RangeEdge {
    Start,
    End,
}

/// `panelKey` — one time field week-wide.
pub fn panel_key(day: DayKey, range_id: &str, edge: RangeEdge) -> String {
    format!(
        "{}:{range_id}:{}",
        day.slug(),
        match edge {
            RangeEdge::Start => "start",
            RangeEdge::End => "end",
        }
    )
}

/// `toMinutes`.
pub fn to_minutes(value: &str) -> i64 {
    let mut parts = value.split(':');
    let hours = parts.next().and_then(|part| part.parse::<i64>().ok());
    let minutes = parts.next().and_then(|part| part.parse::<i64>().ok());
    match (hours, minutes) {
        (Some(hours), Some(minutes)) => hours * 60 + minutes,
        _ => 0,
    }
}

/// `toValue` — clamped to the last minute of the day and zero-padded.
pub fn to_value(minutes: i64) -> String {
    let clamped = minutes.clamp(0, 24 * 60 - 1);
    format!("{:02}:{:02}", clamped / 60, clamped % 60)
}

/// `label12`.
pub fn label12(value: &str) -> String {
    let minutes = to_minutes(value);
    let hours = minutes / 60;
    let suffix = if hours < 12 { "AM" } else { "PM" };
    let hour12 = if hours % 12 == 0 { 12 } else { hours % 12 };
    format!("{hour12}:{:02} {suffix}", minutes % 60)
}

/// `buildOptions`.
pub fn build_options(step: i64) -> Vec<TimeOption> {
    let step = step.max(1) as usize;
    (0..24 * 60)
        .step_by(step)
        .map(|minutes| {
            let value = to_value(minutes as i64);
            TimeOption {
                label: label12(&value),
                value,
            }
        })
        .collect()
}

/// Motion's step order for a value that may sit off the grid.
fn snap_to_option(minutes: i64, slots: &[i64]) -> i64 {
    let mut best = slots[0];
    for slot in slots {
        if (slot - minutes).abs() < (best - minutes).abs() {
            best = *slot;
        }
    }
    best
}

/// `withCurrentOption` — keep an off-grid persisted value selectable.
fn with_current_option(
    filtered: Vec<TimeOption>,
    all: &[TimeOption],
    current: Option<&str>,
) -> Vec<TimeOption> {
    let Some(current) = current else {
        return filtered;
    };
    if filtered.iter().any(|option| option.value == current) {
        return filtered;
    }
    let extra = all
        .iter()
        .find(|option| option.value == current)
        .cloned()
        .unwrap_or_else(|| TimeOption {
            value: current.to_string(),
            label: label12(current),
        });
    let mut out = filtered;
    out.push(extra);
    out.sort_by_key(|option| to_minutes(&option.value));
    out
}

/// `clampRange` — same-day ranges only; the just-chosen endpoint stays put.
pub fn clamp_range(
    start: &str,
    end: &str,
    options: &[TimeOption],
    changed: RangeEdge,
) -> (String, String) {
    let slots: Vec<i64> = options
        .iter()
        .map(|option| to_minutes(&option.value))
        .collect();
    if slots.is_empty() {
        return (start.to_string(), end.to_string());
    }

    let start_minutes = to_minutes(start);
    let end_minutes = to_minutes(end);
    if end_minutes > start_minutes {
        return (start.to_string(), end.to_string());
    }

    let keep_or_snap = |value: &str| {
        let minutes = to_minutes(value);
        if slots.contains(&minutes) {
            minutes
        } else {
            snap_to_option(minutes, &slots)
        }
    };

    if changed == RangeEdge::End {
        let end_slot = keep_or_snap(end);
        let earlier = slots.iter().rev().find(|slot| **slot < end_slot).copied();
        if earlier.is_none() && slots.len() > 1 {
            // Midnight cannot end a positive same-day range: use the first pair.
            return (to_value(slots[0]), to_value(slots[1]));
        }
        return (
            to_value(earlier.unwrap_or_else(|| keep_or_snap(start))),
            to_value(end_slot),
        );
    }

    let start_slot = keep_or_snap(start);
    let later = slots.iter().find(|slot| **slot > start_slot).copied();
    if later.is_none() && slots.len() > 1 {
        // The last slot cannot start a positive range: use the final pair.
        return (
            to_value(slots[slots.len() - 2]),
            to_value(slots[slots.len() - 1]),
        );
    }
    (
        to_value(start_slot),
        to_value(later.unwrap_or_else(|| keep_or_snap(end))),
    )
}

/// `startOptions`.
pub fn start_options(options: &[TimeOption], end: &str, current: Option<&str>) -> Vec<TimeOption> {
    let end_minutes = to_minutes(end);
    let filtered: Vec<TimeOption> = options
        .iter()
        .filter(|option| to_minutes(&option.value) < end_minutes)
        .cloned()
        .collect();
    // An invalid midnight end has no earlier option; expose midnight so choosing
    // it can move the end forward through `clamp_range` instead of trapping the row.
    let recovery = if filtered.is_empty() {
        options.iter().take(1).cloned().collect()
    } else {
        filtered
    };
    with_current_option(recovery, options, current)
}

/// `endOptions`.
pub fn end_options(options: &[TimeOption], start: &str, current: Option<&str>) -> Vec<TimeOption> {
    let start_minutes = to_minutes(start);
    let filtered: Vec<TimeOption> = options
        .iter()
        .filter(|option| to_minutes(&option.value) > start_minutes)
        .cloned()
        .collect();
    // An invalid last-slot start has no later option; expose the last slot so
    // choosing it can move the start backward through `clamp_range`.
    let recovery = if filtered.is_empty() {
        options.iter().rev().take(1).cloned().collect()
    } else {
        filtered
    };
    with_current_option(recovery, options, current)
}

/// `defaultWeek` — Mon–Fri 9–5, weekends off, with fixed ids.
pub fn default_week() -> WeekAvailability {
    let workday = |day: DayKey| DayAvailability {
        enabled: true,
        ranges: vec![TimeRange::new(
            format!("{}-0", day.slug()),
            "09:00",
            "17:00",
        )],
    };
    let off = |day: DayKey| DayAvailability {
        enabled: false,
        ranges: vec![TimeRange::new(
            format!("{}-0", day.slug()),
            "09:00",
            "17:00",
        )],
    };
    WeekAvailability::new([
        workday(DayKey::Mon),
        workday(DayKey::Tue),
        workday(DayKey::Wed),
        workday(DayKey::Thu),
        workday(DayKey::Fri),
        off(DayKey::Sat),
        off(DayKey::Sun),
    ])
}

/// The panels the week currently puts on screen.
fn live_panels(week: &WeekAvailability) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for day in DayKey::ALL {
        let state = week.get(day);
        if !state.enabled {
            continue;
        }
        for range in &state.ranges {
            ids.insert(panel_key(day, &range.id, RangeEdge::Start));
            ids.insert(panel_key(day, &range.id, RangeEdge::End));
        }
    }
    ids
}

/// A row of the day's range list: a time range, or the "Unavailable" marker.
#[derive(Clone, PartialEq, Debug)]
enum RangeSlot {
    Range(TimeRange),
    Unavailable,
}

impl RangeSlot {
    /// The presence key: the range id, or the marker's constant.
    fn key(&self) -> String {
        match self {
            RangeSlot::Range(range) => range.id.clone(),
            RangeSlot::Unavailable => "unavailable".to_string(),
        }
    }

    fn elevated(&self, open_range: Option<&str>) -> bool {
        match self {
            RangeSlot::Range(range) => open_range == Some(range.id.as_str()),
            RangeSlot::Unavailable => false,
        }
    }
}

/// The presence child for one slot: `AnimatePresence` needs the key on the
/// node the loop body produces, so the dispatch lives one component down.
#[component]
#[allow(clippy::too_many_arguments)]
fn RangeSlotView(
    slot: RangeSlot,
    day: DayKey,
    options: Rc<Vec<TimeOption>>,
    reduce: bool,
    week: WeekAvailability,
    elevated: bool,
    open_panel: Option<String>,
    on_start: EventHandler<(String, String)>,
    on_end: EventHandler<(String, String)>,
    on_remove: EventHandler<String>,
    on_panel_open_change: EventHandler<(String, String, bool)>,
) -> Element {
    match slot {
        RangeSlot::Range(range) => rsx! {
            RangeRow {
                day,
                range,
                options,
                reduce,
                week,
                elevated,
                open_panel,
                on_start,
                on_end,
                on_remove,
                on_panel_open_change,
            }
        },
        RangeSlot::Unavailable => rsx! { UnavailableRow { reduce } },
    }
}

/// One endpoint patch, applied then clamped.
#[derive(Clone, PartialEq, Debug)]
enum RangePatch {
    Start(String),
    End(String),
}

fn with_range_patch(
    state: &DayAvailability,
    id: &str,
    patch: &RangePatch,
    options: &[TimeOption],
) -> DayAvailability {
    let ranges = state
        .ranges
        .iter()
        .map(|range| {
            if range.id != id {
                return range.clone();
            }
            let (start, end, changed) = match patch {
                RangePatch::Start(value) => (value.clone(), range.end.clone(), RangeEdge::Start),
                RangePatch::End(value) => (range.start.clone(), value.clone(), RangeEdge::End),
            };
            let (start, end) = clamp_range(&start, &end, options, changed);
            TimeRange {
                id: range.id.clone(),
                start,
                end,
            }
        })
        .collect();
    DayAvailability {
        enabled: state.enabled,
        ranges,
    }
}

// --- presence configs -------------------------------------------------------

fn range_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        );
    }
    let mut enter = motion_style! { opacity: 0.0, y: -6.0 };
    enter.add_css_property("filter", "blur(4px)");
    let mut center = motion_style! { opacity: 1.0, y: 0.0 };
    center.add_css_property("filter", "blur(0px)");
    let mut exit = motion_style! { opacity: 0.0, y: -4.0 };
    exit.add_css_property("filter", "blur(4px)");
    PresenceConfig::with_transitions(
        enter,
        center,
        exit,
        spring(SPRING_LAYOUT),
        spring(SPRING_LAYOUT),
    )
}

fn unavailable_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        );
    }
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, y: -4.0 },
        motion_style! { opacity: 1.0, y: 0.0 },
        motion_style! { opacity: 0.0, y: -4.0 },
        spring(SPRING_LAYOUT),
        spring(SPRING_LAYOUT),
    )
}

/// The morph panel's entrance, `surface_config` from `popover-morph`.
fn copy_panel_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(120),
            tween_ease_out(120),
        );
    }
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, scale: 0.96 },
        motion_style! { opacity: 1.0, scale: 1.0 },
        motion_style! { opacity: 0.0, scale: 0.96 },
        spring(SPRING_PANEL),
        spring(SPRING_PANEL),
    )
}

/// `copy-menu`'s icon swap, keyed on `copied`.
fn glyph_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        );
    }
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, scale: 0.5 },
        motion_style! { opacity: 1.0, scale: 1.0 },
        motion_style! { opacity: 0.0, scale: 0.5 },
        spring(SPRING_PRESS),
        spring(SPRING_PRESS),
    )
}

// --- scheduler --------------------------------------------------------------

/// `AvailabilityScheduler`.
#[component]
pub fn AvailabilityScheduler(
    /// Controlled value; `Some` means controlled (Rust has no `undefined`).
    #[props(default = None)]
    value: Option<WeekAvailability>,
    #[props(default = None)] default_value: Option<WeekAvailability>,
    #[props(default = None)] on_change: Option<EventHandler<WeekAvailability>>,
    /// Minutes between selectable times. Default 30.
    #[props(default = 30)]
    step: i64,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();

    // The rendered week follows an incoming controlled value, but toggles land
    // in the same store so a parent that ignores `on_change` cannot freeze the
    // rows (the `AttachmentUpload` / `Table` convention).
    let mut week_signal = use_signal(|| default_value.clone().unwrap_or_else(default_week));
    if let Some(controlled) = value.as_ref() {
        if *week_signal.peek() != *controlled {
            week_signal.set(controlled.clone());
        }
    }
    let mut open_panel = use_signal(|| None::<String>);
    let mut open_day = use_signal(|| None::<DayKey>);
    let copy_counter = use_hook(|| Rc::new(Cell::new(0u32)));

    let options: Rc<Vec<TimeOption>> = Rc::new(build_options(step));

    let commit = EventHandler::new(move |next: WeekAvailability| {
        week_signal.set(next.clone());
        if let Some(callback) = on_change {
            callback.call(next);
        }
    });

    // A field that leaves — its day switched off, its range removed — never
    // reports its panel closed, so drop its id here.
    use_effect(move || {
        let week = week_signal.read().clone();
        let open = open_panel.peek().clone();
        if let Some(open) = open {
            if !live_panels(&week).contains(&open) {
                open_panel.set(None);
            }
        }
    });

    let set_day = EventHandler::new(move |(day, next): (DayKey, DayAvailability)| {
        let week = week_signal.read().clone();
        commit.call(week.with_day(day, next));
    });

    let panel_open_change = EventHandler::new(move |(day, id, open): (DayKey, String, bool)| {
        let current = open_panel.peek().clone();
        open_panel.set(if open {
            Some(id)
        } else if current.as_deref() == Some(id.as_str()) {
            None
        } else {
            current
        });
        // Elevation stays on the row that opened last so the panel's collapse
        // animation finishes above its neighbours.
        if open {
            open_day.set(Some(day));
        }
    });

    let copy_day = {
        let counter = copy_counter.clone();
        EventHandler::new(move |(from, targets): (DayKey, Vec<DayKey>)| {
            let week = week_signal.read().clone();
            let source = week.get(from).clone();
            let mut next = week.clone();
            for target in targets {
                let ranges = source
                    .ranges
                    .iter()
                    .map(|range| {
                        let number = counter.get();
                        counter.set(number + 1);
                        TimeRange::new(
                            format!("{}-c{number}", target.slug()),
                            range.start.clone(),
                            range.end.clone(),
                        )
                    })
                    .collect();
                next = next.with_day(
                    target,
                    DayAvailability {
                        enabled: source.enabled,
                        ranges,
                    },
                );
            }
            commit.call(next);
        })
    };

    let week_now = week_signal.read().clone();
    let open_now = open_panel.read().clone();
    let open_day_now = *open_day.read();

    rsx! {
        div {
            "data-beui-scheduler": "",
            class: cn!("w-full max-w-xl divide-y divide-border", class),
            for day in DayKey::ALL {
                DayRow {
                    key: "{day.slug()}",
                    day,
                    label: day.label().to_string(),
                    week: week_now.clone(),
                    options: options.clone(),
                    reduce,
                    elevated: open_day_now == Some(day),
                    open_panel: open_now.clone(),
                    on_change: set_day,
                    on_copy: copy_day,
                    on_panel_open_change: panel_open_change,
                }
            }
        }
    }
}

// --- one day ----------------------------------------------------------------

#[component]
fn DayRow(
    day: DayKey,
    label: String,
    /// The whole week: the row's layout animation has to re-commit when *any*
    /// day changes, which is what `layout="position"` on every row does.
    week: WeekAvailability,
    options: Rc<Vec<TimeOption>>,
    reduce: bool,
    elevated: bool,
    open_panel: Option<String>,
    on_change: EventHandler<(DayKey, DayAvailability)>,
    on_copy: EventHandler<(DayKey, Vec<DayKey>)>,
    on_panel_open_change: EventHandler<(DayKey, String, bool)>,
) -> Element {
    let id_counter = use_hook(|| Rc::new(Cell::new(0u32)));
    let mut open_range = use_signal(|| None::<String>);
    let state = week.get(day).clone();

    let flip = use_flip(
        week.clone(),
        FlipOptions::default()
            .position_only()
            .transition(spring(SPRING_LAYOUT)),
    );

    let next_id = {
        let counter = id_counter.clone();
        move || {
            let number = counter.get();
            counter.set(number + 1);
            format!("{}-n{number}", day.slug())
        }
    };

    // --- switch ---------------------------------------------------------
    let set_enabled = {
        let state = state.clone();
        let next_id = next_id.clone();
        EventHandler::new(move |enabled: bool| {
            if enabled && state.ranges.is_empty() {
                on_change.call((
                    day,
                    DayAvailability {
                        enabled,
                        ranges: vec![TimeRange::new(next_id(), "09:00", "17:00")],
                    },
                ));
            } else {
                on_change.call((
                    day,
                    DayAvailability {
                        enabled,
                        ranges: state.ranges.clone(),
                    },
                ));
            }
        })
    };

    // --- ranges ---------------------------------------------------------
    let add_range = {
        let state = state.clone();
        let next_id = next_id.clone();
        EventHandler::new(move |_: ()| {
            let start = state
                .ranges
                .last()
                .map(|last| (to_minutes(&last.end) + 60).min(24 * 60 - 60))
                .unwrap_or(540);
            let mut ranges = state.ranges.clone();
            ranges.push(TimeRange::new(
                next_id(),
                to_value(start),
                to_value(start + 60),
            ));
            on_change.call((
                day,
                DayAvailability {
                    enabled: true,
                    ranges,
                },
            ));
        })
    };

    let remove_range = {
        let state = state.clone();
        EventHandler::new(move |id: String| {
            let ranges: Vec<TimeRange> = state
                .ranges
                .iter()
                .filter(|range| range.id != id)
                .cloned()
                .collect();
            // Removing the last slot marks the day unavailable.
            on_change.call((
                day,
                DayAvailability {
                    enabled: !ranges.is_empty(),
                    ranges,
                },
            ));
        })
    };

    let set_start = {
        let state = state.clone();
        let options = options.clone();
        EventHandler::new(move |(id, value): (String, String)| {
            on_change.call((
                day,
                with_range_patch(&state, &id, &RangePatch::Start(value), &options),
            ));
        })
    };

    let set_end = {
        let state = state.clone();
        let options = options.clone();
        EventHandler::new(move |(id, value): (String, String)| {
            on_change.call((
                day,
                with_range_patch(&state, &id, &RangePatch::End(value), &options),
            ));
        })
    };

    // The row marks the range that opened last so its panel paints above the
    // ranges stacked against it.
    let range_panel_change =
        EventHandler::new(move |(range_id, panel_id, open): (String, String, bool)| {
            if open {
                open_range.set(Some(range_id));
            }
            on_panel_open_change.call((day, panel_id, open));
        });

    // One `for` over a slot list rather than `for` + `if`: `AnimatePresence`
    // normalizes a single dynamic root, and two roots in the same child set
    // would make it reject the whole subtree.
    let slots: Vec<RangeSlot> = if state.enabled {
        state.ranges.iter().cloned().map(RangeSlot::Range).collect()
    } else {
        vec![RangeSlot::Unavailable]
    };
    let open_range_now = open_range.read().clone();
    let row_style = flip.style_with(if elevated { "z-index:1" } else { "" });

    rsx! {
        div {
            "data-beui-scheduler-row": "{day.slug()}",
            "data-beui-flip": flip.scope(),
            style: row_style,
            class: "relative flex flex-col gap-3 py-4 sm:flex-row sm:items-start sm:gap-4",
            // toggle + label; actions ride along on mobile
            div { class: "flex items-center justify-between sm:w-36 sm:shrink-0 sm:justify-start sm:pt-1",
                div { class: "flex items-center gap-2.5",
                    Switch {
                        checked: state.enabled,
                        on_checked_change: move |next| set_enabled.call(next),
                        aria_label: "Toggle {label} availability",
                        class: "scale-90",
                    }
                    span { class: "text-sm font-medium text-foreground", "{label}" }
                }
                div { class: "flex items-center gap-1 sm:hidden",
                    DayActions {
                        day,
                        label: label.clone(),
                        reduce,
                        on_add: add_range,
                        on_copy: move |targets: Vec<DayKey>| on_copy.call((day, targets)),
                    }
                }
            }
            // ranges or unavailable
            div { class: "flex min-w-0 flex-1 flex-col gap-2",
                AnimatePresence {
                    initial: false,
                    mode: PresenceMode::PopLayout,
                    for slot in slots.iter() {
                        RangeSlotView {
                            key: "{slot.key()}",
                            slot: slot.clone(),
                            day,
                            options: options.clone(),
                            reduce,
                            week: week.clone(),
                            elevated: slot.elevated(open_range_now.as_deref()),
                            open_panel: open_panel.clone(),
                            on_start: set_start,
                            on_end: set_end,
                            on_remove: remove_range,
                            on_panel_open_change: range_panel_change,
                        }
                    }
                }
            }
            // actions (desktop)
            div { class: "hidden shrink-0 items-center gap-1 pt-0.5 sm:flex",
                DayActions {
                    day,
                    label: label.clone(),
                    reduce,
                    on_add: add_range,
                    on_copy: move |targets: Vec<DayKey>| on_copy.call((day, targets)),
                }
            }
        }
    }
}

// --- one time range ---------------------------------------------------------

#[component]
#[allow(clippy::too_many_arguments)]
fn RangeRow(
    day: DayKey,
    range: TimeRange,
    options: Rc<Vec<TimeOption>>,
    reduce: bool,
    week: WeekAvailability,
    elevated: bool,
    open_panel: Option<String>,
    on_start: EventHandler<(String, String)>,
    on_end: EventHandler<(String, String)>,
    on_remove: EventHandler<String>,
    on_panel_open_change: EventHandler<(String, String, bool)>,
) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(range_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);
    let flip = use_flip(
        week,
        FlipOptions::default()
            .position_only()
            .transition(spring(SPRING_LAYOUT)),
    );
    let base = if elevated { "z-index:1" } else { "" };
    let style = format!("{};{}", gate.style_with(&css), flip.style_with(base));

    let start_id = panel_key(day, &range.id, RangeEdge::Start);
    let end_id = panel_key(day, &range.id, RangeEdge::End);
    let start_list = start_options(&options, &range.end, Some(&range.start));
    let end_list = end_options(&options, &range.start, Some(&range.end));

    let start_open = open_panel.as_deref() == Some(start_id.as_str());
    let end_open = open_panel.as_deref() == Some(end_id.as_str());
    let start_range = range.id.clone();
    let end_range = range.id.clone();
    let end_range_open = range.id.clone();
    let remove_range_open = range.id.clone();
    let remove_range_click = range.id.clone();
    let start_id_for_open = start_id.clone();
    let end_id_for_open = end_id.clone();

    rsx! {
        div {
            inert: gate.inert_attr(),
            style,
            class: "relative flex items-center gap-2",
            div { class: "min-w-0 flex-1 sm:max-w-[132px]",
                TimeSelect {
                    value: range.start.clone(),
                    options: start_list,
                    open: start_open,
                    on_change: move |value: String| on_start.call((start_range.clone(), value)),
                    on_open_change: move |next: bool| {
                        on_panel_open_change.call((end_range_open.clone(), start_id_for_open.clone(), next))
                    },
                }
            }
            span { class: "text-muted-foreground", "–" }
            div { class: "min-w-0 flex-1 sm:max-w-[132px]",
                TimeSelect {
                    value: range.end.clone(),
                    options: end_list,
                    open: end_open,
                    on_change: move |value: String| on_end.call((end_range.clone(), value)),
                    on_open_change: move |next: bool| {
                        on_panel_open_change.call((remove_range_open.clone(), end_id_for_open.clone(), next))
                    },
                }
            }
            IconButton {
                label: "Remove time range",
                title: "Remove",
                reduce,
                on_click: move |_| on_remove.call(remove_range_click.clone()),
                X { class: "h-4 w-4" }
            }
        }
    }
}

/// The source wraps this `motion.span` in the same `AnimatePresence`.
#[component]
fn UnavailableRow(reduce: bool) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(unavailable_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    rsx! {
        span {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: "py-1 text-sm text-muted-foreground sm:py-2",
            "Unavailable"
        }
    }
}

// --- add / copy -------------------------------------------------------------

/// The two icon buttons, in both the mobile and the desktop slot (two instances,
/// as in the source's two render positions).
#[component]
fn DayActions(
    day: DayKey,
    label: String,
    reduce: bool,
    on_add: EventHandler<()>,
    on_copy: EventHandler<Vec<DayKey>>,
) -> Element {
    let add_label = format!("Add time range to {label}");
    let copy_label = format!("Copy {label} hours to other days");

    rsx! {
        IconButton {
            label: add_label,
            title: "Add time",
            reduce,
            on_click: move |_| on_add.call(()),
            Plus { class: "h-4 w-4" }
        }
        CopyMenu {
            from_day: day,
            aria_label: copy_label,
            reduce,
            on_apply: on_copy,
        }
    }
}

/// How much room `copy-menu` wants below its trigger before it flips up.
const COPY_PANEL_ESTIMATE: f64 = 300.0;

/// `CopyMenu` — copy this day's hours to the other days.
///
/// In place rather than portaled (see the module docs): the panel is an
/// `absolute` layer inside the trigger's wrapper, with the same morph clip and
/// scale entrance as `popover-morph`.
#[component]
fn CopyMenu(
    from_day: DayKey,
    aria_label: String,
    reduce: bool,
    on_apply: EventHandler<Vec<DayKey>>,
) -> Element {
    let mut open = use_signal(|| false);
    let mut copied = use_signal(|| false);
    let mut picked = use_signal(BTreeSet::<DayKey>::new);
    let mut flip_up = use_signal(|| false);

    let close = EventHandler::new(move |_: ()| open.set(false));
    let scope = use_dismiss(open, close, DismissOptions::default());

    let others: Vec<DayKey> = DayKey::ALL
        .into_iter()
        .filter(|day| *day != from_day)
        .collect();

    let toggle = EventHandler::new(move |day: DayKey| {
        let mut next = picked.read().clone();
        if !next.remove(&day) {
            next.insert(day);
        }
        picked.set(next);
    });

    let apply = {
        EventHandler::new(move |targets: Vec<DayKey>| {
            if targets.is_empty() {
                return;
            }
            on_apply.call(targets);
            open.set(false);
            picked.write().clear();
            copied.set(true);
            spawn(async move {
                let _ = Time::delay(Duration::from_millis(1200)).await;
                copied.set(false);
            });
        })
    };

    // Decide the side from the trigger's box once the panel opens: the source's
    // portaled panel picks the roomier side, and an in-place panel can too with
    // `top`/`bottom` on the same anchored layer.
    let toggle_open = {
        let selector = format!("[data-beui-copy-trigger=\"{}\"]", from_day.slug());
        EventHandler::new(move |_: ()| {
            if *open.read() {
                open.set(false);
                return;
            }
            open.set(true);
            let selector = selector.clone();
            spawn(async move {
                let script = format!(
                    "const el = document.querySelector({});\n\
                     if (!el) return null;\n\
                     const r = el.getBoundingClientRect();\n\
                     let bottom = r.bottom;\n\
                     let node = el.parentElement;\n\
                     while (node) {{ bottom = Math.max(bottom, node.getBoundingClientRect().bottom); node = node.parentElement; }}\n\
                     return [bottom, window.innerHeight];",
                    platform::js_string(&selector)
                );
                if let Some(values) = platform::json::<Vec<f64>>(&script).await {
                    if values.len() >= 2 {
                        flip_up.set(values[0] + 8.0 + COPY_PANEL_ESTIMATE > values[1]);
                    }
                }
            });
        })
    };

    let panel_layers: Vec<()> = if open() { vec![()] } else { Vec::new() };
    let glyph_key = if copied() { "done" } else { "copy" };
    let panel_key_name = "copy-panel";
    let up = flip_up();
    let picked_now = picked.read().clone();
    let others_for_all = others.clone();

    rsx! {
        span {
            "data-beui-scope": scope.attr(),
            class: "relative inline-flex",
            button {
                r#type: "button",
                "data-beui-copy-trigger": "{from_day.slug()}",
                aria_label: "{aria_label}",
                aria_haspopup: "dialog",
                aria_expanded: if open() { "true" } else { "false" },
                title: "Copy times",
                class: "inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                onclick: move |_| toggle_open.call(()),
                AnimatePresence {
                    initial: false,
                    mode: PresenceMode::PopLayout,
                    for done in [copied()] {
                        CopyGlyph {
                            key: "{glyph_key}",
                            done,
                            reduce,
                        }
                    }
                }
            }
            AnimatePresence {
                initial: false,
                for _ in panel_layers {
                    CopyPanel {
                        key: "{panel_key_name}",
                        other_days: others.clone(),
                        picked: picked_now.clone(),
                        flip_up: up,
                        reduce,
                        on_toggle: toggle,
                        on_apply: apply,
                        on_apply_all: {
                            let all = others_for_all.clone();
                            EventHandler::new(move |_: ()| apply.call(all.clone()))
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn CopyGlyph(done: bool, reduce: bool) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(glyph_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    rsx! {
        span {
            inert: gate.inert_attr(),
            style: gate.style_with(&css),
            class: if done { "text-foreground" } else { "" },
            if done {
                Check { class: "h-4 w-4" }
            } else {
                Copy { class: "h-4 w-4" }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn CopyPanel(
    other_days: Vec<DayKey>,
    picked: BTreeSet<DayKey>,
    flip_up: bool,
    reduce: bool,
    on_toggle: EventHandler<DayKey>,
    on_apply: EventHandler<Vec<DayKey>>,
    on_apply_all: EventHandler<()>,
) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(copy_panel_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    // The same 92% → 0 clip the portaled surface uses, from the trigger's corner.
    let mut clip = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let present = gate.is_present();
    let mut presence = use_signal(|| present);
    if *presence.peek() != present {
        presence.set(present);
    }
    use_effect(move || {
        let target = if presence() { 1.0f32 } else { 0.0f32 };
        let _ = clip.animate_to(target, tween_ease_out(320));
    });
    let inset = 92.0 * (1.0 - f64::from(clip.get_value()));

    let anchor = if flip_up {
        "bottom:calc(100% + 8px)"
    } else {
        "top:calc(100% + 8px)"
    };
    let inner_style = format!(
        "clip-path:inset({} 0% {} 0% round 16px);transform-origin:{} right",
        if flip_up {
            format!("{inset}%")
        } else {
            "0%".to_string()
        },
        if flip_up {
            "0%".to_string()
        } else {
            format!("{inset}%")
        },
        if flip_up { "bottom" } else { "top" },
    );
    let outer_style = format!(
        "position:absolute;right:0;{anchor};width:13rem;z-index:30;{}",
        gate.style_with(&css)
    );

    let selected: Vec<DayKey> = picked.iter().copied().collect();

    rsx! {
        div {
            inert: gate.inert_attr(),
            style: outer_style,
            div {
                "data-beui-copy-panel": "",
                role: "dialog",
                "aria-label": "Copy times to",
                style: inner_style,
                class: "overflow-hidden rounded-xl border border-border bg-background p-2 shadow-xl",
                p { class: "px-2 py-1.5 text-xs font-medium text-muted-foreground", "Copy times to" }
                div { class: "flex flex-col",
                    for day in other_days.iter() {
                        Checkbox {
                            key: "{day.slug()}",
                            checked: picked.contains(day),
                            on_checked_change: {
                                let day = *day;
                                move |_| on_toggle.call(day)
                            },
                            label: day.label(),
                            class: "w-full flex-row-reverse justify-between rounded-lg px-2 py-1.5 transition-colors hover:bg-muted [&_button]:size-4 [&_button]:rounded-[5px] [&_button]:border [&_button[data-state=unchecked]]:border-border-strong",
                        }
                    }
                }
                div { class: "mt-1 flex items-center gap-2 border-t border-border px-1 pt-2",
                    button {
                        r#type: "button",
                        class: "flex-1 rounded-lg px-2 py-1.5 text-xs font-medium text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:bg-muted",
                        onclick: move |_| on_apply_all.call(()),
                        "Every day"
                    }
                    button {
                        r#type: "button",
                        "data-beui-copy-apply": "",
                        disabled: selected.is_empty(),
                        class: "flex-1 rounded-lg bg-primary px-2 py-1.5 text-xs font-semibold text-primary-foreground outline-none transition-opacity hover:opacity-90 focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40",
                        onclick: {
                            let selected = selected.clone();
                            move |_| on_apply.call(selected.clone())
                        },
                        "Apply"
                    }
                }
            }
        }
    }
}

// --- time field -------------------------------------------------------------

/// `TimeSelect` — the library `Select` with a capped option list.
#[component]
fn TimeSelect(
    value: String,
    options: Vec<TimeOption>,
    open: bool,
    on_change: EventHandler<String>,
    on_open_change: EventHandler<bool>,
) -> Element {
    rsx! {
        Select {
            value: Some(value),
            open,
            on_value_change: move |next| on_change.call(next),
            on_open_change: move |next| on_open_change.call(next),
            class: "w-full",
            SelectTrigger { class: "tabular-nums",
                SelectValue { class: "whitespace-nowrap" }
            }
            SelectContent {
                div { class: "max-h-56 overflow-y-auto overscroll-contain",
                    for option in options.iter() {
                        SelectItem {
                            key: "{option.value}",
                            value: option.value.clone(),
                            label: option.label.clone(),
                            class: "tabular-nums",
                            "{option.label}"
                        }
                    }
                }
            }
        }
    }
}

// --- icon button ------------------------------------------------------------

/// `IconButton` — the source's `motion.button` with `whileTap` scale 0.86.
#[component]
fn IconButton(
    label: String,
    #[props(default = None)] title: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] expanded: Option<bool>,
    reduce: bool,
    #[props(default = None)] class: Option<String>,
    on_click: EventHandler<()>,
    children: Element,
) -> Element {
    let mut scale = use_number_motion(1.0);
    let release = move |_: Event<PointerData>| {
        if !reduce {
            animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
        }
    };

    rsx! {
        button {
            r#type: "button",
            aria_label: "{label}",
            aria_expanded: expanded.map(|value| if value { "true" } else { "false" }),
            title,
            disabled,
            style: scale_style(number(&scale)),
            class: cn!(
                "inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40",
                class,
            ),
            onpointerdown: move |_| {
                if !reduce && !disabled {
                    animate_number(&mut scale, 0.86, spring(SPRING_PRESS));
                }
            },
            onpointerup: release,
            onpointercancel: release,
            onclick: move |_| on_click.call(()),
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_grid_options_at_the_requested_step() {
        let options = build_options(30);
        assert_eq!(options.len(), 48);
        assert_eq!(options[0].value, "00:00");
        assert_eq!(options[0].label, "12:00 AM");
        assert_eq!(options[1].value, "00:30");
        assert_eq!(options[2].label, "1:00 AM");
        assert_eq!(options[34].value, "17:00");
        assert_eq!(options[34].label, "5:00 PM");
        assert_eq!(options[47].value, "23:30");
        // A step that would spin forever in JS still terminates here.
        assert_eq!(build_options(0).len(), 24 * 60);
    }

    #[test]
    fn labels_and_values_round_trip() {
        assert_eq!(label12("00:00"), "12:00 AM");
        assert_eq!(label12("12:00"), "12:00 PM");
        assert_eq!(label12("23:45"), "11:45 PM");
        assert_eq!(to_value(0), "00:00");
        assert_eq!(to_value(9 * 60), "09:00");
        assert_eq!(to_value(24 * 60), "23:59");
        assert_eq!(to_minutes("09:30"), 570);
    }

    #[test]
    fn a_valid_pair_is_left_alone() {
        let options = build_options(30);
        assert_eq!(
            clamp_range("09:00", "17:00", &options, RangeEdge::Start),
            ("09:00".to_string(), "17:00".to_string())
        );
        // An off-grid persisted end is not rewritten.
        let hourly = build_options(60);
        assert_eq!(
            clamp_range("09:00", "17:00", &hourly, RangeEdge::End),
            ("09:00".to_string(), "17:00".to_string())
        );
    }

    #[test]
    fn moving_the_end_past_the_start_pulls_the_start_back() {
        let options = build_options(30);
        // End 09:00 with start 17:00: the end stays, the start moves onto 08:30.
        assert_eq!(
            clamp_range("17:00", "09:00", &options, RangeEdge::End),
            ("08:30".to_string(), "09:00".to_string())
        );
        // Midnight cannot end a positive range: fall back to the first pair.
        assert_eq!(
            clamp_range("17:00", "00:00", &options, RangeEdge::End),
            ("00:00".to_string(), "00:30".to_string())
        );
    }

    #[test]
    fn moving_the_start_past_the_end_pushes_the_end_out() {
        let options = build_options(30);
        assert_eq!(
            clamp_range("18:00", "17:00", &options, RangeEdge::Start),
            ("18:00".to_string(), "18:30".to_string())
        );
        // The last slot cannot start a positive range: use the final pair.
        assert_eq!(
            clamp_range("23:30", "23:00", &options, RangeEdge::Start),
            ("23:00".to_string(), "23:30".to_string())
        );
    }

    #[test]
    fn option_lists_keep_the_current_value_and_expose_a_recovery() {
        let options = build_options(30);
        let starts = start_options(&options, "09:00", Some("09:00"));
        assert!(starts
            .iter()
            .all(|option| option.value == "09:00" || to_minutes(&option.value) < 540));
        assert!(
            starts.iter().any(|option| option.value == "09:00"),
            "off-grid current kept"
        );
        // Midnight end leaves no earlier start: midnight itself is exposed.
        let recovery = start_options(&options, "00:00", None);
        assert_eq!(recovery.len(), 1);
        assert_eq!(recovery[0].value, "00:00");

        let ends = end_options(&options, "09:00", Some("09:00"));
        assert!(ends
            .iter()
            .all(|option| option.value == "09:00" || to_minutes(&option.value) > 540));
        let recovery = end_options(&options, "23:30", None);
        assert_eq!(recovery.len(), 1);
        assert_eq!(recovery[0].value, "23:30");
    }

    #[test]
    fn the_default_week_is_mon_fri_nine_to_five() {
        let week = default_week();
        for day in DayKey::ALL {
            let state = week.get(day);
            match day {
                DayKey::Sat | DayKey::Sun => assert!(!state.enabled),
                _ => {
                    assert!(state.enabled);
                    assert_eq!(state.ranges.len(), 1);
                    assert_eq!(state.ranges[0].start, "09:00");
                    assert_eq!(state.ranges[0].end, "17:00");
                }
            }
        }
        assert_eq!(week.get(DayKey::Mon).ranges[0].id, "mon-0");
    }

    #[test]
    fn live_panels_track_the_enabled_ranges() {
        let week = default_week();
        let panels = live_panels(&week);
        assert_eq!(panels.len(), 5 * 2);
        assert!(panels.contains("mon:mon-0:start"));
        assert!(!panels.contains("sat:sat-0:start"));
    }

    #[test]
    fn patching_a_range_clamps_it_and_leaves_the_others() {
        let options = build_options(30);
        let state = DayAvailability {
            enabled: true,
            ranges: vec![
                TimeRange::new("a", "09:00", "12:00"),
                TimeRange::new("b", "13:00", "17:00"),
            ],
        };
        let next = with_range_patch(&state, "b", &RangePatch::Start("18:00".into()), &options);
        assert_eq!(next.ranges[0].start, "09:00");
        assert_eq!(next.ranges[1].start, "18:00");
        assert_eq!(next.ranges[1].end, "18:30");
    }
}

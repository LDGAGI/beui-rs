//! `charts/status-bar/` — the uptime history bar with an inspecting cursor.
//!
//! The model (`rows`, active period, tooltip id, hover capability) is provided
//! through a context, exactly like the source's `StatusBarContext`; `StatusBar`
//! composes `StatusBarPlot` / `StatusBarTooltip` / `StatusBarLegend` when no
//! children are given, and always renders the screen-reader table.
//!
//! Differences from the source: the inspection cursor reads the hidden
//! range input's `input` event for both pointer and keyboard rather than
//! hit-testing pointer x against equal-width bars, and the per-bar entrance
//! stagger is a CSS animation.

use beui::prelude::*;

use crate::common::use_unique_id;

const IN_KEYFRAMES: &str = "@keyframes beui-status-in{from{opacity:0;transform:translateY(4px) scaleY(0.9)}to{opacity:1;transform:translateY(0) scaleY(1)}}";

/// `StatusBarDatum`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBarDatum {
    pub id: String,
    pub label: String,
    pub status: String,
    pub description: Option<String>,
}

impl StatusBarDatum {
    pub fn new(id: impl Into<String>, label: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status: status.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// `StatusBarStatus`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBarStatus {
    pub id: String,
    pub label: String,
    /// Any CSS color, including theme variables.
    pub color: String,
}

impl StatusBarStatus {
    pub fn new(id: impl Into<String>, label: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color: color.into(),
        }
    }
}

/// `STATUS_BAR_STATUSES`.
pub fn status_bar_statuses() -> Vec<StatusBarStatus> {
    vec![
        StatusBarStatus::new("operational", "Operational", "#059669"),
        StatusBarStatus::new("degraded", "Degraded", "#d97706"),
        StatusBarStatus::new("outage", "Outage", "#e11d48"),
        StatusBarStatus::new("maintenance", "Maintenance", "#2563eb"),
        StatusBarStatus::new("unknown", "No data", "var(--muted-foreground)"),
    ]
}

/// A resolved period + status pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBarRow {
    pub datum: StatusBarDatum,
    pub status: StatusBarStatus,
}

#[derive(Clone, Copy)]
struct StatusBarContext {
    rows: Signal<Vec<StatusBarRow>>,
    statuses: Signal<Vec<StatusBarStatus>>,
    label: Signal<String>,
    active_id: Signal<Option<String>>,
    tooltip_open: Signal<bool>,
    tooltip_id: Signal<String>,
    on_active_change: Option<EventHandler<Option<String>>>,
    controlled: bool,
}

fn build_rows(data: &[StatusBarDatum], statuses: &[StatusBarStatus]) -> Vec<StatusBarRow> {
    data.iter()
        .filter_map(|datum| {
            statuses
                .iter()
                .find(|status| status.id == datum.status)
                .map(|status| StatusBarRow {
                    datum: datum.clone(),
                    status: status.clone(),
                })
        })
        .collect()
}

/// `StatusBar`.
#[component]
pub fn StatusBar(
    data: Vec<StatusBarDatum>,
    #[props(default = status_bar_statuses())] statuses: Vec<StatusBarStatus>,
    #[props(default = "Status history".to_string())] label: String,
    #[props(default)] active_id: Option<String>,
    #[props(default)] default_active_id: Option<String>,
    #[props(default)] on_active_change: Option<EventHandler<Option<String>>>,
    #[props(default)] children: Option<Element>,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let rows_signal = use_signal(|| build_rows(&data, &statuses));
    let statuses_signal = use_signal(|| statuses.clone());
    let label_signal = use_signal(|| label.clone());
    let mut internal_active = use_signal(|| default_active_id.clone());
    let tooltip_open = use_signal(|| false);
    let tooltip_id_value = use_unique_id("status-bar-tip");
    let tooltip_id = use_signal(|| tooltip_id_value.clone());
    let controlled = active_id.is_some();

    use_effect(use_reactive((&active_id,), move |(active_id,)| {
        if let Some(active_id) = active_id {
            internal_active.set(Some(active_id));
        }
    }));

    let context = StatusBarContext {
        rows: rows_signal,
        statuses: statuses_signal,
        label: label_signal,
        active_id: internal_active,
        tooltip_open,
        tooltip_id,
        on_active_change,
        controlled,
    };
    use_context_provider(|| context);

    let rows = rows_signal();

    rsx! {
        style { dangerous_inner_html: "{IN_KEYFRAMES}" }
        section {
            "aria-label": "{label}",
            "data-slot": "status-bar",
            class: cn!("min-w-0 w-full space-y-3", class),
            ..attributes,
            if let Some(children) = children {
                {children}
            } else {
                div { class: "relative",
                    StatusBarPlot {}
                    StatusBarTooltip {}
                }
                StatusBarLegend {}
            }
            div { class: "sr-only",
                table {
                    caption { "{label} data" }
                    thead {
                        tr {
                            th { scope: "col", "Period" }
                            th { scope: "col", "Status" }
                            th { scope: "col", "Details" }
                        }
                    }
                    tbody {
                        for row in rows.iter() {
                            tr { key: "{row.datum.id}",
                                th { scope: "row", "{row.datum.label}" }
                                td { "{row.status.label}" }
                                td { "{row.datum.description.clone().unwrap_or_default()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `StatusBarPlot`.
#[component]
pub fn StatusBarPlot(
    #[props(default)] class: String,
    #[props(default)] bar_class_name: String,
    #[props(default = true)] show_labels: bool,
) -> Element {
    let context = use_context::<StatusBarContext>();
    let reduce = use_reduced_motion()();
    let rows = (context.rows)();
    if rows.is_empty() {
        return rsx! {
            p { class: cn!("py-4 text-sm text-muted-foreground", class), "No status data" }
        };
    }
    let label = (context.label)();
    let active_id = current_active(&context);
    let active_index = rows
        .iter()
        .position(|row| Some(&row.datum.id) == active_id.as_ref())
        .map(|index| index as i64)
        .unwrap_or(-1);
    let index = active_index.max(0) as usize;
    let row = rows.get(index).cloned();
    let rows_len = rows.len();
    let tooltip_open = (context.tooltip_open)();
    let bar_class = cn!(
        "relative block h-8 min-w-0 flex-1 rounded-full",
        bar_class_name
    );
    let valuetext = row
        .as_ref()
        .map(|row| {
            let mut parts = vec![row.datum.label.clone(), row.status.label.clone()];
            if let Some(description) = &row.datum.description {
                parts.push(description.clone());
            }
            parts.join(". ")
        })
        .unwrap_or_default();

    rsx! {
        div { "data-slot": "status-bar-plot", class: cn!("min-w-0 space-y-1.5", class),
            div { class: "relative h-11 rounded-md has-focus-visible:outline-2 has-focus-visible:outline-offset-4 has-focus-visible:outline-ring",
                div { "aria-hidden": "true", class: "flex h-full items-center gap-0.5 sm:gap-1",
                    for (index, row) in rows.iter().enumerate() {
                        {
                            let active = tooltip_open && active_index >= 0 && active_index as usize == index;
                            let delay = if reduce { 0.0 } else { index as f64 / (rows_len.saturating_sub(1).max(1)) as f64 * 0.18 };
                            let bar_style = format!(
                                "animation:beui-status-in 240ms cubic-bezier(0.16,1,0.3,1) both {delay}s;"
                            );
                            let ring_opacity = if active { 1.0 } else { 0.0 };
                            rsx! {
                                span {
                                    key: "{row.datum.id}",
                                    "data-status-period": "{row.datum.id}",
                                    "data-status": "{row.status.id}",
                                    class: "{bar_class}",
                                    style: "{bar_style}",
                                    span {
                                        class: "pointer-events-none absolute inset-0 rounded-[inherit]",
                                        style: "background-color:{row.status.color};transition:background-color 160ms cubic-bezier(0.16,1,0.3,1);",
                                    }
                                    span {
                                        class: "pointer-events-none absolute -inset-0.5 rounded-[inherit] ring-1 ring-foreground/60",
                                        style: "opacity:{ring_opacity};transition:opacity 100ms cubic-bezier(0.16,1,0.3,1);",
                                    }
                                }
                            }
                        }
                    }
                }
                input {
                    r#type: "range",
                    "aria-label": "{label}: inspect period",
                    min: "0",
                    max: "{rows_len - 1}",
                    step: "1",
                    value: "{index}",
                    "aria-valuetext": "{valuetext}",
                    "aria-describedby": if tooltip_open && active_index >= 0 { Some((context.tooltip_id)()) } else { None },
                    class: "absolute inset-0 m-0 h-full w-full cursor-crosshair opacity-0",
                    onfocus: {
                        let id = row.as_ref().map(|row| row.datum.id.clone());
                        let mut active_id_signal = context.active_id;
                        let mut open = context.tooltip_open;
                        let set = context.on_active_change;
                        let controlled = context.controlled;
                        move |_| {
                            if let Some(id) = id.clone() {
                                if !controlled { active_id_signal.set(Some(id.clone())); }
                                if let Some(handler) = set { handler.call(Some(id.clone())); }
                            }
                            open.set(true);
                        }
                    },
                    onblur: {
                        let mut open = context.tooltip_open;
                        move |_| open.set(false)
                    },
                    oninput: {
                        let rows = rows.clone();
                        let mut active_id_signal = context.active_id;
                        let mut open = context.tooltip_open;
                        let set = context.on_active_change;
                        let controlled = context.controlled;
                        move |event| {
                            if let Ok(index) = event.value().parse::<usize>() {
                                if let Some(row) = rows.get(index) {
                                    let id = row.datum.id.clone();
                                    if !controlled { active_id_signal.set(Some(id.clone())); }
                                    if let Some(handler) = set { handler.call(Some(id)); }
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
                                Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End
                            ) {
                                open.set(true);
                            }
                        }
                    },
                }
            }
            if show_labels {
                div { class: "flex justify-between gap-4 text-xs text-muted-foreground",
                    span { "{rows[0].datum.label}" }
                    span { "{rows[rows_len - 1].datum.label}" }
                }
            }
        }
    }
}

/// `StatusBarTooltip`.
#[component]
pub fn StatusBarTooltip(
    #[props(default)] children: Option<Element>,
    #[props(default)] class: String,
) -> Element {
    let context = use_context::<StatusBarContext>();
    let rows = (context.rows)();
    let active_id = current_active(&context);
    let active = rows
        .iter()
        .position(|row| Some(&row.datum.id) == active_id.as_ref())
        .and_then(|index| rows.get(index).cloned());
    if !(context.tooltip_open)() || active.is_none() {
        return rsx! {};
    }
    let row = active.expect("checked above");
    let index = rows
        .iter()
        .position(|candidate| candidate.datum.id == row.datum.id)
        .unwrap_or(0);
    let percent = (index as f64 + 0.5) / (rows.len() as f64) * 100.0;

    rsx! {
        div {
            id: "{(context.tooltip_id)()}",
            role: "tooltip",
            class: cn!("pointer-events-none absolute bottom-full z-10 mb-2 -translate-x-1/2 rounded-xl border border-border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md", class),
            style: "left:{percent}%",
            if let Some(children) = children {
                {children}
            } else {
                span { class: "flex flex-col gap-1",
                    span { class: "text-muted-foreground", "{row.datum.label}" }
                    span { class: "flex items-center gap-2 font-medium",
                        span {
                            "aria-hidden": "true",
                            class: "size-1.5 shrink-0 rounded-full",
                            style: "background-color:{row.status.color};",
                        }
                        "{row.status.label}"
                    }
                    if let Some(description) = row.datum.description.clone() {
                        span { class: "text-muted-foreground", "{description}" }
                    }
                }
            }
        }
    }
}

/// `StatusBarLegend`.
#[component]
pub fn StatusBarLegend(
    #[props(default)] show_counts: bool,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let context = use_context::<StatusBarContext>();
    let statuses = (context.statuses)();
    let rows = (context.rows)();
    rsx! {
        ul {
            "aria-label": "Status legend",
            "data-slot": "status-bar-legend",
            class: cn!("flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-muted-foreground", class),
            ..attributes,
            for status in statuses.iter() {
                li { key: "{status.id}", class: "flex items-center gap-1.5",
                    span {
                        "aria-hidden": "true",
                        class: "h-3 w-1.5 rounded-full",
                        style: "background-color:{status.color};",
                    }
                    span { "{status.label}" }
                    if show_counts {
                        span { class: "tabular-nums",
                            "({rows.iter().filter(|row| row.status.id == status.id).count()})"
                        }
                    }
                }
            }
        }
    }
}

fn current_active(context: &StatusBarContext) -> Option<String> {
    (context.active_id)()
}

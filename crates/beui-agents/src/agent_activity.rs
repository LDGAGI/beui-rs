//! `agents/agent-activity/` — the streaming activity log.
//!
//! The source types are a discriminated union; here they are an enum of structs
//! with the same fields. `AgentActivity` renders the working shimmer row or the
//! completed summary disclosure over the same `ActivityRow` shapes, measures its
//! content through the page (`ResizeObserver`, see [`crate::timers`]) and glides
//! a capped viewport with the same `streamOffset` maths.
//!
//! Not ported: the `renderWorkingStatus` / `renderCompletedStatus` render props
//! (Dioxus components take elements, not render functions) and the per-item
//! enter/exit springs. Both deliberate.

use beui::prelude::*;

use crate::common::{scroll_to_top, use_unique_id, Icon};
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;
use crate::loading_states::ThinkingShimmer;
use crate::timers::use_observed_height;

const PULSE_KEYFRAMES: &str =
    "@keyframes beui-activity-pulse{0%,100%{opacity:0.35}50%{opacity:0.8}}";

/// `AgentActivityStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AgentActivityStatus {
    #[default]
    Working,
    Complete,
}

/// `AgentStepStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AgentStepStatus {
    Pending,
    Active,
    #[default]
    Complete,
}

/// `AgentActivityContentType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AgentActivityContentType {
    Step,
    Text,
    Search,
    Tool,
    Trace,
    #[default]
    Mixed,
}

/// `AgentActivityStep`.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentActivityStep {
    pub id: String,
    pub label: Element,
    pub status: AgentStepStatus,
    pub meta: Option<Element>,
}

/// `AgentActivityText`.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentActivityText {
    pub id: String,
    pub content: Element,
}

/// `AgentSearchResult`.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentSearchResult {
    pub id: String,
    pub title: Element,
    pub domain: Option<Element>,
    pub url: Option<String>,
    pub icon: Option<Element>,
}

/// `AgentActivitySearch`.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentActivitySearch {
    pub id: String,
    pub query: Element,
    pub results: Vec<AgentSearchResult>,
    pub more_count: Option<usize>,
}

/// `AgentActivityTool`.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentActivityTool {
    pub id: String,
    pub action: String,
    pub target: Element,
    pub additions: Option<i64>,
    pub deletions: Option<i64>,
}

/// `AgentActivityTrace`.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentActivityTrace {
    pub id: String,
    pub kind: String,
    pub label: Element,
    pub detail: Option<Element>,
    pub icon: Option<Element>,
}

/// `AgentActivityItem`.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentActivityItem {
    Step(AgentActivityStep),
    Text(AgentActivityText),
    Search(AgentActivitySearch),
    Tool(AgentActivityTool),
    Trace(AgentActivityTrace),
}

impl AgentActivityItem {
    pub fn id(&self) -> &str {
        match self {
            Self::Step(item) => &item.id,
            Self::Text(item) => &item.id,
            Self::Search(item) => &item.id,
            Self::Tool(item) => &item.id,
            Self::Trace(item) => &item.id,
        }
    }

    pub fn content_type(&self) -> AgentActivityContentType {
        match self {
            Self::Step(_) => AgentActivityContentType::Step,
            Self::Text(_) => AgentActivityContentType::Text,
            Self::Search(_) => AgentActivityContentType::Search,
            Self::Tool(_) => AgentActivityContentType::Tool,
            Self::Trace(_) => AgentActivityContentType::Trace,
        }
    }
}

fn get_content_type(items: &[AgentActivityItem]) -> AgentActivityContentType {
    match items.first() {
        Some(first) => {
            let first_type = first.content_type();
            if items.iter().all(|item| item.content_type() == first_type) {
                first_type
            } else {
                AgentActivityContentType::Mixed
            }
        }
        None => AgentActivityContentType::Mixed,
    }
}

fn default_active_label(content_type: AgentActivityContentType) -> &'static str {
    match content_type {
        AgentActivityContentType::Search => "Searching the web…",
        AgentActivityContentType::Tool => "Running tools…",
        AgentActivityContentType::Trace => "Working through the run…",
        AgentActivityContentType::Mixed => "Working through it…",
        AgentActivityContentType::Step | AgentActivityContentType::Text => "Thinking…",
    }
}

fn format_duration(duration: f64) -> String {
    let seconds = duration.max(0.0).round() as i64;
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    if remainder == 0 {
        format!("{minutes}m")
    } else {
        format!("{minutes}m {remainder}s")
    }
}

fn summary_text(
    content_type: AgentActivityContentType,
    items: &[AgentActivityItem],
    duration: f64,
) -> String {
    match content_type {
        AgentActivityContentType::Step | AgentActivityContentType::Text => {
            format!("Thought for {}", format_duration(duration))
        }
        AgentActivityContentType::Search => "Searched the web".to_string(),
        AgentActivityContentType::Tool => {
            let count = items.len();
            format!("Ran {count} {}", if count == 1 { "tool" } else { "tools" })
        }
        AgentActivityContentType::Trace => {
            let messages = items
                .iter()
                .filter(|item| {
                    matches!(item, AgentActivityItem::Trace(trace) if trace.kind == "thinking" || trace.kind == "message")
                })
                .count();
            let tools = items.len() - messages;
            format!(
                "{tools} {}, {messages} {}",
                if tools == 1 {
                    "tool call"
                } else {
                    "tool calls"
                },
                if messages == 1 { "message" } else { "messages" },
            )
        }
        AgentActivityContentType::Mixed => {
            let count = items.len();
            format!(
                "Completed {count} {}",
                if count == 1 { "step" } else { "steps" }
            )
        }
    }
}

fn action_icon(action: &str) -> IconName {
    match action {
        "read" => IconName::FileText,
        "edit" | "write" => IconName::PencilLine,
        "run" => IconName::SquareTerminal,
        _ => IconName::Wrench,
    }
}

fn trace_icon(kind: &str) -> IconName {
    match kind {
        "thinking" => IconName::Sparkles,
        "message" => IconName::MessageSquare,
        "write" => IconName::PencilLine,
        "run" => IconName::SquareTerminal,
        "read" => IconName::ImageIcon,
        _ => IconName::Wrench,
    }
}

fn step_row(item: &AgentActivityStep) -> Element {
    let state = item.status;
    rsx! {
        div { class: "flex min-h-7 items-start gap-2.5 rounded-md px-1.5 py-1",
            span { "aria-hidden": "true", class: "mt-0.5 grid size-4 shrink-0 place-items-center text-muted-foreground/70",
                if state == AgentStepStatus::Complete {
                    Icon { name: IconName::Check, class: "size-4", "stroke-width": "1.8" }
                } else if state == AgentStepStatus::Active {
                    span { class: "relative grid size-3 place-items-center",
                        span {
                            class: "absolute inset-0 rounded-full bg-foreground/10",
                            style: "animation:beui-activity-pulse 1.5s linear infinite;",
                        }
                        span { class: "size-1.5 rounded-full bg-foreground/60" }
                    }
                } else {
                    Icon { name: IconName::Circle, class: "size-3", "stroke-width": "1.5" }
                }
            }
            span {
                class: cn!(
                    "min-w-0 flex-1 leading-5",
                    if state == AgentStepStatus::Pending { "text-muted-foreground/55" } else { "text-foreground/90" },
                ),
                {item.label.clone()}
            }
            if let Some(meta) = item.meta.clone() {
                span { class: "shrink-0 leading-5 text-muted-foreground/55", {meta} }
            }
        }
    }
}

fn text_row(item: &AgentActivityText) -> Element {
    rsx! {
        div { class: "rounded-md px-1.5 py-1 leading-5 text-muted-foreground", {item.content.clone()} }
    }
}

fn search_result_row(result: &AgentSearchResult) -> Element {
    let class =
        "flex min-h-7 items-center gap-2 rounded-md px-1.5 py-1 text-left outline-none transition-colors";
    let content = rsx! {
        span { "aria-hidden": "true", class: "grid size-5 shrink-0 place-items-center text-muted-foreground",
            if let Some(icon) = result.icon.clone() {
                {icon}
            } else {
                Icon { name: IconName::Globe2, class: "size-3", "stroke-width": "2" }
            }
        }
        span { class: "min-w-0 truncate font-medium text-foreground/90", {result.title.clone()} }
        if let Some(domain) = result.domain.clone() {
            span { class: "min-w-0 truncate text-muted-foreground/55", {domain} }
        }
    };
    match result.url.clone() {
        Some(url) => rsx! {
            a { href: "{url}", class: "{class} focus-visible:ring-2 focus-visible:ring-ring", {content} }
        },
        None => rsx! {
            div { class: "{class}", {content} }
        },
    }
}

fn search_row(item: &AgentActivitySearch) -> Element {
    rsx! {
        div { class: "space-y-0.5",
            div { class: "flex min-h-7 items-center gap-2.5 rounded-md px-1.5 py-1 text-muted-foreground",
                Icon { name: IconName::Search, class: "size-4 shrink-0", "stroke-width": "1.7", "aria-hidden": "true" }
                span { class: "min-w-0 truncate", {item.query.clone()} }
            }
            if !item.results.is_empty() {
                div { class: "space-y-0.5 pl-4",
                    for result in item.results.iter() {
                        {search_result_row(result)}
                    }
                }
            }
            if let Some(more_count) = item.more_count {
                div { class: "px-1.5 py-1 pl-8 text-muted-foreground/55", "+{more_count} more" }
            }
        }
    }
}

fn tool_row(item: &AgentActivityTool) -> Element {
    let mut chars = item.action.chars();
    let action = match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    };
    let additions = item.additions;
    let deletions = item.deletions;
    rsx! {
        div { class: "flex min-h-8 min-w-0 items-center gap-2.5 rounded-md px-1.5 py-0.5 leading-5",
            span { "aria-hidden": "true", class: "grid size-4 shrink-0 place-items-center text-muted-foreground/70",
                Icon { name: action_icon(&item.action), class: "size-4" }
            }
            span { class: "shrink-0 font-medium text-foreground/90", "{action}" }
            span { class: "min-w-0 flex-1 truncate rounded-lg bg-muted/80 px-2.5 py-1 font-mono text-xs text-muted-foreground/70",
                {item.target.clone()}
            }
            if additions.is_some() || deletions.is_some() {
                span { class: "flex shrink-0 items-center gap-2 font-mono tabular-nums",
                    if let Some(additions) = additions {
                        span { class: "text-emerald-500", "+{additions}" }
                    }
                    if let Some(deletions) = deletions {
                        span { class: "text-rose-500", "−{deletions}" }
                    }
                }
            }
        }
    }
}

fn trace_row(item: &AgentActivityTrace) -> Element {
    rsx! {
        div { class: "grid min-h-8 grid-cols-[1rem_auto_minmax(0,1fr)] items-center gap-2.5 rounded-md px-1.5 py-0.5",
            span { "aria-hidden": "true", class: "grid size-4 place-items-center text-muted-foreground/70",
                if let Some(icon) = item.icon.clone() {
                    {icon}
                } else {
                    Icon { name: trace_icon(&item.kind), class: "size-4" }
                }
            }
            span { class: "font-medium text-foreground/90", {item.label.clone()} }
            if let Some(detail) = item.detail.clone() {
                span { class: "min-w-0 truncate rounded-lg bg-muted/80 px-2.5 py-1 font-mono text-xs text-muted-foreground/70", {detail} }
            } else {
                span {}
            }
        }
    }
}

/// `ActivityRow`.
#[component]
pub fn ActivityRow(item: AgentActivityItem) -> Element {
    match &item {
        AgentActivityItem::Text(item) => text_row(item),
        AgentActivityItem::Search(item) => search_row(item),
        AgentActivityItem::Tool(item) => tool_row(item),
        AgentActivityItem::Trace(item) => trace_row(item),
        AgentActivityItem::Step(item) => step_row(item),
    }
}

/// `AgentActivity`.
#[component]
pub fn AgentActivity(
    items: Vec<AgentActivityItem>,
    #[props(default)] content_type: Option<AgentActivityContentType>,
    #[props(default)] status: AgentActivityStatus,
    #[props(default)] duration: f64,
    #[props(default)] open: Option<bool>,
    #[props(default)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = true)] collapse_on_complete: bool,
    #[props(default)] active_label: Option<Element>,
    #[props(default)] summary: Option<Element>,
    #[props(default = 208)] max_height: i32,
    #[props(default)] class: String,
    #[props(default)] content_class_name: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("agent-activity");
    let trigger_id = format!("{base}-trigger");
    let content_id = format!("{base}-content");
    let content_selector = format!("[data-beui-activity-content=\"{base}\"]");
    let viewport_selector = format!("[data-beui-activity-viewport=\"{base}\"]");
    let mut internal_open = use_signal(|| default_open);
    let current_open = open.unwrap_or(internal_open());
    let working = status == AgentActivityStatus::Working;
    let expanded = working || current_open;
    let resolved_content_type = if items.is_empty() {
        content_type.unwrap_or(AgentActivityContentType::Mixed)
    } else {
        get_content_type(&items)
    };
    let content_height = use_observed_height(content_selector);

    let mut set_open = move |next: bool| {
        if open.is_none() {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };

    // Collapse when the run completes (source effect; `Cell` for previous, as in
    // `todo_list.rs`).
    let previous_status = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(status)));
    use_effect(use_reactive((&status,), move |(status,)| {
        if previous_status.get() == AgentActivityStatus::Working
            && status == AgentActivityStatus::Complete
        {
            let next = !collapse_on_complete;
            if open.is_none() {
                internal_open.set(next);
            }
            if let Some(handler) = on_open_change {
                handler.call(next);
            }
        }
        previous_status.set(status);
    }));

    let capped_height = content_height().min(max_height.max(0) as f64);
    let viewport_height = if working {
        max_height.max(0) as f64
    } else {
        capped_height
    };
    let capped = content_height() > max_height.max(0) as f64;
    let stream_offset = if working {
        (viewport_height - content_height()).min(0.0)
    } else {
        0.0
    };
    let mask = if capped {
        if working {
            "linear-gradient(to bottom, transparent, black 12px)"
        } else {
            "linear-gradient(to bottom, transparent, black 12px, black calc(100% - 12px), transparent)"
        }
    } else {
        ""
    };

    let live_label =
        active_label.unwrap_or_else(|| rsx! { "{default_active_label(resolved_content_type)}" });
    let completed_summary = summary
        .unwrap_or_else(|| rsx! { "{summary_text(resolved_content_type, &items, duration)}" });
    let rotate = if expanded { 180 } else { 0 };
    let chevron_duration = if reduce { 0 } else { 200 };
    let scroll_target = viewport_selector.clone();
    let mask_style = if mask.is_empty() {
        String::new()
    } else {
        format!("mask-image:{mask};-webkit-mask-image:{mask};")
    };

    rsx! {
        style { dangerous_inner_html: "{PULSE_KEYFRAMES}" }
        div {
            "data-state": if working { "working" } else if expanded { "open" } else { "closed" },
            "data-content": match resolved_content_type {
                AgentActivityContentType::Step => "step",
                AgentActivityContentType::Text => "text",
                AgentActivityContentType::Search => "search",
                AgentActivityContentType::Tool => "tool",
                AgentActivityContentType::Trace => "trace",
                AgentActivityContentType::Mixed => "mixed",
            },
            "aria-busy": if working { "true" } else { "false" },
            class: cn!("w-full text-sm", class),
            if working {
                div { id: "{trigger_id}", role: "status", class: "flex h-7 min-w-0 items-center text-muted-foreground",
                    ThinkingShimmer { {live_label} }
                }
            } else {
                button {
                    id: "{trigger_id}",
                    r#type: "button",
                    "aria-expanded": if expanded { "true" } else { "false" },
                    "aria-controls": "{content_id}",
                    onclick: move |_| {
                        let next = !current_open;
                        set_open(next);
                        if next {
                            scroll_to_top(&scroll_target);
                        }
                    },
                    class: "group flex h-7 min-w-0 items-center gap-1.5 rounded-md text-left font-medium text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    span { class: "truncate", {completed_summary} }
                    span {
                        "aria-hidden": "true",
                        class: "inline-flex shrink-0 text-muted-foreground/70 group-hover:text-foreground",
                        style: "transform:rotate({rotate}deg);transition:transform {chevron_duration}ms cubic-bezier(0.34,1.56,0.64,1);",
                        Icon { name: IconName::ChevronDown, class: "size-3.5" }
                    }
                }
            }

            AgentDisclosure {
                id: content_id.clone(),
                role: "region",
                aria_labelledby: trigger_id,
                open: expanded,
                div {
                    "data-beui-activity-viewport": "{base}",
                    class: cn!(
                        "scrollbar-hide pr-1",
                        if capped && expanded && !working { "overflow-y-auto" } else { "overflow-y-hidden" },
                    ),
                    style: "height:{viewport_height}px;{mask_style}",
                    div {
                        "data-beui-activity-content": "{base}",
                        role: "list",
                        class: cn!("space-y-0.5 py-2", content_class_name),
                        style: "transform:translateY({stream_offset}px);",
                        for item in items.iter() {
                            div {
                                key: "{item.id()}",
                                role: "listitem",
                                ActivityRow { item: item.clone() }
                            }
                        }
                    }
                }
            }
        }
    }
}

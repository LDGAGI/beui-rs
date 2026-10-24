//! `agents/tool-result.tsx` — the collapsible tool-call result card.
//!
//! Same DOM as the source: a trigger row (kind glyph, title / meta / tool name,
//! status, chevron), a [`AgentDisclosure`] with a `bg-muted/80` viewport and an
//! optional copy / retry action row. `status` drives opening and collapsing —
//! returning to `running` opens, leaving `running` collapses — via the same
//! previous-value effect the source uses.

use beui::prelude::*;

use crate::agent_code::{AgentCode, AgentCodeLanguage};
use crate::common::{
    copy_to_clipboard, schedule_reset, scroll_to_bottom, use_unique_id, Icon, RollText,
};
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;

/// `ToolResultStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ToolResultStatus {
    #[default]
    Running,
    Success,
    Error,
    Cancelled,
}

/// `ToolResultKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ToolResultKind {
    Terminal,
    Request,
    #[default]
    Custom,
}

fn status_label(status: ToolResultStatus) -> &'static str {
    match status {
        ToolResultStatus::Running => "Running",
        ToolResultStatus::Success => "Completed",
        ToolResultStatus::Error => "Failed",
        ToolResultStatus::Cancelled => "Cancelled",
    }
}

fn status_class(status: ToolResultStatus) -> &'static str {
    match status {
        ToolResultStatus::Running => "text-blue-600 dark:text-blue-400",
        ToolResultStatus::Success => "text-emerald-600 dark:text-emerald-400",
        ToolResultStatus::Error => "text-rose-600 dark:text-rose-400",
        ToolResultStatus::Cancelled => "text-muted-foreground",
    }
}

fn kind_icon(kind: ToolResultKind) -> IconName {
    match kind {
        ToolResultKind::Terminal => IconName::SquareTerminal,
        ToolResultKind::Request => IconName::Braces,
        ToolResultKind::Custom => IconName::Wrench,
    }
}

fn status_icon(status: ToolResultStatus) -> IconName {
    match status {
        ToolResultStatus::Running => IconName::LoaderCircle,
        ToolResultStatus::Success => IconName::CircleCheck,
        ToolResultStatus::Error => IconName::CircleX,
        ToolResultStatus::Cancelled => IconName::Ban,
    }
}

/// `ToolResultOutput`.
#[component]
pub fn ToolResultOutput(
    code: String,
    #[props(default)] language: AgentCodeLanguage,
    #[props(default)] class: String,
) -> Element {
    rsx! {
        AgentCode {
            code,
            language,
            class: cn!("whitespace-pre-wrap break-words text-foreground/80", class),
        }
    }
}

/// `ToolResultAction`.
#[component]
fn ToolResultAction(label: String, onclick: EventHandler<()>, children: Element) -> Element {
    rsx! {
        button {
            r#type: "button",
            "aria-label": "{label}",
            title: "{label}",
            onclick: move |_| onclick.call(()),
            class: "grid size-7 place-items-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring active:scale-90",
            {children}
        }
    }
}

/// `ToolResult`.
#[component]
pub fn ToolResult(
    tool: Element,
    title: Element,
    children: Element,
    #[props(default)] status: ToolResultStatus,
    #[props(default)] kind: ToolResultKind,
    #[props(default)] meta: Option<Element>,
    #[props(default)] icon: Option<Element>,
    #[props(default)] open: Option<bool>,
    #[props(default = true)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = true)] collapse_on_complete: bool,
    #[props(default = 220)] max_height: i32,
    #[props(default)] copy_text: Option<String>,
    #[props(default)] on_copy: Option<EventHandler<()>>,
    #[props(default)] on_retry: Option<EventHandler<()>>,
    #[props(default)] class: String,
    #[props(default)] content_class_name: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("tool-result");
    let trigger_id = format!("{base}-trigger");
    let content_id = format!("{base}-content");
    let viewport = format!("{base}-viewport");
    let mut internal_open = use_signal(|| default_open);
    let mut copied = use_signal(|| false);
    let running = status == ToolResultStatus::Running;
    let can_copy = copy_text.is_some() || on_copy.is_some();
    let current_open = open.unwrap_or(internal_open());
    let status_text = status_label(status);

    let mut set_open = move |next: bool| {
        if open.is_none() {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };

    // Opening a running tool, collapsing one that finished. `previous` is a
    // `Cell`, not a signal — reading and writing a signal inside one effect
    // re-runs it forever (see `todo_list.rs`).
    let previous_status = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(status)));
    use_effect(use_reactive((&status,), move |(status,)| {
        let previous = previous_status.get();
        if previous != ToolResultStatus::Running && status == ToolResultStatus::Running {
            internal_open.set(true);
        }
        if previous == ToolResultStatus::Running
            && status != ToolResultStatus::Running
            && collapse_on_complete
        {
            internal_open.set(false);
        }
        previous_status.set(status);
    }));

    let scroll_selector = format!("[data-beui-tool-viewport=\"{viewport}\"]");
    use_effect(use_reactive((&current_open, &running), move |_| {
        if running {
            scroll_to_bottom(&scroll_selector, !reduce);
        }
    }));

    let handle_copy = move |_| {
        match (on_copy, &copy_text) {
            (Some(handler), _) => handler.call(()),
            (None, Some(text)) => copy_to_clipboard(text),
            (None, None) => {}
        }
        copied.set(true);
        schedule_reset(copied, 1600);
    };

    let rotate = if current_open { 180 } else { 0 };
    let chevron_duration = if reduce { 0 } else { 200 };

    rsx! {
        div {
            "data-state": match status {
                ToolResultStatus::Running => "running",
                ToolResultStatus::Success => "success",
                ToolResultStatus::Error => "error",
                ToolResultStatus::Cancelled => "cancelled",
            },
            "aria-busy": if running { "true" } else { "false" },
            class: cn!("w-full text-sm", class),
            button {
                id: "{trigger_id}",
                r#type: "button",
                "aria-expanded": if current_open { "true" } else { "false" },
                "aria-controls": "{content_id}",
                onclick: move |_| set_open(!current_open),
                class: "group flex min-h-9 w-full items-center gap-2 rounded-md py-1 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                span { "aria-hidden": "true", class: "grid size-4 shrink-0 place-items-center text-muted-foreground",
                    if let Some(icon) = icon {
                        {icon}
                    } else {
                        Icon { name: kind_icon(kind), class: "size-4" }
                    }
                }
                span { class: "flex min-w-0 flex-1 items-baseline gap-2",
                    span { class: "min-w-0 truncate font-medium text-foreground/90", {title} }
                    if let Some(meta) = meta {
                        span { class: "shrink-0 text-xs text-muted-foreground/60", {meta} }
                    }
                    span { class: "min-w-0 truncate font-mono text-[11px] text-muted-foreground/55", {tool} }
                }
                span {
                    class: cn!("inline-flex shrink-0 items-center gap-1 text-[11px] font-medium", status_class(status)),
                    Icon {
                        name: status_icon(status),
                        class: cn!("size-3", if running && !reduce { "animate-spin" } else { "" }),
                    }
                    RollText { value: "{status_text}" }
                }
                span {
                    "aria-hidden": "true",
                    class: "shrink-0 text-muted-foreground/50 group-hover:text-muted-foreground",
                    style: "transform:rotate({rotate}deg);transition:transform {chevron_duration}ms cubic-bezier(0.34,1.56,0.64,1);",
                    Icon { name: IconName::ChevronDown, class: "size-3.5" }
                }
            }

            AgentDisclosure {
                id: content_id.clone(),
                role: "region",
                aria_labelledby: trigger_id.clone(),
                open: current_open,
                div { class: "pl-6 pt-1.5",
                    div { class: "overflow-hidden rounded-xl bg-muted/80",
                        div {
                            "data-beui-tool-viewport": "{viewport}",
                            role: "log",
                            "aria-live": "polite",
                            class: "scrollbar-hide overflow-y-auto",
                            style: "max-height:{max_height}px",
                            div { class: cn!("p-3", content_class_name), {children} }
                        }
                        if can_copy || on_retry.is_some() {
                            div { class: "flex items-center gap-0.5 px-2 pb-1.5",
                                if can_copy {
                                    ToolResultAction {
                                        label: if copied() { "Copied".to_string() } else { "Copy result".to_string() },
                                        onclick: handle_copy,
                                        Icon {
                                            name: if copied() { IconName::Check } else { IconName::Copy },
                                            class: "size-3.5",
                                        }
                                    }
                                }
                                if let Some(on_retry) = on_retry {
                                    ToolResultAction {
                                        label: "Run again".to_string(),
                                        onclick: move |_| on_retry.call(()),
                                        Icon { name: IconName::RotateCcw, class: "size-3.5" }
                                    }
                                }
                                span { class: "ml-auto text-[11px] text-muted-foreground/55",
                                    RollText { value: "{status_text}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

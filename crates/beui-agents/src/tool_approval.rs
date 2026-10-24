//! `agents/tool-approval.tsx` — the allow/deny card for a tool call.
//!
//! Same DOM as the source: a status glyph, title + tool name, a status badge, an
//! optional description, a `View details` disclosure of the parameters, and the
//! pending action row. Leaving `pending` collapses the details, via the same
//! previous-value effect the source uses (stored in a `Cell` — see
//! `todo_list.rs` for why not a signal).

use beui::prelude::*;

use crate::agent_code::{AgentCode, AgentCodeLanguage};
use crate::common::{use_unique_id, Icon};
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;

/// `ToolApprovalStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ToolApprovalStatus {
    #[default]
    Pending,
    Approving,
    Approved,
    Denied,
    Running,
    Complete,
    Error,
}

/// `ToolApprovalParameter`.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolApprovalParameter {
    pub id: String,
    pub label: Element,
    pub value: Element,
}

impl ToolApprovalParameter {
    pub fn new(id: impl Into<String>, label: Element, value: Element) -> Self {
        Self {
            id: id.into(),
            label,
            value,
        }
    }
}

fn status_copy(status: ToolApprovalStatus) -> &'static str {
    match status {
        ToolApprovalStatus::Pending => "Approval required",
        ToolApprovalStatus::Approving => "Approving",
        ToolApprovalStatus::Approved => "Approved",
        ToolApprovalStatus::Denied => "Denied",
        ToolApprovalStatus::Running => "Running",
        ToolApprovalStatus::Complete => "Completed",
        ToolApprovalStatus::Error => "Failed",
    }
}

fn status_badge_class(status: ToolApprovalStatus) -> &'static str {
    match status {
        ToolApprovalStatus::Pending => {
            "border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400"
        }
        ToolApprovalStatus::Approving | ToolApprovalStatus::Running => {
            "border-blue-500/30 bg-blue-500/10 text-blue-600 dark:text-blue-400"
        }
        ToolApprovalStatus::Approved | ToolApprovalStatus::Complete => {
            "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
        }
        ToolApprovalStatus::Denied | ToolApprovalStatus::Error => {
            "border-rose-500/30 bg-rose-500/10 text-rose-600 dark:text-rose-400"
        }
    }
}

/// `ToolApprovalCode`.
#[component]
pub fn ToolApprovalCode(
    code: String,
    #[props(default)] language: AgentCodeLanguage,
    #[props(default)] class: String,
) -> Element {
    rsx! {
        AgentCode {
            code,
            language,
            class: cn!(
                "whitespace-pre-wrap break-words rounded-lg border border-border/50 bg-muted/30 px-2.5 py-2",
                class,
            ),
        }
    }
}

/// `ToolApproval`.
#[component]
pub fn ToolApproval(
    tool: Element,
    #[props(default)] title: Option<Element>,
    #[props(default)] description: Option<Element>,
    #[props(default)] parameters: Vec<ToolApprovalParameter>,
    #[props(default)] status: ToolApprovalStatus,
    #[props(default)] open: Option<bool>,
    #[props(default)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default)] on_approve: Option<EventHandler<()>>,
    #[props(default)] on_always_allow: Option<EventHandler<()>>,
    #[props(default)] on_deny: Option<EventHandler<()>>,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("tool-approval");
    let details_id = format!("{base}-details");
    let mut internal_open = use_signal(|| default_open);
    let current_open = open.unwrap_or(internal_open());
    let busy = matches!(
        status,
        ToolApprovalStatus::Approving | ToolApprovalStatus::Running
    );
    let pending = status == ToolApprovalStatus::Pending;
    let error = status == ToolApprovalStatus::Error;

    let mut set_open = move |next: bool| {
        if open.is_none() {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };

    // Leaving `pending` collapses the details (source effect).
    let previous_status = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(status)));
    use_effect(use_reactive((&status,), move |(status,)| {
        if previous_status.get() == ToolApprovalStatus::Pending
            && status != ToolApprovalStatus::Pending
        {
            if open.is_none() {
                internal_open.set(false);
            }
            if let Some(handler) = on_open_change {
                handler.call(false);
            }
        }
        previous_status.set(status);
    }));

    let glyph = if busy {
        IconName::LoaderCircle
    } else if error {
        IconName::CircleAlert
    } else if status == ToolApprovalStatus::Denied {
        IconName::X
    } else if matches!(
        status,
        ToolApprovalStatus::Approved | ToolApprovalStatus::Complete
    ) {
        IconName::Check
    } else {
        IconName::ShieldCheck
    };
    let has_parameters = !parameters.is_empty();
    let rotate = if current_open { 180 } else { 0 };
    let chevron_duration = if reduce { 0 } else { 200 };

    rsx! {
        div {
            "data-state": match status {
                ToolApprovalStatus::Pending => "pending",
                ToolApprovalStatus::Approving => "approving",
                ToolApprovalStatus::Approved => "approved",
                ToolApprovalStatus::Denied => "denied",
                ToolApprovalStatus::Running => "running",
                ToolApprovalStatus::Complete => "complete",
                ToolApprovalStatus::Error => "error",
            },
            "aria-busy": if busy { "true" } else { "false" },
            class: cn!("w-full overflow-hidden rounded-2xl border border-border/60 bg-muted/20 text-sm", class),
            div { class: "flex items-start gap-3 p-4",
                span {
                    "aria-hidden": "true",
                    class: cn!(
                        "mt-0.5 grid size-8 shrink-0 place-items-center rounded-xl border border-border/60 bg-background text-muted-foreground",
                        if error { "text-destructive" } else { "" },
                    ),
                    Icon {
                        name: glyph,
                        class: cn!("size-4", if busy && !reduce { "animate-spin" } else { "" }),
                    }
                }

                div { class: "min-w-0 flex-1",
                    div { class: "flex min-w-0 items-start justify-between gap-3",
                        div { class: "min-w-0",
                            div { class: "font-medium text-foreground",
                                {title.unwrap_or_else(|| rsx! { "Allow this tool to run?" })}
                            }
                            div { class: "mt-0.5 truncate font-mono text-xs text-muted-foreground", {tool} }
                        }
                        span {
                            class: cn!(
                                "shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-medium transition-colors",
                                status_badge_class(status),
                            ),
                            "{status_copy(status)}"
                        }
                    }
                    if let Some(description) = description {
                        p { class: "mt-2 leading-5 text-muted-foreground", {description} }
                    }

                    if has_parameters {
                        button {
                            r#type: "button",
                            "aria-expanded": if current_open { "true" } else { "false" },
                            "aria-controls": "{details_id}",
                            onclick: move |_| set_open(!current_open),
                            class: "mt-2 inline-flex items-center gap-1 rounded-md text-xs font-medium text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                            "View details"
                            span {
                                "aria-hidden": "true",
                                style: "transform:rotate({rotate}deg);transition:transform {chevron_duration}ms cubic-bezier(0.34,1.56,0.64,1);",
                                Icon { name: IconName::ChevronDown, class: "size-3.5" }
                            }
                        }
                    }
                }
            }

            AgentDisclosure { id: details_id.clone(), open: current_open,
                dl { class: "mx-4 mb-4 grid gap-2 rounded-xl border border-border/50 bg-background/70 p-3",
                    for parameter in parameters.iter() {
                        div {
                            key: "{parameter.id}",
                            class: "grid grid-cols-[minmax(0,7rem)_minmax(0,1fr)] items-center gap-3 text-xs",
                            dt { class: "text-muted-foreground", {parameter.label.clone()} }
                            dd { class: "min-w-0 break-words font-mono text-foreground/85", {parameter.value.clone()} }
                        }
                    }
                }
            }

            if pending {
                div { class: "flex flex-wrap items-center gap-2 border-t border-border/60 px-4 py-3",
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            if let Some(handler) = on_approve {
                                handler.call(());
                            }
                        },
                        class: "rounded-xl bg-foreground px-3 py-1.5 text-xs font-medium text-background outline-none transition-transform focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 active:scale-[0.97]",
                        "Allow once"
                    }
                    if on_always_allow.is_some() {
                        button {
                            r#type: "button",
                            onclick: move |_| {
                                if let Some(handler) = on_always_allow {
                                    handler.call(());
                                }
                            },
                            class: "rounded-xl border border-border/60 bg-background px-3 py-1.5 text-xs font-medium text-foreground outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.97]",
                            "Always allow"
                        }
                    }
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            if let Some(handler) = on_deny {
                                handler.call(());
                            }
                        },
                        class: "rounded-xl px-3 py-1.5 text-xs font-medium text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                        "Deny"
                    }
                }
            }
        }
    }
}

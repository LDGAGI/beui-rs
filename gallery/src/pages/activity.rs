//! Tool approvals and agent activity, on screen: the approval card in its
//! pending/approved states, the streaming activity log, and the three loading
//! surfaces.

use beui::prelude::*;
use beui_agents::agent_activity::{
    AgentActivity, AgentActivityItem, AgentActivitySearch, AgentActivityStatus, AgentActivityStep,
    AgentActivityText, AgentActivityTool, AgentActivityTrace, AgentSearchResult, AgentStepStatus,
};
use beui_agents::loading_states::{
    AgentProgress, ReasoningText, ReasoningTextVariant, ThinkingShimmer,
};
use beui_agents::tool_approval::{ToolApproval, ToolApprovalParameter, ToolApprovalStatus};

#[component]
pub fn Activity() -> Element {
    let parameters = vec![
        ToolApprovalParameter::new(
            "path",
            rsx! { "path" },
            rsx! { span { class: "text-foreground/85", "crates/beui-agents/src/lib.rs" } },
        ),
        ToolApprovalParameter::new(
            "bytes",
            rsx! { "bytes" },
            rsx! { span { class: "text-foreground/85", "1 284" } },
        ),
    ];

    let working_items = vec![
        AgentActivityItem::Step(AgentActivityStep {
            id: "s1".into(),
            label: rsx! { "Read the source files" },
            status: AgentStepStatus::Complete,
            meta: Some(rsx! { "1.2s" }),
        }),
        AgentActivityItem::Tool(AgentActivityTool {
            id: "t1".into(),
            action: "read".into(),
            target: rsx! { "prompt-input.tsx" },
            additions: None,
            deletions: None,
        }),
        AgentActivityItem::Text(AgentActivityText {
            id: "x1".into(),
            content: rsx! { "The composer measures a hidden mirror." },
        }),
        AgentActivityItem::Search(AgentActivitySearch {
            id: "q1".into(),
            query: rsx! { "motion spring stiffness damping mass" },
            results: vec![AgentSearchResult {
                id: "r1".into(),
                title: rsx! { "Motion docs" },
                domain: Some(rsx! { "motion.dev" }),
                url: Some("https://motion.dev".into()),
                icon: None,
            }],
            more_count: Some(4),
        }),
        AgentActivityItem::Step(AgentActivityStep {
            id: "s2".into(),
            label: rsx! { "Port the component" },
            status: AgentStepStatus::Active,
            meta: None,
        }),
    ];

    let complete_items = vec![
        AgentActivityItem::Trace(AgentActivityTrace {
            id: "tr1".into(),
            kind: "thinking".into(),
            label: rsx! { "Thinking" },
            detail: Some(rsx! { "12 lines" }),
            icon: None,
        }),
        AgentActivityItem::Tool(AgentActivityTool {
            id: "t2".into(),
            action: "edit".into(),
            target: rsx! { "lib.rs" },
            additions: Some(18),
            deletions: Some(3),
        }),
    ];

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Agents"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Approvals & activity" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The tool approval card, the streaming activity log, and the loading surfaces an agent shows while it works."
                }
            }

            Panel {
                title: "ToolApproval",
                note: "Pending with parameters, and the same card after approval collapses its details.",
                div { class: "flex max-w-2xl flex-col gap-4",
                    ToolApproval {
                        tool: rsx! { "fs.read" },
                        parameters: parameters.clone(),
                        on_approve: move |_| {},
                        on_always_allow: move |_| {},
                        on_deny: move |_| {},
                    }
                    ToolApproval {
                        tool: rsx! { "fs.read" },
                        status: ToolApprovalStatus::Approved,
                        parameters,
                    }
                }
            }

            Panel {
                title: "AgentActivity",
                note: "Working (always expanded, capped viewport) and completed (collapsible summary).",
                div { class: "flex max-w-2xl flex-col gap-4",
                    AgentActivity { items: working_items }
                    AgentActivity {
                        items: complete_items,
                        status: AgentActivityStatus::Complete,
                        duration: 12.0,
                    }
                }
            }

            Panel {
                title: "Loading states",
                note: "ThinkingShimmer, the progress grid, and ReasoningText in its three variants.",
                div { class: "flex flex-col gap-6",
                    ThinkingShimmer {}
                    AgentProgress {}
                    div { class: "flex flex-col gap-3",
                        ReasoningText {}
                        ReasoningText { variant: ReasoningTextVariant::Swap }
                        ReasoningText { variant: ReasoningTextVariant::Scramble }
                    }
                }
            }
        }
    }
}

#[component]
fn Panel(title: String, note: String, children: Element) -> Element {
    rsx! {
        section { class: "flex flex-col gap-4",
            div { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold tracking-tight", "{title}" }
                p { class: "max-w-3xl text-sm text-muted-foreground", "{note}" }
            }
            {children}
        }
    }
}

//! Agents, on screen: the batch of structured-conversation pieces.
//!
//! Each panel mirrors one beUI preview surface — a code block, the task list
//! (with a completed item and one in progress), a tool result, the citations
//! disclosure, a completed streaming response with sources, and the bare
//! `AgentDisclosure` primitive.

use beui::prelude::*;
use beui_agents::agent_code::AgentCodeLanguage;
use beui_agents::citations::{CitationItem, Citations};
use beui_agents::code_block::{CodeBlock, CodeBlockStatus};
use beui_agents::disclosure::AgentDisclosure;
use beui_agents::streaming_response::{StreamingResponse, StreamingResponseStatus};
use beui_agents::todo_list::{TodoItem, TodoItemStatus, TodoList};
use beui_agents::tool_result::{ToolResult, ToolResultKind, ToolResultOutput, ToolResultStatus};

#[component]
pub fn Agents() -> Element {
    let sample_code = "export function greet(name: string) {\n  // Say hello\n  const message = `Hello, ${name}!`;\n  return message;\n}".to_string();

    let todos = vec![
        TodoItem::new("1", rsx! { "Read the beUI source" }).status(TodoItemStatus::Completed),
        TodoItem::new("2", rsx! { "List DOM, classes and props" })
            .status(TodoItemStatus::InProgress)
            .progress(62.0),
        TodoItem::new("3", rsx! { "Port the component" }),
    ];

    let citations = vec![
        CitationItem::new("1", rsx! { "beUI on GitHub" })
            .domain(rsx! { "github.com" })
            .url("https://github.com/earendil-works"),
        CitationItem::new("2", rsx! { "Motion docs" })
            .domain(rsx! { "motion.dev" })
            .url("https://motion.dev"),
    ];

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Agents"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Agent surfaces"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Messages, streaming responses, tool calls, citations and the task list — the pieces an agent conversation is built from."
                }
            }

            Panel {
                title: "CodeBlock",
                note: "Streaming code with line numbers and a highlighted line.",
                CodeBlock {
                    code: sample_code,
                    language: AgentCodeLanguage::TypeScript,
                    filename: Some(rsx! { "greet.ts" }),
                    status: CodeBlockStatus::Streaming,
                    highlight_lines: vec![3],
                }
            }

            Panel {
                title: "TodoList",
                note: "One item in progress, one done; the counter and status glyphs follow state.",
                div { class: "max-w-md",
                    TodoList { items: todos }
                }
            }

            Panel {
                title: "ToolResult",
                note: "A finished tool call with copy and retry actions.",
                div { class: "max-w-2xl",
                    ToolResult {
                        kind: ToolResultKind::Terminal,
                        title: rsx! { "Run tests" },
                        tool: rsx! { "bun test" },
                        status: ToolResultStatus::Success,
                        copy_text: "3 passed, 0 failed",
                        on_retry: move |_| {},
                        ToolResultOutput { code: "3 passed, 0 failed", language: AgentCodeLanguage::Bash }
                    }
                }
            }

            Panel {
                title: "Citations",
                note: "The Sources disclosure with favicons and outbound links.",
                div { class: "max-w-md",
                    Citations { citations: citations.clone() }
                }
            }

            Panel {
                title: "StreamingResponse",
                note: "A completed answer with copy, feedback and a sources footer.",
                div { class: "max-w-2xl",
                    StreamingResponse {
                        status: StreamingResponseStatus::Complete,
                        copy_text: "The port keeps the source DOM.",
                        sources: citations,
                        default_sources_open: true,
                        p { "The port keeps the source DOM, its class strings and its accessibility tree. Motion details are covered by the motion crate's module docs." }
                    }
                }
            }

            Panel {
                title: "AgentDisclosure",
                note: "The shared reveal used by every collapsible agent surface.",
                div { class: "max-w-md rounded-2xl border border-border/70 p-3",
                    AgentDisclosure { open: true,
                        p { class: "text-sm text-muted-foreground", "Revealed content." }
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

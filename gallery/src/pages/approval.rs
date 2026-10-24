//! Approval flows, on screen: the multi-question `ApprovalCard` and the
//! `FileDiff` disclosure.

use beui::prelude::*;
use beui_agents::approval_card::{ApprovalCard, ApprovalCardOption, ApprovalCardQuestion};
use beui_agents::file_diff::{FileDiff, FileDiffLine, FileDiffStatus};

#[component]
pub fn Approval() -> Element {
    let questions = vec![
        ApprovalCardQuestion::new("scope", rsx! { "Which changes should I keep?" })
            .description(rsx! { "Pick everything that belongs in this commit." })
            .options(vec![
                ApprovalCardOption::new("dioxus", "Dioxus components"),
                ApprovalCardOption::new("gallery", "Gallery pages"),
                ApprovalCardOption::new("docs", "Documentation"),
            ])
            .multiple(true)
            .allow_custom(true),
        ApprovalCardQuestion::new("message", rsx! { "How should the commit read?" }).options(vec![
            ApprovalCardOption::new("plain", "Plain summary"),
            ApprovalCardOption::new("conventional", "Conventional commit"),
        ]),
    ];

    let diff_lines = vec![
        FileDiffLine::new("0", "export function greet(name: string) {")
            .old_line(1)
            .new_line(1),
        FileDiffLine::new("1", "  return `Hello, ${name}!`;")
            .old_line(2)
            .new_line(2),
        FileDiffLine::new("2", "  // Say hello").added().new_line(3),
        FileDiffLine::new("3", "  return message;")
            .added()
            .new_line(4),
        FileDiffLine::new("4", "  const removed = true;")
            .removed()
            .old_line(3),
    ];

    let diff_copy =
        "export function greet(name: string) {\n  return `Hello, ${name}!`;\n}".to_string();

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Agents"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Approval & diffs" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The multi-question approval card, the plain approve/reject card, and the file diff disclosure."
                }
            }

            Panel {
                title: "ApprovalCard (questions)",
                note: "Two questions with checkboxes/radios and a custom response; Next moves between them.",
                div { class: "max-w-2xl",
                    ApprovalCard { questions }
                }
            }

            Panel {
                title: "ApprovalCard (plain)",
                note: "A description-only card with the three actions.",
                div { class: "max-w-2xl",
                    ApprovalCard {
                        title: rsx! { "Deploy to production?" },
                        description: rsx! { "This will run the release workflow on main." },
                        on_approve: move |_| {},
                        on_reject: move |_| {},
                        on_request_changes: move |_| {},
                        on_dismiss: move |_| {},
                    }
                }
            }

            Panel {
                title: "FileDiff",
                note: "Added, removed and context lines with the change counts.",
                div { class: "max-w-2xl",
                    FileDiff {
                        file: rsx! { "greet.ts" },
                        lines: diff_lines,
                        status: FileDiffStatus::Complete,
                        copy_text: diff_copy,
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

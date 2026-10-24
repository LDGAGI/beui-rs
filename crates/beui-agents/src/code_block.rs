//! `agents/code-block.tsx` — the framed, copyable code surface.
//!
//! Same DOM as the source: a rounded `bg-muted/80` card with a 40px header
//! (file glyph, filename, language label, status pill, copy button) and a
//! scroll viewport of line-numbered rows. While `status == Streaming` the
//! viewport autoscrolls to the bottom and the header shows a spinner; copying
//! writes the code and flips the button to a check for 1600 ms.

use beui::prelude::*;

use crate::agent_code::{code_lines, tokenize, AgentCodeLanguage, AgentCodeLine};
use crate::common::{copy_to_clipboard, schedule_reset, scroll_to_bottom, use_unique_id, Icon};
use crate::icons::IconName;

/// `CodeBlockStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CodeBlockStatus {
    Streaming,
    #[default]
    Complete,
}

/// `CodeBlock`.
#[component]
pub fn CodeBlock(
    code: String,
    #[props(default)] language: AgentCodeLanguage,
    #[props(default)] filename: Option<Element>,
    #[props(default)] status: CodeBlockStatus,
    #[props(default = true)] show_line_numbers: bool,
    #[props(default)] highlight_lines: Vec<usize>,
    #[props(default = 280)] max_height: i32,
    #[props(default)] wrap: bool,
    #[props(default = true)] copyable: bool,
    #[props(default)] on_copy: Option<EventHandler<()>>,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let streaming = status == CodeBlockStatus::Streaming;
    let tokens = tokenize(&code, language);
    let lines = code_lines(&code);
    let mut copied = use_signal(|| false);
    let viewport = use_unique_id("beui-code");
    let viewport_selector = format!("[data-beui-code-viewport=\"{viewport}\"]");

    // Autoscroll while streaming, on the same prop dependencies as the source's
    // layout effect.
    let scroll_selector = viewport_selector.clone();
    use_effect(use_reactive((&code, &streaming), move |_| {
        if streaming {
            scroll_to_bottom(&scroll_selector, !reduce);
        }
    }));

    let handle_copy = move |_| {
        match on_copy {
            Some(handler) => {
                handler.call(());
            }
            None => copy_to_clipboard(&code),
        }
        copied.set(true);
        schedule_reset(copied, 1600);
    };

    rsx! {
        div {
            "data-state": if streaming { "streaming" } else { "complete" },
            "aria-busy": if streaming { "true" } else { "false" },
            class: cn!("w-full overflow-hidden rounded-2xl bg-muted/80 text-sm", class),
            div { class: "flex h-10 items-center gap-2.5 px-3",
                Icon {
                    name: IconName::FileCode2,
                    class: "size-3.5 shrink-0 text-muted-foreground/70",
                    "aria-hidden": "true",
                }
                if let Some(filename) = filename {
                    span { class: "min-w-0 truncate font-mono text-xs text-foreground/80", {filename} }
                }
                span { class: "text-[10px] font-medium uppercase tracking-wide text-muted-foreground/55",
                    "{language.label()}"
                }
                span {
                    class: cn!(
                        "ml-auto inline-flex shrink-0 items-center gap-1 text-[10px] font-medium",
                        if streaming { "text-blue-600 dark:text-blue-400" } else { "text-emerald-600 dark:text-emerald-400" },
                    ),
                    Icon {
                        name: if streaming { IconName::LoaderCircle } else { IconName::Check },
                        class: cn!("size-3", if streaming && !reduce { "animate-spin" } else { "" }),
                    }
                    if streaming { "Writing" } else { "Ready" }
                }
                if copyable || on_copy.is_some() {
                    button {
                        r#type: "button",
                        "aria-label": if copied() { "Copied" } else { "Copy code" },
                        title: if copied() { "Copied" } else { "Copy code" },
                        onclick: handle_copy,
                        class: "grid size-7 shrink-0 place-items-center rounded-full text-muted-foreground outline-none transition-colors hover:bg-background/70 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring active:scale-90",
                        Icon {
                            name: if copied() { IconName::Check } else { IconName::Copy },
                            class: "size-3.5",
                        }
                    }
                }
            }

            div {
                "data-beui-code-viewport": "{viewport}",
                role: if streaming { Some("log") } else { None },
                "aria-live": if streaming { Some("polite") } else { None },
                class: "scrollbar-hide overflow-auto border-t border-foreground/[0.06] py-2",
                style: "max-height:{max_height}px",
                pre { class: "m-0 min-w-max font-mono text-xs leading-5 text-foreground/85",
                    code {
                        for (index, (_content, _offset)) in lines.iter().enumerate() {
                            {
                                let line_number = index + 1;
                                let highlighted = highlight_lines.contains(&line_number);
                                rsx! {
                                    span {
                                        key: "{index}",
                                        class: cn!(
                                            "grid min-h-5",
                                            if show_line_numbers { "grid-cols-[2.75rem_minmax(0,1fr)]" } else { "grid-cols-1" },
                                            if highlighted { "bg-blue-500/[0.07]" } else { "" },
                                        ),
                                        if show_line_numbers {
                                            span { class: "select-none pr-3 text-right tabular-nums text-muted-foreground/35",
                                                "{line_number}"
                                            }
                                        }
                                        AgentCodeLine {
                                            code: lines[index].0.clone(),
                                            tokens: tokens.get(index).cloned(),
                                            class: cn!(
                                                "pr-4",
                                                if show_line_numbers { "pl-1" } else { "pl-4" },
                                                if wrap { "whitespace-pre-wrap break-words" } else { "whitespace-pre" },
                                            ),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

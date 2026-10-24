//! `agents/file-diff.tsx` — the per-file diff disclosure.
//!
//! Same DOM as the source: a trigger row (file glyph, path, `+n`/`−n`, applying
//! status, chevron) over an [`AgentDisclosure`] viewport of four-column rows
//! (`oldLine`, `newLine`, marker, code). Highlighting reuses the port's
//! tokenizer; the copy action is the shared clipboard helper.

use beui::prelude::*;

use crate::agent_code::{tokenize, AgentCodeLanguage, AgentCodeLine};
use crate::common::{copy_to_clipboard, schedule_reset, scroll_to_bottom, use_unique_id, Icon};
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;

/// `FileDiffStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FileDiffStatus {
    #[default]
    Streaming,
    Complete,
}

/// `FileDiffLineType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FileDiffLineType {
    Added,
    Removed,
    #[default]
    Context,
}

/// `FileDiffLine`.
#[derive(Clone, Debug, PartialEq)]
pub struct FileDiffLine {
    pub id: String,
    pub line_type: Option<FileDiffLineType>,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub content: String,
}

impl FileDiffLine {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            line_type: None,
            old_line: None,
            new_line: None,
            content: content.into(),
        }
    }

    pub fn added(mut self) -> Self {
        self.line_type = Some(FileDiffLineType::Added);
        self
    }

    pub fn removed(mut self) -> Self {
        self.line_type = Some(FileDiffLineType::Removed);
        self
    }

    pub fn old_line(mut self, line: u32) -> Self {
        self.old_line = Some(line);
        self
    }

    pub fn new_line(mut self, line: u32) -> Self {
        self.new_line = Some(line);
        self
    }
}

/// `FileDiff`.
#[component]
pub fn FileDiff(
    file: Element,
    lines: Vec<FileDiffLine>,
    #[props(default)] status: FileDiffStatus,
    #[props(default)] open: Option<bool>,
    #[props(default = true)] default_open: bool,
    #[props(default)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = true)] collapse_on_complete: bool,
    #[props(default = 220)] max_height: i32,
    #[props(default)] language: AgentCodeLanguage,
    #[props(default)] copy_text: Option<String>,
    #[props(default)] on_copy: Option<EventHandler<()>>,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("file-diff");
    let trigger_id = format!("{base}-trigger");
    let content_id = format!("{base}-content");
    let viewport = format!("{base}-viewport");
    let mut internal_open = use_signal(|| default_open);
    let mut copied = use_signal(|| false);
    let current_open = open.unwrap_or(internal_open());
    let streaming = status == FileDiffStatus::Streaming;
    let additions = lines
        .iter()
        .filter(|line| line.line_type == Some(FileDiffLineType::Added))
        .count();
    let deletions = lines
        .iter()
        .filter(|line| line.line_type == Some(FileDiffLineType::Removed))
        .count();
    let can_copy = copy_text.is_some() || on_copy.is_some();
    let code = lines
        .iter()
        .map(|line| line.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let tokens = tokenize(&code, language);

    let mut set_open = move |next: bool| {
        if open.is_none() {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    };

    // Opening a streamed diff, collapsing one that finished.
    let previous_status = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(status)));
    use_effect(use_reactive((&status,), move |(status,)| {
        if previous_status.get() != FileDiffStatus::Streaming && status == FileDiffStatus::Streaming
        {
            if open.is_none() {
                internal_open.set(true);
            }
            if let Some(handler) = on_open_change {
                handler.call(true);
            }
        }
        if previous_status.get() == FileDiffStatus::Streaming
            && status == FileDiffStatus::Complete
            && collapse_on_complete
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

    let scroll_selector = format!("[data-beui-diff-viewport=\"{viewport}\"]");
    use_effect(use_reactive((&current_open, &streaming), move |_| {
        if streaming {
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

    let change_class = |added: bool| {
        if added {
            "font-mono text-xs tabular-nums text-emerald-600 dark:text-emerald-400"
        } else {
            "font-mono text-xs tabular-nums text-rose-600 dark:text-rose-400"
        }
    };
    let rotate = if current_open { 180 } else { 0 };
    let chevron_duration = if reduce { 0 } else { 200 };

    rsx! {
        div {
            "data-state": if streaming { "streaming" } else { "complete" },
            "aria-busy": if streaming { "true" } else { "false" },
            class: cn!("w-full text-sm", class),
            button {
                id: "{trigger_id}",
                r#type: "button",
                "aria-expanded": if current_open { "true" } else { "false" },
                "aria-controls": "{content_id}",
                onclick: move |_| set_open(!current_open),
                class: "group flex min-h-9 w-full items-center gap-2 rounded-md py-1 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                Icon { name: IconName::FileCode2, class: "size-4 shrink-0 text-muted-foreground", "aria-hidden": "true" }
                span { class: "min-w-0 flex-1 truncate font-mono text-xs text-foreground/80", {file} }
                span { class: "flex shrink-0 items-center gap-2",
                    if additions > 0 {
                        span { class: change_class(true), "+{additions}" }
                    }
                    if deletions > 0 {
                        span { class: change_class(false), "−{deletions}" }
                    }
                }
                span { class: "grid size-4 shrink-0 place-items-center text-muted-foreground/60",
                    if streaming {
                        Icon {
                            name: IconName::LoaderCircle,
                            class: cn!("size-3.5", if !reduce { "animate-spin" } else { "" }),
                            "aria-label": "Applying changes",
                        }
                    } else {
                        Icon { name: IconName::Check, class: "size-3.5", "aria-label": "Changes applied" }
                    }
                }
                span {
                    "aria-hidden": "true",
                    class: "shrink-0 text-muted-foreground/45 transition-colors group-hover:text-muted-foreground",
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
                            "data-slot": "file-diff-viewport",
                            "data-beui-diff-viewport": "{viewport}",
                            "aria-live": "polite",
                            class: "scrollbar-hide overflow-auto",
                            style: "max-height:{max_height}px",
                            div { class: "font-mono text-xs leading-5",
                                span { class: "sr-only", "File changes" }
                                for (index, line) in lines.iter().enumerate() {
                                    {
                                        let line_type = line.line_type.unwrap_or(FileDiffLineType::Context);
                                        let row_class = cn!(
                                            "grid grid-cols-[2.25rem_2.25rem_1rem_minmax(0,1fr)]",
                                            if line_type == FileDiffLineType::Added { "bg-emerald-500/[0.07]" } else { "" },
                                            if line_type == FileDiffLineType::Removed { "bg-rose-500/[0.07]" } else { "" },
                                        );
                                        let marker_class = cn!(
                                            "select-none text-center text-muted-foreground/45",
                                            if line_type == FileDiffLineType::Added { "text-emerald-600 dark:text-emerald-400" } else { "" },
                                            if line_type == FileDiffLineType::Removed { "text-rose-600 dark:text-rose-400" } else { "" },
                                        );
                                        rsx! {
                                            div {
                                                key: "{line.id}",
                                                class: row_class,
                                                span { class: "select-none pr-2 text-right tabular-nums text-muted-foreground/40",
                                                    {line.old_line.map(|n| n.to_string()).unwrap_or_default()}
                                                }
                                                span { class: "select-none pr-2 text-right tabular-nums text-muted-foreground/40",
                                                    {line.new_line.map(|n| n.to_string()).unwrap_or_default()}
                                                }
                                                span { class: marker_class,
                                                    match line_type {
                                                        FileDiffLineType::Added => "+",
                                                        FileDiffLineType::Removed => "−",
                                                        FileDiffLineType::Context => "",
                                                    }
                                                }
                                                AgentCodeLine {
                                                    code: line.content.clone(),
                                                    tokens: tokens.get(index).cloned(),
                                                    class: "min-w-0 whitespace-pre px-1.5",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if can_copy {
                            div { class: "flex justify-end px-2 pb-1.5 pt-1",
                                button {
                                    r#type: "button",
                                    "aria-label": if copied() { "Copied" } else { "Copy diff" },
                                    title: if copied() { "Copied" } else { "Copy diff" },
                                    onclick: handle_copy,
                                    class: "grid size-7 place-items-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-background/70 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring active:scale-90",
                                    Icon {
                                        name: if copied() { IconName::Check } else { IconName::Copy },
                                        class: "size-3.5",
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

//! `agents/streaming-response.tsx` — the streamed answer with completion
//! actions.
//!
//! The prose block keeps the source's arbitrary-variant class string verbatim
//! (that string is what styles rendered Markdown), the action row appears only
//! once streaming stops, and the sources footer reuses the citation list behind
//! a [`AgentDisclosure`].

use beui::prelude::*;

use crate::citations::{CitationItem, CitationList, CitationStack};
use crate::common::{copy_to_clipboard, schedule_reset, use_unique_id, Icon};
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;

/// `StreamingResponseStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StreamingResponseStatus {
    #[default]
    Streaming,
    Complete,
    Error,
}

/// `StreamingResponseFeedback`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamingResponseFeedback {
    Up,
    Down,
}

/// `ResponseAction`.
#[component]
fn ResponseAction(
    label: String,
    #[props(default)] active: bool,
    onclick: EventHandler<()>,
    children: Element,
) -> Element {
    let feedback = label == "Helpful" || label == "Not helpful";
    rsx! {
        button {
            r#type: "button",
            "aria-label": "{label}",
            title: "{label}",
            "aria-pressed": if feedback { if active { "true" } else { "false" } } else { "" },
            onclick: move |_| onclick.call(()),
            class: cn!(
                "grid size-7 place-items-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring active:scale-90",
                if active { "bg-muted text-foreground" } else { "" },
            ),
            {children}
        }
    }
}

const CONTENT_CLASS: &str = "text-sm leading-6 text-foreground/90 [&_a]:font-medium [&_a]:underline [&_a]:underline-offset-4 [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[0.9em] [&_ol]:my-3 [&_ol]:list-decimal [&_ol]:space-y-1 [&_ol]:pl-5 [&_p+p]:mt-3 [&_pre]:my-3 [&_pre]:overflow-x-auto [&_pre]:rounded-xl [&_pre]:border [&_pre]:border-border [&_pre]:bg-muted/45 [&_pre]:p-3 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_ul]:my-3 [&_ul]:list-disc [&_ul]:space-y-1 [&_ul]:pl-5";

/// `StreamingResponse`.
#[component]
pub fn StreamingResponse(
    children: Element,
    #[props(default)] status: StreamingResponseStatus,
    #[props(default)] copy_text: Option<String>,
    #[props(default)] on_copy: Option<EventHandler<()>>,
    #[props(default)] on_retry: Option<EventHandler<()>>,
    #[props(default)] sources: Vec<CitationItem>,
    #[props(default)] sources_open: Option<bool>,
    #[props(default)] default_sources_open: bool,
    #[props(default)] on_sources_open_change: Option<EventHandler<bool>>,
    #[props(default)] source_id_prefix: Option<String>,
    #[props(default)] feedback: Option<StreamingResponseFeedback>,
    #[props(default)] default_feedback: Option<StreamingResponseFeedback>,
    #[props(default)] on_feedback_change: Option<EventHandler<Option<StreamingResponseFeedback>>>,
    #[props(default = true)] announce: bool,
    #[props(default = true)] show_actions: bool,
    #[props(default)] class: String,
    #[props(default)] content_class_name: String,
    #[props(default)] actions_class_name: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("response-source");
    let mut copied = use_signal(|| false);
    let mut internal_feedback = use_signal(|| default_feedback);
    let mut internal_sources_open = use_signal(|| default_sources_open);
    let current_feedback = feedback.or(internal_feedback());
    let current_sources_open = sources_open.unwrap_or(internal_sources_open());
    let streaming = status == StreamingResponseStatus::Streaming;
    let complete = status == StreamingResponseStatus::Complete;
    let can_copy = copy_text.is_some() || on_copy.is_some();
    let has_sources = !sources.is_empty();
    let show_actions_row =
        show_actions && !streaming && (can_copy || on_retry.is_some() || complete || has_sources);
    let sources_content_id = format!("{base}-sources");
    let prefix = source_id_prefix.unwrap_or(base);

    let handle_copy = move |_| {
        match (on_copy, &copy_text) {
            (Some(handler), _) => handler.call(()),
            (None, Some(text)) => copy_to_clipboard(text),
            (None, None) => {}
        }
        copied.set(true);
        schedule_reset(copied, 1600);
    };

    // `apply_up` / `apply_down` are the source's `setFeedback` with the toggle
    // inlined: two handlers cannot share one `FnMut`, and every captured input
    // is `Copy`.
    let mut apply_up = move || {
        let value = if current_feedback == Some(StreamingResponseFeedback::Up) {
            None
        } else {
            Some(StreamingResponseFeedback::Up)
        };
        if feedback.is_none() {
            internal_feedback.set(value);
        }
        if let Some(handler) = on_feedback_change {
            handler.call(value);
        }
    };
    let mut apply_down = move || {
        let value = if current_feedback == Some(StreamingResponseFeedback::Down) {
            None
        } else {
            Some(StreamingResponseFeedback::Down)
        };
        if feedback.is_none() {
            internal_feedback.set(value);
        }
        if let Some(handler) = on_feedback_change {
            handler.call(value);
        }
    };

    let mut set_sources_open = move |next: bool| {
        if sources_open.is_none() {
            internal_sources_open.set(next);
        }
        if let Some(handler) = on_sources_open_change {
            handler.call(next);
        }
    };

    let source_count = sources.len();
    let source_word = if source_count == 1 {
        "source"
    } else {
        "sources"
    };
    let rotate = if current_sources_open { 180 } else { 0 };
    let chevron_duration = if reduce { 0 } else { 200 };

    rsx! {
        div {
            "data-state": match status {
                StreamingResponseStatus::Streaming => "streaming",
                StreamingResponseStatus::Complete => "complete",
                StreamingResponseStatus::Error => "error",
            },
            "aria-busy": if streaming { "true" } else { "false" },
            class: cn!("w-full", class),
            div {
                "aria-live": if announce { "polite" } else { "off" },
                class: cn!(CONTENT_CLASS, content_class_name),
                {children}
            }

            if show_actions_row {
                div { class: "mt-3",
                    div { class: cn!("flex items-center gap-0.5", actions_class_name),
                        if can_copy {
                            ResponseAction {
                                label: if copied() { "Copied".to_string() } else { "Copy response".to_string() },
                                onclick: handle_copy,
                                Icon {
                                    name: if copied() { IconName::Check } else { IconName::Copy },
                                    class: "size-3.5",
                                }
                            }
                        }
                        if let Some(on_retry) = on_retry {
                            ResponseAction {
                                label: "Retry response".to_string(),
                                onclick: move |_| on_retry.call(()),
                                Icon { name: IconName::RotateCcw, class: "size-3.5" }
                            }
                        }
                        if complete {
                            ResponseAction {
                                label: "Helpful".to_string(),
                                active: current_feedback == Some(StreamingResponseFeedback::Up),
                                onclick: move |_| apply_up(),
                                Icon { name: IconName::ThumbsUp, class: "size-3.5" }
                            }
                            ResponseAction {
                                label: "Not helpful".to_string(),
                                active: current_feedback == Some(StreamingResponseFeedback::Down),
                                onclick: move |_| apply_down(),
                                Icon { name: IconName::ThumbsDown, class: "size-3.5" }
                            }
                        }
                        if has_sources {
                            button {
                                r#type: "button",
                                "aria-expanded": if current_sources_open { "true" } else { "false" },
                                "aria-controls": "{sources_content_id}",
                                onclick: move |_| set_sources_open(!current_sources_open),
                                class: "group ml-1 inline-flex min-h-7 items-center gap-2 rounded-md px-1.5 text-xs text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                                CitationStack { citations: sources.clone() }
                                span { class: "tabular-nums", "{source_count} {source_word}" }
                                span {
                                    "aria-hidden": "true",
                                    class: "text-muted-foreground/50 group-hover:text-muted-foreground",
                                    style: "transform:rotate({rotate}deg);transition:transform {chevron_duration}ms cubic-bezier(0.34,1.56,0.64,1);",
                                    Icon { name: IconName::ChevronDown, class: "size-3" }
                                }
                            }
                        }
                    }

                    if has_sources {
                        AgentDisclosure { id: sources_content_id.clone(), open: current_sources_open,
                            CitationList {
                                citations: sources,
                                id_prefix: Some(prefix),
                                class: "mt-2 rounded-xl bg-muted p-2",
                            }
                        }
                    }
                }
            }
        }
    }
}

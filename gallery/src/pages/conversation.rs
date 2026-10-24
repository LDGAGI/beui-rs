//! The conversation pieces, on screen: `Message` + bubbles inside a
//! `MessageScroller`, and the `PromptInput` composer.

use beui::prelude::*;
use beui_agents::message::{
    Message, MessageAvatar, MessageContent, MessageFooter, MessageGroup, MessageHeader,
    MessageMarker, MessageScroller, MessageTyping,
};
use beui_agents::message_bubble::{
    MessageBubble, MessageBubbleCollapsible, MessageBubbleContent, MessageBubbleGroup,
    MessageBubbleVariant,
};
use beui_agents::prompt_input::{PromptAction, PromptInput, PromptModel};

#[component]
pub fn Conversation() -> Element {
    let models = vec![
        PromptModel::new("gpt-5", rsx! { "GPT-5" }),
        PromptModel::new("claude", rsx! { "Claude" }),
        PromptModel::new("local", rsx! { "Local model" }).disabled(true),
    ];
    let actions = vec![
        PromptAction::new("file", rsx! { "Add file" })
            .description(rsx! { "Attach context from a file" }),
        PromptAction::new("image", rsx! { "Add image" })
            .description(rsx! { "Attach a screenshot" }),
    ];

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Agents"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Conversation" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Message rows, bubble variants, the follow-the-output scroller, and the composer that drives it."
                }
            }

            Panel {
                title: "Message + MessageScroller",
                note: "User and assistant rows with avatars, a marker, a typing indicator and a collapsible bubble.",
                div { class: "h-80 overflow-hidden rounded-2xl border border-border/70 bg-background p-3",
                    MessageScroller {
                        MessageGroup {
                            Message { from: beui_agents::message_context::MessageFrom::User,
                                MessageAvatar { "U" }
                                MessageContent {
                                    MessageHeader { "You" }
                                    MessageBubble { variant: MessageBubbleVariant::Solid,
                                        MessageBubbleContent { "Can you port the message scroller?" }
                                    }
                                }
                            }
                            Message { from: beui_agents::message_context::MessageFrom::Assistant,
                                MessageAvatar { "A" }
                                MessageContent {
                                    MessageHeader { "Assistant" }
                                    MessageBubbleGroup {
                                        MessageBubble {
                                            MessageBubbleContent {
                                                "Sure — the viewport follows streamed output while you stay near the live edge."
                                            }
                                        }
                                        MessageBubble {
                                            MessageBubbleContent {
                                                MessageBubbleCollapsible { collapsed_lines: 2,
                                                    "A collapsible bubble clamps to a line count and reveals the rest behind a mask. Toggle it to see the mask fade out."
                                                }
                                            }
                                        }
                                    }
                                    MessageFooter { "just now" }
                                }
                            }
                            MessageMarker { "Conversation started" }
                            Message { from: beui_agents::message_context::MessageFrom::Assistant,
                                MessageAvatar { placeholder: true }
                                MessageContent {
                                    MessageTyping {}
                                }
                            }
                        }
                    }
                }
            }

            Panel {
                title: "PromptInput",
                note: "Enter submits, Shift+Enter newlines; the model picker and the add-to-prompt popover are inline stand-ins.",
                div { class: "max-w-2xl",
                    PromptInput {
                        models: models,
                        actions: actions,
                        placeholder: "Ask the agent to do something…",
                        on_submit: move |(value, model): (String, Option<String>)| {
                            let _ = (value, model);
                        },
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

//! Generation surfaces, on screen: the image-generation card in three states,
//! the voice orb, and the chat-app sidebar shell.

use beui::prelude::*;
use beui_agents::chat_app::{use_animated_sidebar, ChatApp};
use beui_agents::image_generation::{ImageGeneration, ImageGenerationStatus};
use beui_agents::voice_orb::VoiceOrb;

#[component]
pub fn Generation() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Agents"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Generation" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The image surface (dither field while it works), the voice orb, and the chat shell that folds its sidebar."
                }
            }

            Panel {
                title: "ImageGeneration",
                note: "Queued/generating show the canvas dither field; refining dims it; complete reveals the media; error offers a retry.",
                div { class: "grid grid-cols-1 gap-6 sm:grid-cols-3",
                    ImageGeneration {
                        status: ImageGenerationStatus::Generating,
                        prompt: "a lighthouse at dusk",
                    }
                    ImageGeneration {
                        status: ImageGenerationStatus::Complete,
                        prompt: "a lighthouse at dusk",
                        div { class: "size-full bg-gradient-to-br from-orange-400 via-amber-200 to-emerald-700" }
                    }
                    ImageGeneration {
                        status: ImageGenerationStatus::Error,
                        prompt: "a lighthouse at dusk",
                        on_retry: move |_| {},
                    }
                }
            }

            Panel {
                title: "VoiceOrb",
                note: "DOM + gradient stand-in; the GLSL renderer is deferred to interop.",
                div { class: "flex items-center gap-8",
                    VoiceOrb { activity: 0.0, class: "w-40" }
                    VoiceOrb { activity: 0.7, class: "w-40" }
                }
            }

            Panel {
                title: "ChatApp",
                note: "The shell folds its sidebar under 600px; the toggle uses the provided context.",
                div { class: "h-64",
                    ChatApp {
                        div { class: "flex size-full",
                            ChatSidebar {}
                            div { class: "flex-1 p-4 text-sm text-muted-foreground", "Conversation pane" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ChatSidebar() -> Element {
    let sidebar = use_animated_sidebar();
    let expanded = sidebar.open();
    let width = if expanded { "12rem" } else { "3rem" };
    rsx! {
        div {
            class: "flex h-full flex-col gap-2 border-r border-border bg-muted/40 p-3",
            style: "width:{width};transition:width 220ms cubic-bezier(0.16,1,0.3,1);",
            button {
                r#type: "button",
                "aria-label": "Toggle sidebar",
                onclick: move |_| sidebar.toggle(),
                class: "grid size-8 place-items-center rounded-lg text-sm hover:bg-muted",
                if expanded { "⟨" } else { "⟩" }
            }
            if expanded {
                div { class: "text-xs text-muted-foreground", "Sidebar" }
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

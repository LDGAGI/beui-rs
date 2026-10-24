//! `collapsible.preview.tsx` — the disclosure, with its rotating chevron.

use beui::prelude::*;
use beui::{Collapsible, CollapsibleContent, CollapsibleIndicator, CollapsibleTrigger};

#[component]
pub fn CollapsiblePage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Collapsible" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The height change is a layout animation: the content box jumps to its new height in the same commit and a transform that starts as the inverse of the change decays to nothing."
                }
            }
            Preview {}
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

#[component]
fn Preview() -> Element {
    rsx! {
        Panel {
            title: "Collapsible",
            note: "The trigger keeps `aria-expanded` and `aria-controls` in sync, the content is inert while closed, and closing while focus is inside hands focus back to the trigger.",
            div { class: "flex w-full max-w-sm flex-col gap-6",
                Collapsible { class: "rounded-xl border border-border bg-background p-1",
                    CollapsibleTrigger {
                        "More details"
                        CollapsibleIndicator {}
                    }
                    CollapsibleContent { content_class: "px-3 pb-3 pt-1 text-sm leading-6 text-muted-foreground",
                        p { "Keep extra information tucked away until you need it." }
                        p { "A second line, so the open height is clearly larger than the closed one." }
                    }
                }
            }
        }
    }
}

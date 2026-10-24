//! `alert.preview.tsx` — one banner, four messages, and the variant switch.
//!
//! The source's preview is assembled from `Tabs` and `Button`, which live in
//! other lanes; until they are merged here the page reproduces the same markup
//! with plain buttons carrying the same classes, so the copy and the layout are
//! the preview's even while the dependencies are not.

use beui::prelude::*;
use beui::{
    Alert, AlertAction, AlertClose, AlertContent, AlertDescription, AlertIcon, AlertTitle,
    AlertVariant,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewVariant {
    Info,
    Success,
    Warning,
    Destructive,
}

impl PreviewVariant {
    const ALL: [PreviewVariant; 4] = [
        PreviewVariant::Info,
        PreviewVariant::Success,
        PreviewVariant::Warning,
        PreviewVariant::Destructive,
    ];

    const fn label(self) -> &'static str {
        match self {
            PreviewVariant::Info => "Info",
            PreviewVariant::Success => "Success",
            PreviewVariant::Warning => "Warning",
            PreviewVariant::Destructive => "Error",
        }
    }

    const fn variant(self) -> AlertVariant {
        match self {
            PreviewVariant::Info => AlertVariant::Info,
            PreviewVariant::Success => AlertVariant::Success,
            PreviewVariant::Warning => AlertVariant::Warning,
            PreviewVariant::Destructive => AlertVariant::Destructive,
        }
    }

    const fn title(self) -> &'static str {
        match self {
            PreviewVariant::Info => "A little heads up",
            PreviewVariant::Success => "All set",
            PreviewVariant::Warning => "Almost at the limit",
            PreviewVariant::Destructive => "Something went wrong",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            PreviewVariant::Info => "Your changes are saved automatically as you work.",
            PreviewVariant::Success => "Your changes have been saved.",
            PreviewVariant::Warning => {
                "You're getting close to your storage limit. Free up some space before uploading more files."
            }
            PreviewVariant::Destructive => "We couldn't save your changes. Give it another try.",
        }
    }
}

#[component]
pub fn AlertPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Alert" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A status banner that keeps its layout while it fades: the surface is a presence child, so it stays mounted for the exit and releases interaction in the same commit the exit starts."
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
    let variant = use_signal(|| PreviewVariant::Info);
    let open = use_signal(|| true);
    let mut set_variant = variant;
    let mut set_open = open;
    let selected = variant();

    rsx! {
        Panel {
            title: "Alert",
            note: "Pick a variant; the icon crossfades and the surface springs to the new height. Dismiss, or close it and bring it back from the button underneath — focus returns to that button.",
            div { class: "flex w-full max-w-md flex-col items-center gap-6",
                div { class: "flex w-full items-center rounded-xl bg-muted p-1",
                    for item in PreviewVariant::ALL {
                        button {
                            key: "{item.label()}",
                            r#type: "button",
                            role: "tab",
                            class: cn!(
                                "press rounded-lg px-3 py-1.5 text-xs",
                                if selected == item { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground" },
                            ),
                            onclick: move |_| {
                                set_variant.set(item);
                                set_open.set(true);
                            },
                            "{item.label()}"
                        }
                    }
                }
                Alert {
                    variant: selected.variant(),
                    open: Some(open()),
                    on_open_change: move |next| {
                        let mut signal = set_open;
                        signal.set(next);
                    },
                    return_focus: Some("#alert-show".to_string()),
                    AlertIcon {}
                    AlertContent {
                        AlertTitle { "{selected.title()}" }
                        AlertDescription { "{selected.description()}" }
                        if selected == PreviewVariant::Destructive {
                            AlertAction {
                                button {
                                    r#type: "button",
                                    class: "press h-8 rounded-lg border border-border px-3 text-xs font-medium",
                                    onclick: move |_| set_variant.set(PreviewVariant::Success),
                                    "Try again"
                                }
                            }
                        }
                    }
                    AlertClose {}
                }
                button {
                    id: "alert-show",
                    r#type: "button",
                    disabled: open(),
                    "data-alert-show": "true",
                    class: "press h-9 rounded-lg px-3 text-sm text-muted-foreground disabled:opacity-50",
                    onclick: move |_| set_open.set(true),
                    "Show alert"
                }
            }
        }
    }
}

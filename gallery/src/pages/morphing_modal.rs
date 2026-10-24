//! `morphing-modal.preview.tsx` — one wallet panel, three views.

use beui::prelude::*;
use beui::{
    BanIcon, LockIcon, MorphingModal, ScanFaceIcon, ScrollTextIcon, ShieldCheckIcon, Trash2Icon,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Options,
    PrivateKey,
    Recovery,
}

impl View {
    const fn id(self) -> &'static str {
        match self {
            View::Options => "options",
            View::PrivateKey => "private-key",
            View::Recovery => "recovery",
        }
    }
}

const WORDS: [&str; 12] = [
    "mountain", "river", "candle", "harbor", "amber", "violet", "spring", "ocean", "marble",
    "thunder", "willow", "crystal",
];

#[component]
pub fn MorphingModalPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Morphing modal" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "One panel, several views. Switching the view crossfades the body through a nested popLayout presence, so the old view leaves while the new one enters."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let view = use_signal(|| None::<View>);
    let mut set_view = view;

    let children = match view() {
        None => rsx! {},
        Some(View::Options) => rsx! {
            OptionsPanel {
                on_private_key: move |_| set_view.set(Some(View::PrivateKey)),
                on_recovery: move |_| set_view.set(Some(View::Recovery)),
                on_close: move |_| set_view.set(None),
            }
        },
        Some(View::PrivateKey) => rsx! {
            PrivateKeyPanel { on_back: move |_| set_view.set(Some(View::Options)) }
        },
        Some(View::Recovery) => rsx! {
            RecoveryPanel { on_back: move |_| set_view.set(Some(View::Options)) }
        },
    };

    rsx! {
        section { class: "flex flex-col items-center gap-3",
            button {
                r#type: "button",
                "data-modal-open": "true",
                class: "press inline-flex h-10 items-center rounded-full border border-border bg-card px-5 text-sm font-medium text-foreground hover:border-(--color-border-strong)",
                onclick: move |_| set_view.set(Some(View::Options)),
                "Open wallet options"
            }
            p { class: "text-xs text-muted-foreground",
                "Click a row. The modal morphs height to match new content."
            }
            MorphingModal {
                view_id: view().map(|view| view.id().to_string()),
                on_close: move |_| set_view.set(None),
                {children}
            }
        }
    }
}

#[component]
fn Header(title: &'static str, on_close: EventHandler<()>) -> Element {
    rsx! {
        div { class: "mb-4 flex items-center justify-between",
            h2 { class: "text-base font-semibold text-foreground", "{title}" }
            button {
                r#type: "button",
                "aria-label": "Close",
                class: "inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground hover:bg-foreground/[0.06]",
                onclick: move |_| on_close.call(()),
                "✕"
            }
        }
    }
}

#[component]
fn Row(
    label: &'static str,
    #[props(default = false)] destructive: bool,
    on_click: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: cn!(
                "press flex w-full items-center gap-3 rounded-2xl px-4 py-3 text-sm font-medium transition-colors",
                if destructive {
                    "bg-destructive/10 text-destructive hover:bg-destructive/15"
                } else {
                    "bg-foreground/[0.04] text-foreground hover:bg-foreground/[0.08]"
                },
            ),
            onclick: move |_| on_click.call(()),
            {children}
            "{label}"
        }
    }
}

#[component]
fn OptionsPanel(
    on_private_key: EventHandler<()>,
    on_recovery: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            Header { title: "Options", on_close }
            div { class: "flex flex-col gap-2",
                Row { label: "View Private Key", on_click: on_private_key,
                    LockIcon { class: "h-4 w-4" }
                }
                Row { label: "View Recovery Phrase", on_click: on_recovery,
                    ScrollTextIcon { class: "h-4 w-4" }
                }
                Row { label: "Remove Wallet", destructive: true, on_click: on_close,
                    Trash2Icon { class: "h-4 w-4" }
                }
            }
        }
    }
}

#[component]
fn PrivateKeyPanel(on_back: EventHandler<()>) -> Element {
    rsx! {
        div {
            div { class: "mb-3 flex items-start justify-between",
                LockIcon { class: "h-5 w-5 text-foreground" }
                button {
                    r#type: "button",
                    "aria-label": "Back",
                    class: "inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground hover:bg-foreground/[0.06]",
                    onclick: move |_| on_back.call(()),
                    "✕"
                }
            }
            h2 { class: "text-xl font-semibold tracking-tight text-foreground", "Private Key" }
            p { class: "mt-2 text-sm text-muted-foreground",
                "Your Private Key is the key used to back up your wallet. Keep it secret and secure at all times."
            }
            hr { class: "my-4 border-border" }
            ul { class: "flex flex-col gap-2.5 text-sm text-muted-foreground",
                li { class: "flex items-center gap-2.5", ShieldCheckIcon { class: "h-4 w-4" } "Keep your private key safe" }
                li { class: "flex items-center gap-2.5", ScrollTextIcon { class: "h-4 w-4" } "Don't share it with anyone else" }
                li { class: "flex items-center gap-2.5", BanIcon { class: "h-4 w-4" } "If you lose it, we can't recover it" }
            }
            div { class: "mt-5 flex gap-2",
                button {
                    r#type: "button",
                    class: "press inline-flex h-10 flex-1 items-center justify-center rounded-full bg-foreground/[0.06] text-sm font-medium text-foreground",
                    onclick: move |_| on_back.call(()),
                    "Cancel"
                }
                button {
                    r#type: "button",
                    class: "press inline-flex h-10 flex-1 items-center justify-center gap-2 rounded-full bg-foreground text-sm font-medium text-background",
                    onclick: move |_| on_back.call(()),
                    ScanFaceIcon { class: "h-4 w-4" }
                    "Reveal"
                }
            }
        }
    }
}

#[component]
fn RecoveryPanel(on_back: EventHandler<()>) -> Element {
    rsx! {
        div {
            div { class: "mb-3 flex items-start justify-between",
                ScrollTextIcon { class: "h-5 w-5 text-foreground" }
                button {
                    r#type: "button",
                    "aria-label": "Back",
                    class: "inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground hover:bg-foreground/[0.06]",
                    onclick: move |_| on_back.call(()),
                    "✕"
                }
            }
            h2 { class: "text-xl font-semibold tracking-tight text-foreground", "Recovery Phrase" }
            p { class: "mt-2 text-sm text-muted-foreground",
                "12 words you can use to restore your wallet on any device. Write them down somewhere safe."
            }
            div { class: "mt-4 grid grid-cols-3 gap-2",
                for (index, word) in WORDS.iter().enumerate() {
                    div {
                        key: "{word}",
                        class: "rounded-lg border border-border bg-background/40 px-2 py-1.5 text-xs text-foreground",
                        span { class: "mr-1 text-muted-foreground", "{index + 1}." }
                        "{word}"
                    }
                }
            }
            button {
                r#type: "button",
                class: "press mt-5 inline-flex h-10 w-full items-center justify-center rounded-full bg-foreground text-sm font-medium text-background",
                onclick: move |_| on_back.call(()),
                "Done"
            }
        }
    }
}

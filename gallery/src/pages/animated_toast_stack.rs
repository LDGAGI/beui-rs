//! `animated-toast-stack.preview.tsx` — open toasts from any edge.

use beui::prelude::*;
use beui::{
    use_animated_toast_stack, AnimatedToast, AnimatedToastPatch, AnimatedToastStack,
    AnimatedToastStackOptions, CheckIcon, LoaderCircleIcon, ToastClassNames, ToastPosition,
    ToastStatus, XIcon,
};
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

const POSITIONS: [ToastPosition; 6] = [
    ToastPosition::TopLeft,
    ToastPosition::TopCenter,
    ToastPosition::TopRight,
    ToastPosition::BottomLeft,
    ToastPosition::BottomCenter,
    ToastPosition::BottomRight,
];

const POSITION_LABELS: [(&str, ToastPosition); 6] = [
    ("top-left", ToastPosition::TopLeft),
    ("top-center", ToastPosition::TopCenter),
    ("top-right", ToastPosition::TopRight),
    ("bottom-left", ToastPosition::BottomLeft),
    ("bottom-center", ToastPosition::BottomCenter),
    ("bottom-right", ToastPosition::BottomRight),
];

const EXAMPLES: [(&str, ToastStatus, &str, &str, i64); 4] = [
    ("Title only", ToastStatus::Success, "Saved", "", 0),
    (
        "Promise",
        ToastStatus::Loading,
        "Publishing component",
        "Bundling source, preview, and registry metadata.",
        0,
    ),
    (
        "Success",
        ToastStatus::Success,
        "Component published",
        "Registry endpoint and raw source are available.",
        -1,
    ),
    (
        "Error",
        ToastStatus::Error,
        "Snapshot failed",
        "Retry after the browser target settles.",
        -1,
    ),
];

fn example_toast(label: &str) -> AnimatedToast {
    let (_, status, title, description, duration) = EXAMPLES
        .iter()
        .find(|(name, ..)| *name == label)
        .copied()
        .unwrap();
    let mut toast = AnimatedToast::new("", title)
        .status(status)
        .duration(duration);
    if !description.is_empty() {
        toast = toast.description(description);
    }
    toast
}

#[component]
pub fn AnimatedToastStackPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Toast stack" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Each toast is a presence child of one popLayout boundary, so dismissing one pops it out of layout while the siblings move immediately. The card swings on drag=\"x\" but only ever shows 18% of the finger, so the threshold reads the raw gesture."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let handle = use_animated_toast_stack(AnimatedToastStackOptions {
        default_duration: 3600,
        limit: Some(5),
    });
    let mut position = use_signal(|| ToastPosition::BottomRight);

    let dismiss = EventHandler::new(move |id: String| handle.dismiss(&id));
    let open_example = move |label: &'static str| {
        let id = handle.show(example_toast(label));
        if label == "Promise" {
            spawn(async move {
                if Time::delay(Duration::from_millis(1800)).await.is_ok() {
                    handle.update(
                        &id,
                        AnimatedToastPatch {
                            status: Some(ToastStatus::Success),
                            title: Some("Publish complete".to_string()),
                            description: Some(
                                "Toast updated in-place from loading to success.".to_string(),
                            ),
                            duration: Some(3200),
                        },
                    );
                }
            });
        }
    };
    let mut move_stack = move |next: ToastPosition| {
        position.set(next);
        handle.show(
            AnimatedToast::new("", "Position changed")
                .status(ToastStatus::Info)
                .description(format!("New toasts open from {}.", position_label(next))),
        );
    };

    let names = ToastClassNames {
        surface: Some("bg-card/95".to_string()),
        ..Default::default()
    };

    rsx! {
        div { class: "flex min-h-72 w-full flex-col items-center justify-center gap-6",
            AnimatedToastStack {
                toasts: (handle.toasts)(),
                on_dismiss: dismiss,
                position: position(),
                placement: Some(beui::ToastPlacement::Fixed),
                max_visible: 4,
                class_names: names,
            }
            div { class: "flex flex-col items-center gap-2 text-center",
                p { class: "text-sm font-medium text-foreground", "Open a real toast" }
                p { class: "max-w-sm text-xs leading-5 text-muted-foreground",
                    "Toasts render fixed on the screen. Change position to open a toast from that edge."
                }
            }
            div { class: "flex flex-wrap items-center justify-center gap-2",
                for (label, status, ..) in EXAMPLES {
                    button {
                        key: "{label}",
                        r#type: "button",
                        "data-toast-example": "{label}",
                        class: "press inline-flex h-9 items-center gap-2 rounded-full border border-border bg-card px-4 text-xs font-medium text-foreground transition-colors hover:border-(--color-border-strong)",
                        onclick: move |_| open_example(label),
                        if status == ToastStatus::Loading {
                            LoaderCircleIcon { class: "h-3.5 w-3.5" }
                        } else if status == ToastStatus::Success {
                            CheckIcon { class: "h-3.5 w-3.5" }
                        } else {
                            XIcon { class: "h-3.5 w-3.5" }
                        }
                        "{label}"
                    }
                }
                button {
                    r#type: "button",
                    "data-toast-clear": "true",
                    class: "press inline-flex h-9 items-center rounded-full px-4 text-xs font-medium text-muted-foreground hover:bg-foreground/[0.06] hover:text-foreground",
                    onclick: move |_| handle.clear(),
                    "Clear"
                }
            }
            div { class: "flex flex-wrap items-center justify-center gap-1.5",
                for (label, value) in POSITION_LABELS {
                    button {
                        key: "{label}",
                        r#type: "button",
                        class: cn!(
                            "press rounded-full px-2.5 py-1 text-[11px] font-medium transition-colors",
                            if position() == value { "bg-foreground text-background" } else { "bg-foreground/[0.04] text-muted-foreground hover:bg-foreground/[0.08] hover:text-foreground" },
                        ),
                        onclick: move |_| move_stack(value),
                        "{label}"
                    }
                }
            }
        }
    }
}

fn position_label(position: ToastPosition) -> &'static str {
    POSITIONS
        .iter()
        .position(|value| *value == position)
        .and_then(|index| POSITION_LABELS.get(index).map(|(label, _)| *label))
        .unwrap_or("bottom-right")
}

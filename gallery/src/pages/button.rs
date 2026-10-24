//! Button, on screen — `button-base`, `button-magnetic`, `button-metallic`
//! and `button-stateful` previews.
//!
//! The base preview is the component's own gallery: four variants at `md`, the
//! four sizes, the icon size, and the two ripple buttons. The other three show
//! the magnetic pull, the chrome rim and the state roll.

use beui::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

#[component]
pub fn ButtonPage() -> Element {
    let ok_state = use_signal(|| ButtonState::Idle);
    let err_state = use_signal(|| ButtonState::Idle);

    let run = move |ok: bool| {
        let mut state = if ok { ok_state } else { err_state };
        state.set(ButtonState::Loading);
        spawn(async move {
            let _ = Time::delay(Duration::from_millis(1400)).await;
            state.set(if ok {
                ButtonState::Success
            } else {
                ButtonState::Error
            });
            let _ = Time::delay(Duration::from_millis(1800)).await;
            state.set(ButtonState::Idle);
        });
    };

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Button" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A press and hover spring on SPRING_PRESS, four variants and four sizes, and an optional Material-style ripple."
                }
            }

            section { class: "flex flex-col gap-4",
                SectionHeading {
                    title: "Variants",
                    note: "primary, secondary, outline and ghost at md."
                }
                div { class: "flex flex-wrap items-center justify-center gap-3 rounded-xl border border-border bg-card p-8",
                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Md,
                        "Continue"
                        IconArrowRight { class: "h-4 w-4" }
                    }
                    Button { variant: ButtonVariant::Secondary, size: ButtonSize::Md,
                        IconDownload { class: "h-4 w-4" }
                        "Download"
                    }
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Md, "Outline" }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Md, "Ghost" }
                }
            }

            section { class: "flex flex-col gap-4",
                SectionHeading {
                    title: "Sizes",
                    note: "sm, md, lg and the square icon size."
                }
                div { class: "flex flex-wrap items-center justify-center gap-3 rounded-xl border border-border bg-card p-8",
                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Sm, "Small" }
                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Md, "Medium" }
                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Lg, "Large" }
                    Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Icon,
                        aria_label: "Delete",
                        IconTrash2 { class: "h-4 w-4" }
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                SectionHeading {
                    title: "Ripple",
                    note: "Off by default; the circle spawns at the press point and fades over 1.6s on EASE_OUT."
                }
                div { class: "flex flex-wrap items-center justify-center gap-3 rounded-xl border border-border bg-card p-8",
                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Md, ripple: true, "Ripple" }
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Md, ripple: true, "Tap me" }
                }
            }

            section { class: "flex flex-col gap-4",
                SectionHeading {
                    title: "Magnetic",
                    note: "The wrapper pulls toward the cursor on SPRING_MOUSE; touch and reduced-motion users get a still button."
                }
                div { class: "flex flex-wrap items-center justify-center gap-4 rounded-xl border border-border bg-card p-8",
                    MagneticButton { variant: ButtonVariant::Primary, size: ButtonSize::Md, strength: 0.35,
                        "Hover me"
                        IconArrowRight { class: "h-4 w-4" }
                    }
                    MagneticButton { variant: ButtonVariant::Secondary, size: ButtonSize::Md, strength: 0.25, "Subtle pull" }
                    MagneticButton { variant: ButtonVariant::Outline, size: ButtonSize::Md, strength: 0.5, "Strong pull" }
                }
            }

            section { class: "flex flex-col gap-4",
                SectionHeading {
                    title: "Metallic",
                    note: "A chrome rim drifts on an 8s loop; hovering sends a reflection across the face."
                }
                div { class: "flex flex-wrap items-center justify-center gap-3 rounded-xl border border-border bg-card px-6 py-10",
                    MetallicButton {
                        "Continue"
                        IconArrowUpRight { class: "size-4" }
                    }
                    MetallicButton { size: ButtonSize::Sm,
                        IconSparkles { class: "size-3.5" }
                        "Generate"
                    }
                    MetallicButton { size: ButtonSize::Icon, aria_label: "Magic tools",
                        IconSparkles { class: "size-4" }
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                SectionHeading {
                    title: "Stateful",
                    note: "Click to run: loading rolls in, then success or error, then back to idle."
                }
                div { class: "flex flex-col items-center justify-center gap-3 rounded-xl border border-border bg-card p-8",
                    StatefulButton {
                        state: ok_state(),
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Md,
                        onclick: move |_| run(true),
                        loading_text: "Saving",
                        success_text: "Saved",
                        icon: rsx! { IconArrowRight { class: "h-4 w-4" } },
                        "Save changes"
                    }
                    StatefulButton {
                        state: err_state(),
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Md,
                        onclick: move |_| run(false),
                        loading_text: "Submitting",
                        error_text: "Failed",
                        "Submit"
                    }
                }
            }
        }
    }
}

#[component]
fn SectionHeading(title: String, note: String) -> Element {
    rsx! {
        div { class: "flex flex-col gap-1",
            h2 { class: "text-lg font-semibold tracking-tight", "{title}" }
            p { class: "max-w-3xl text-sm text-muted-foreground", "{note}" }
        }
    }
}

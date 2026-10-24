//! `motion/feedback-widget.tsx` — a corner trigger that morphs into a feedback
//! panel, with sending / sent / error states.
//!
//! The status machine (`idle → open → sending → sent | error`), the shell's
//! radius morph (40 → 20), the staged interaction (the field is armed only once
//! the open morph has settled), the Escape / outside-pointerdown dismissal
//! (blocked while sending), the 1600ms auto-close after success, the eight
//! sprinkles bursting from the check, the check's spring and the SVG path draw
//! are all the source's. The open ease `[0.34, 1.25, 0.64, 1]` (a slight
//! overshoot) and the calmer close `[0.22, 1, 0.36, 1]` are kept per direction.
//!
//! Two substitutions: the source's `Button` / `StatefulButton` (another lane)
//! become plain buttons carrying the same classes and the same
//! `Submit` / `Sending` label swap, and the async `onSubmit` becomes a sync
//! handler plus an `error` prop the caller flips for a failed submission.

use crate::icons::{AlertCircle, MessageSquare, X};
use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationManager, MotionHandle};

const MORPH_OPEN_EASE: &str = "cubic-bezier(0.34, 1.25, 0.64, 1)";
const MORPH_CLOSE_EASE: &str = "cubic-bezier(0.22, 1, 0.36, 1)";
const MORPH_OPEN_DURATION: u64 = 400;
const MORPH_CLOSE_DURATION: u64 = 280;
const SUCCESS_DURATION_MS: u64 = 1600;

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

const SPRINKLE_KEYFRAMES: &str = "\
@keyframes beui-feedback-sprinkle{\
0%{opacity:0;transform:translate(0,0) scale(0.4)}\
30%{opacity:1}\
100%{opacity:0;transform:translate(var(--fx), var(--fy)) scale(0.4)}}\
@keyframes beui-feedback-check{\
0%{transform:scale(0)}\
100%{transform:scale(1)}}\
@keyframes beui-feedback-draw{to{stroke-dashoffset:0}}\
@keyframes beui-feedback-burst{0%,100%{opacity:0.55}50%{opacity:1}}";

/// The widget's state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FeedbackStatus {
    #[default]
    Idle,
    Open,
    Sending,
    Sent,
    Error,
}

/// Which corner the widget sits in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FeedbackPosition {
    #[default]
    BottomRight,
    BottomLeft,
}

/// Props for [`FeedbackWidget`].
#[derive(Props, Clone, PartialEq)]
pub struct FeedbackWidgetProps {
    /// Called on submit. A failed submission is reported by setting `error`.
    #[props(default)]
    pub on_submit: Option<EventHandler<String>>,
    #[props(default)]
    pub position: FeedbackPosition,
    #[props(default = "Help us improve".to_string())]
    pub title: String,
    #[props(default = "Share an idea or report a bug".to_string())]
    pub placeholder: String,
    #[props(default)]
    pub icon: Option<Element>,
    /// Set by the caller when the submission failed.
    #[props(default)]
    pub error: bool,
    /// Set by the caller while an async submission is in flight.
    #[props(default)]
    pub pending: bool,
    #[props(default)]
    pub class: Option<String>,
}

/// Which layer is on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewKey {
    Trigger,
    Form,
    Sent,
    Error,
}

impl ViewKey {
    fn is_panel(self) -> bool {
        self != ViewKey::Trigger
    }
}

/// A view that fades, lifts and blurs in and out. The source drives these
/// through `AnimatePresence`; here each layer owns its own progress, so the
/// leaving view keeps painting while the next one arrives.
#[component]
fn FadeLayer(entering: bool, reduce: bool, children: Element) -> Element {
    let mut progress = use_motion(if entering { 0.0f32 } else { 1.0f32 }).expect("0/1 are finite");
    use_effect(move || {
        let target = if entering { 1.0 } else { 0.0 };
        let config = if reduce {
            beui_core::ease::tween_ease_out(if entering { 180 } else { 140 })
        } else {
            beui_core::ease::tween_ease_out(if entering { 240 } else { 160 })
        };
        let _ = progress.animate_to(target, config);
    });
    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if reduce {
        format!("opacity:{p}")
    } else if entering {
        format!(
            "opacity:{p};transform:translateY({}px);filter:blur({}px)",
            8.0 * (1.0 - p),
            4.0 * (1.0 - p)
        )
    } else {
        format!(
            "opacity:{p};transform:translateY({}px);filter:blur({}px)",
            -8.0 * (1.0 - p),
            4.0 * (1.0 - p)
        )
    };

    rsx! {
        div { style, {children} }
    }
}

/// A feedback widget.
#[component]
pub fn FeedbackWidget(props: FeedbackWidgetProps) -> Element {
    let FeedbackWidgetProps {
        on_submit,
        position,
        title,
        placeholder,
        icon,
        error,
        pending,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let left = position == FeedbackPosition::BottomLeft;
    let mut status = use_signal(FeedbackStatus::default);
    let mut message = use_signal(String::new);
    let mut generation = use_signal(|| 0u32);

    let open = status() != FeedbackStatus::Idle;
    let busy = status() == FeedbackStatus::Sending;

    let dismiss = {
        let mut status = status;
        let mut message = message;
        EventHandler::new(move |_| {
            if status() == FeedbackStatus::Sending {
                return;
            }
            status.set(FeedbackStatus::Idle);
            message.set(String::new());
        })
    };
    let scope = use_dismiss_for(open, dismiss);

    // The async contract: `pending` holds the sending state, `error` switches
    // to the failure view, and clearing `pending` lands on success.
    let mut finish = {
        let mut status = status;
        let mut message = message;
        let mut generation = generation;
        move || {
            status.set(FeedbackStatus::Sent);
            let run = generation() + 1;
            generation.set(run);
            spawn(async move {
                let _ = beui_core::platform::json::<bool>(&format!(
                    "await new Promise((resolve) => setTimeout(resolve, {SUCCESS_DURATION_MS}));\nreturn true;"
                ))
                .await;
                if generation() == run {
                    status.set(FeedbackStatus::Idle);
                    message.set(String::new());
                }
            });
        }
    };
    // `error` and `pending` are plain props, so they have to be declared as
    // dependencies: an effect only re-runs on the signals it tracks.
    use_effect(use_reactive((&error, &pending), move |(error, pending)| {
        if status() != FeedbackStatus::Sending {
            return;
        }
        if error {
            status.set(FeedbackStatus::Error);
        } else if !pending {
            finish();
        }
    }));

    // Focus the field once the open morph has settled, so the caret never
    // appears inside a scaled panel.
    let mut focus_field = move || {
        let run = generation() + 1;
        generation.set(run);
        let delay = if reduce_value { 0 } else { MORPH_OPEN_DURATION };
        spawn(async move {
            let _ = beui_core::platform::json::<bool>(&format!(
                "await new Promise((resolve) => setTimeout(resolve, {delay}));\n\
                 const el = document.querySelector('[data-beui-feedback-field]');\n\
                 if (el) el.focus();\n\
                 return true;"
            ))
            .await;
            let _ = run;
        });
    };

    let mut submit = {
        let mut status = status;
        move || {
            if busy || message().trim().is_empty() {
                return;
            }
            status.set(FeedbackStatus::Sending);
            if let Some(handler) = on_submit {
                handler.call(message());
            }
            // The outcome is decided by the effect above, once the caller's
            // `pending` / `error` have propagated: a synchronous handler leaves
            // both clear and lands on success, an async one sets `pending`.
        }
    };

    let shell_style = format!(
        "transform-origin:{};transition:border-radius {}ms {}, width {}ms {}, height {}ms {}, padding {}ms {}",
        if left { "bottom left" } else { "bottom right" },
        if open { MORPH_OPEN_DURATION } else { MORPH_CLOSE_DURATION },
        if open { MORPH_OPEN_EASE } else { MORPH_CLOSE_EASE },
        if open { MORPH_OPEN_DURATION } else { MORPH_CLOSE_DURATION },
        if open { MORPH_OPEN_EASE } else { MORPH_CLOSE_EASE },
        if open { MORPH_OPEN_DURATION } else { MORPH_CLOSE_DURATION },
        if open { MORPH_OPEN_EASE } else { MORPH_CLOSE_EASE },
        if open { MORPH_OPEN_DURATION } else { MORPH_CLOSE_DURATION },
        if open { MORPH_OPEN_EASE } else { MORPH_CLOSE_EASE },
    );
    let shell_class = cn!(
        "pointer-events-auto absolute bottom-0 overflow-hidden border border-border bg-background text-foreground shadow-lg",
        if open {
            "w-[min(86vw,320px)] rounded-[20px] p-2"
        } else {
            "h-12 w-12 rounded-[40px] p-0"
        },
        if left { "left-0" } else { "right-0" },
    );
    let root_class = cn!(
        "pointer-events-none absolute bottom-4 z-30",
        if left { "left-4" } else { "right-4" },
        class,
    );

    // The layer swap: the leaving view finishes its fade while the next one
    // arrives. Built as a list rather than through `AnimatePresence`, which
    // refuses pre-built keyed children.
    let view_key = match status() {
        FeedbackStatus::Sent => ViewKey::Sent,
        FeedbackStatus::Error => ViewKey::Error,
        FeedbackStatus::Idle => ViewKey::Trigger,
        _ => ViewKey::Form,
    };
    let mut rendered_key = use_signal(|| view_key);
    let mut outgoing_key = use_signal(|| None::<ViewKey>);
    use_effect(use_reactive((&view_key,), move |(view_key,)| {
        let previous = rendered_key();
        if previous == view_key {
            return;
        }
        outgoing_key.set(Some(previous));
        rendered_key.set(view_key);
        spawn(async move {
            let _ = beui_core::platform::json::<bool>(
                "await new Promise((resolve) => setTimeout(resolve, 220));\nreturn true;",
            )
            .await;
            if rendered_key() == view_key {
                outgoing_key.set(None);
            }
        });
    }));

    let form_view = |key: ViewKey| -> Element {
        let mut body: Vec<Element> = Vec::new();
        match key {
            ViewKey::Trigger => body.push(rsx! {
                TriggerView {
                    title: title.clone(),
                    icon: icon.clone(),
                    left,
                    on_open: EventHandler::new(move |_| {
                        status.set(FeedbackStatus::Open);
                        focus_field();
                    }),
                }
            }),
            ViewKey::Sent => body.push(rsx! { SentView { reduce: reduce_value } }),
            ViewKey::Error => body.push(rsx! {
                ErrorView {
                    reduce: reduce_value,
                    on_retry: EventHandler::new(move |_| submit()),
                }
            }),
            ViewKey::Form => body.push(rsx! {
                FormView {
                    title: title.clone(),
                    placeholder: placeholder.clone(),
                    message: message(),
                    busy,
                    on_message: EventHandler::new(move |next: String| message.set(next)),
                    on_close: EventHandler::new(move |_| dismiss.call(())),
                    on_submit: EventHandler::new(move |_| submit()),
                }
            }),
        }
        if key.is_panel() {
            rsx! {
                PanelShell {
                    for layer in body {
                        {layer}
                    }
                }
            }
        } else {
            rsx! {
                for layer in body {
                    {layer}
                }
            }
        }
    };

    let mut layers: Vec<(ViewKey, bool, Element)> = Vec::new();
    if let Some(outgoing) = outgoing_key() {
        if outgoing != view_key {
            layers.push((outgoing, false, form_view(outgoing)));
        }
    }
    layers.push((view_key, true, form_view(view_key)));

    rsx! {
        style { dangerous_inner_html: SPRINKLE_KEYFRAMES }
        div { "data-beui-scope": scope.attr(), class: root_class,
            div { class: shell_class, style: shell_style,
                for (key, entering, view) in layers {
                    FadeLayer { key: "{key:?}", entering, reduce: reduce_value, {view} }
                }
            }
        }
    }
}

/// Escape and outside-pointerdown while open (blocked while sending).
fn use_dismiss_for(open: bool, dismiss: EventHandler<()>) -> beui_core::hooks::OverlayScope {
    let mut open_signal = use_signal(|| open);
    use_effect(move || open_signal.set(open));
    use_dismiss(open_signal, dismiss, DismissOptions::default())
}

/// The corner trigger.
#[component]
fn TriggerView(
    title: String,
    icon: Option<Element>,
    left: bool,
    on_open: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            "aria-label": "{title}",
            aria_haspopup: "dialog",
            class: cn!(
                "absolute bottom-0 flex h-12 w-12 items-center justify-center bg-transparent text-foreground",
                if left { "left-0" } else { "right-0" },
            ),
            onclick: move |_| on_open.call(()),
            span { class: "grid h-5 w-5 shrink-0 place-items-center [&>svg]:h-full [&>svg]:w-full",
                if let Some(icon) = icon {
                    {icon}
                } else {
                    MessageSquare { class: Some("h-5 w-5".to_string()) }
                }
            }
        }
    }
}

/// The panel's content wrapper.
#[component]
fn PanelShell(children: Element) -> Element {
    rsx! {
        div { {children} }
    }
}

/// The form.
#[component]
fn FormView(
    title: String,
    placeholder: String,
    message: String,
    busy: bool,
    on_message: EventHandler<String>,
    on_close: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "min-h-[150px] rounded-[20px] bg-border/60 px-4 py-3.5",
            div { class: "flex items-start justify-between gap-3",
                h3 { class: "text-sm font-semibold text-foreground", "{title}" }
                button {
                    r#type: "button",
                    aria_label: "Close",
                    class: "flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-foreground/[0.07] text-muted-foreground transition-colors hover:text-foreground",
                    onclick: move |_| on_close.call(()),
                    X { class: Some("h-3 w-3".to_string()) }
                }
            }
            textarea {
                "data-beui-feedback-field": "",
                value: "{message}",
                placeholder,
                disabled: busy,
                rows: "3",
                class: "mt-2 w-full resize-none bg-transparent text-base text-foreground outline-none placeholder:text-muted-foreground/60 md:text-sm",
                oninput: move |event: Event<FormData>| on_message.call(event.value()),
            }
        }
        div { class: "flex items-center gap-2 px-1 pt-2 pb-1",
            button {
                r#type: "button",
                disabled: busy,
                class: "flex-1 rounded-[12px] bg-border px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground disabled:opacity-40",
                onclick: move |_| on_close.call(()),
                "Cancel"
            }
            button {
                r#type: "button",
                disabled: busy || message.trim().is_empty(),
                class: "flex-1 rounded-[12px] bg-foreground px-4 py-2 text-sm font-medium text-background transition-colors hover:bg-foreground/90 disabled:opacity-40",
                onclick: move |_| on_submit.call(()),
                if busy { "Sending" } else { "Submit" }
            }
        }
    }
}

/// The success view: sprinkles, a springing check and a drawn tick.
#[component]
fn SentView(reduce: bool) -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center gap-1.5 rounded-[20px] bg-border/60 px-4 py-6 text-center",
            div { class: "relative mb-1 flex h-12 w-12 items-center justify-center",
                if !reduce {
                    for index in 0..8usize {
                        span {
                            key: "sprinkle-{index}",
                            class: "absolute h-1.5 w-1.5 rounded-full",
                            style: sprinkle_style(index),
                        }
                    }
                }
                div {
                    class: "flex h-12 w-12 items-center justify-center rounded-full bg-(--color-success)",
                    style: if reduce {
                        "transform:scale(1)".to_string()
                    } else {
                        "animation:beui-feedback-check 320ms cubic-bezier(0.34, 1.56, 0.64, 1) 40ms both".to_string()
                    },
                    svg {
                        view_box: "0 0 24 24",
                        fill: "none",
                        class: "h-5 w-5 text-white",
                        path {
                            d: "M5 12.5l4.5 4.5L19 7.5",
                            stroke: "currentColor",
                            stroke_width: "2.5",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            style: if reduce {
                                String::new()
                            } else {
                                "stroke-dasharray:24;stroke-dashoffset:24;animation:beui-feedback-draw 350ms ease-out 150ms both".to_string()
                            },
                        }
                    }
                }
            }
            h3 { class: "text-sm font-semibold text-foreground", "Thanks!" }
            p { class: "text-xs leading-relaxed text-muted-foreground",
                "Your feedback helps us build something better."
            }
        }
    }
}

/// The error view.
#[component]
fn ErrorView(reduce: bool, on_retry: EventHandler<()>) -> Element {
    let _ = reduce;
    rsx! {
        div { role: "alert", class: "rounded-[20px] bg-border/60 px-4 py-5 text-center",
            div { class: "mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10 text-destructive",
                AlertCircle { class: Some("h-5 w-5".to_string()) }
            }
            h3 { class: "mt-3 text-sm font-semibold text-foreground", "Something went wrong" }
            p { class: "mt-1 text-xs leading-relaxed text-muted-foreground",
                "We couldn’t send your feedback. Please try again."
            }
            button {
                r#type: "button",
                class: "mt-4 rounded-[12px] bg-foreground px-4 py-2 text-sm font-medium text-background transition-colors hover:bg-foreground/90",
                onclick: move |_| on_retry.call(()),
                "Try again"
            }
        }
    }
}

/// One sprinkle, bursting on its own angle.
fn sprinkle_style(index: usize) -> String {
    let angle = (index as f64 / 8.0) * std::f64::consts::PI * 2.0;
    let x = angle.cos() * 26.0;
    let y = angle.sin() * 26.0;
    let color = if index % 2 == 0 {
        "var(--color-success)"
    } else {
        "var(--accent)"
    };
    format!(
        "--fx:{x:.2}px;--fy:{y:.2}px;background-color:{color};\
         animation:beui-feedback-sprinkle 600ms ease-out {}ms both",
        180 + index * 20
    )
}

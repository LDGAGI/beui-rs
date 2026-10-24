//! `motion/signup-form.tsx` — a sign-up form with validation, a password
//! strength meter and a stateful submit.
//!
//! The rules are the source's verbatim: the permissive `EMAIL_PATTERN`, the
//! eight-character minimum, the five `STRENGTH_LABELS`, the length-weighted
//! `passwordStrength` score (length dominates, extra character classes only
//! nudge it), the five `defaultValidate` messages, and the
//! "reward early, punish late" touch rule — errors are computed on every change
//! but a field only shows one once it has been blurred, while a field already
//! in error clears the moment it becomes valid. The strength meter and the
//! error lines keep the source's animations (`scaleX` bars on `SPRING_LAYOUT`,
//! errors arriving out of a blur).
//!
//! Three substitutions, all from another lane: `Input`, `Checkbox` and
//! `StatefulButton` are reimplemented here with the same roles, states and
//! behaviour (label, icons, success tick, reserved error line, `role="checkbox"`
//! with `aria-checked`, and the status-driven label), and the async `onSubmit`
//! becomes a sync handler plus a controlled `status` / `error_message`.

use crate::icons::{Check, Eye, EyeOff, Lock, Mail, User};
use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

const MIN_PASSWORD_LENGTH: usize = 8;
const STRENGTH_LABELS: [&str; 5] = ["Too short", "Weak", "Fair", "Good", "Strong"];
const STRENGTH_COLORS: [&str; 5] = [
    "bg-destructive",
    "bg-destructive",
    "bg-amber-500",
    "bg-amber-400",
    "bg-(--color-success)",
];

const FIELD_KEYFRAMES: &str = "\
@keyframes beui-signup-in{\
0%{opacity:0;transform:translateY(-4px)}\
100%{opacity:1;transform:translateY(0)}}\
@keyframes beui-signup-spin{to{transform:rotate(360deg)}}";

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The password score, 0-4.
pub fn password_strength(password: &str) -> usize {
    if password.chars().count() < MIN_PASSWORD_LENGTH {
        return 0;
    }
    let mut score = 1;
    let length = password.chars().count();
    if length >= 12 {
        score += 1;
    }
    if length >= 16 {
        score += 1;
    }
    let classes = [
        password.chars().any(|c| c.is_ascii_lowercase()),
        password.chars().any(|c| c.is_ascii_uppercase()),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_alphanumeric()),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if classes >= 3 {
        score += 1;
    }
    score.min(4)
}

/// The field values.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SignUpValues {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub terms: bool,
}

/// The submit state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignUpStatus {
    #[default]
    Idle,
    Loading,
    Success,
    Error,
}

/// Which fields the reader has left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Touched {
    name: bool,
    email: bool,
    password: bool,
    confirm_password: bool,
    terms: bool,
}

/// `defaultValidate`.
fn default_validate(values: &SignUpValues) -> Vec<(&'static str, String)> {
    let mut errors = Vec::new();
    if values.name.trim().is_empty() {
        errors.push(("name", "Enter your name.".to_string()));
    }
    if values.email.trim().is_empty() {
        errors.push(("email", "Enter your email.".to_string()));
    } else if !email_looks_valid(&values.email) {
        errors.push((
            "email",
            "That doesn't look like an email address.".to_string(),
        ));
    }
    if values.password.is_empty() {
        errors.push(("password", "Choose a password.".to_string()));
    } else if values.password.chars().count() < MIN_PASSWORD_LENGTH {
        errors.push((
            "password",
            format!("Use at least {MIN_PASSWORD_LENGTH} characters."),
        ));
    }
    if values.confirm_password.is_empty() {
        errors.push(("confirm_password", "Confirm your password.".to_string()));
    } else if values.confirm_password != values.password {
        errors.push(("confirm_password", "Passwords don't match.".to_string()));
    }
    if !values.terms {
        errors.push(("terms", "Accept the terms to continue.".to_string()));
    }
    errors
}

/// Deliberately permissive: the only real check is sending mail.
fn email_looks_valid(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

/// Props for [`SignUpForm`].
#[derive(Props, Clone, PartialEq)]
pub struct SignUpFormProps {
    /// Controlled values. Omit for uncontrolled.
    #[props(default)]
    pub values: Option<SignUpValues>,
    #[props(default)]
    pub default_values: Option<SignUpValues>,
    #[props(default)]
    pub on_values_change: Option<EventHandler<SignUpValues>>,
    /// Called with valid values only.
    #[props(default)]
    pub on_submit: Option<EventHandler<SignUpValues>>,
    /// Controlled submit state. Omit to let the form track it.
    #[props(default)]
    pub status: Option<SignUpStatus>,
    /// Form-level failure message, shown above the submit button.
    #[props(default)]
    pub error_message: Option<String>,
    #[props(default = Some("Create your account".to_string()))]
    pub title: Option<String>,
    #[props(default = Some("Start building in under a minute.".to_string()))]
    pub description: Option<String>,
    #[props(default = "Create account".to_string())]
    pub submit_label: String,
    #[props(default = true)]
    pub strength_meter: bool,
    #[props(default)]
    pub class: Option<String>,
}

/// A sign-up form.
#[component]
pub fn SignUpForm(props: SignUpFormProps) -> Element {
    let SignUpFormProps {
        values: values_prop,
        default_values,
        on_values_change,
        on_submit,
        status: status_prop,
        error_message,
        title,
        description,
        submit_label,
        strength_meter,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let controlled = values_prop.is_some();
    let internal_values = use_signal(|| default_values.clone().unwrap_or_default());
    let internal_status = use_signal(SignUpStatus::default);
    let values = values_prop.clone().unwrap_or(internal_values());
    let status = status_prop.unwrap_or(internal_status());
    let mut touched = use_signal(Touched::default);
    let mut revealed = use_signal(|| false);

    let errors = default_validate(&values);
    let error_for = |key: &str| -> Option<String> {
        errors
            .iter()
            .find(|(field, _)| *field == key)
            .map(|(_, message)| message.clone())
    };
    let shown_error = |key: &str| -> Option<String> {
        let touched = touched();
        let is_touched = match key {
            "name" => touched.name,
            "email" => touched.email,
            "password" => touched.password,
            "confirm_password" => touched.confirm_password,
            "terms" => touched.terms,
            _ => false,
        };
        if is_touched {
            error_for(key)
        } else {
            None
        }
    };
    let is_valid = |key: &str| -> bool {
        let touched = touched();
        let (is_touched, non_empty) = match key {
            "name" => (touched.name, !values.name.is_empty()),
            "email" => (touched.email, !values.email.is_empty()),
            "confirm_password" => (
                touched.confirm_password,
                !values.confirm_password.is_empty(),
            ),
            _ => (false, false),
        };
        is_touched && error_for(key).is_none() && non_empty
    };

    let mut set_value = {
        let mut internal_values = internal_values;
        let mut internal_status = internal_status;
        move |values: SignUpValues, controlled: bool, status_prop: Option<SignUpStatus>| {
            if !controlled {
                internal_values.set(values.clone());
                if status_prop.is_none() {
                    let current = internal_status();
                    if matches!(current, SignUpStatus::Success | SignUpStatus::Error) {
                        internal_status.set(SignUpStatus::Idle);
                    }
                }
            }
            if let Some(handler) = on_values_change {
                handler.call(values);
            }
        }
    };

    let strength = password_strength(&values.password);
    let show_strength = strength_meter && !values.password.is_empty();
    let is_submitting = status == SignUpStatus::Loading;

    let submit_label_dynamic = match status {
        SignUpStatus::Loading => "Creating account".to_string(),
        SignUpStatus::Success => "Account created".to_string(),
        SignUpStatus::Error => "Try again".to_string(),
        SignUpStatus::Idle => submit_label.clone(),
    };
    let submit = {
        let values = values.clone();
        let mut touched = touched;
        let mut internal_status = internal_status;
        let errors_empty = errors.is_empty();
        move || {
            touched.set(Touched {
                name: true,
                email: true,
                password: true,
                confirm_password: true,
                terms: true,
            });
            if !errors_empty {
                return;
            }
            let Some(handler) = on_submit else {
                return;
            };
            if status_prop.is_none() {
                internal_status.set(SignUpStatus::Loading);
            }
            handler.call(values.clone());
            if status_prop.is_none() {
                internal_status.set(SignUpStatus::Success);
            }
        }
    };

    let form_class = cn!(
        "flex w-full max-w-sm flex-col gap-5 rounded-3xl border border-border p-6",
        class,
    );
    let reveal_class = "text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:text-foreground";

    rsx! {
        style { dangerous_inner_html: FIELD_KEYFRAMES }
        form {
            novalidate: true,
            class: form_class,
            onsubmit: move |event| {
                event.prevent_default();
                {
                    let mut submit = submit.clone();
                    submit();
                }
            },
            if title.is_some() || description.is_some() {
                div { class: "flex flex-col gap-1",
                    if let Some(title) = title.clone() {
                        h2 { class: "text-xl font-semibold tracking-tight text-foreground", "{title}" }
                    }
                    if let Some(description) = description.clone() {
                        p { class: "text-sm text-muted-foreground", "{description}" }
                    }
                }
            }
            div { class: "flex flex-col gap-1",
                FieldInput {
                    label: "Name".to_string(),
                    placeholder: "Ada Lovelace".to_string(),
                    autocomplete: "name".to_string(),
                    value: values.name.clone(),
                    error: shown_error("name"),
                    success: is_valid("name"),
                    disabled: is_submitting,
                    reduce: reduce_value,
                    left_icon: rsx! { User { class: None } },
                    on_change: {
                        let mut next = values.clone();
                        move |text: String| {
                            next.name = text.clone();
                            set_value(next.clone(), controlled, status_prop);
                        }
                    },
                    on_blur: move |_| touched.write().name = true,
                }
                FieldInput {
                    label: "Email".to_string(),
                    r#type: "email".to_string(),
                    input_mode: "email".to_string(),
                    placeholder: "you@example.com".to_string(),
                    autocomplete: "email".to_string(),
                    value: values.email.clone(),
                    error: shown_error("email"),
                    success: is_valid("email"),
                    disabled: is_submitting,
                    reduce: reduce_value,
                    left_icon: rsx! { Mail { class: None } },
                    on_change: {
                        let mut next = values.clone();
                        move |text: String| {
                            next.email = text.clone();
                            set_value(next.clone(), controlled, status_prop);
                        }
                    },
                    on_blur: move |_| touched.write().email = true,
                }
                div { class: "flex flex-col gap-2",
                    FieldInput {
                        label: "Password".to_string(),
                        r#type: (if revealed() { "text" } else { "password" }).to_string(),
                        placeholder: "At least 8 characters".to_string(),
                        autocomplete: "new-password".to_string(),
                        value: values.password.clone(),
                        error: shown_error("password"),
                        success: false,
                        disabled: is_submitting,
                        reduce: reduce_value,
                        left_icon: rsx! { Lock { class: None } },
                        right_icon: Some(rsx! {
                            button {
                                r#type: "button",
                                disabled: is_submitting,
                                aria_label: if revealed() { "Hide password" } else { "Show password" },
                                class: reveal_class,
                                onclick: move |_| revealed.toggle(),
                                if revealed() {
                                    EyeOff { class: None }
                                } else {
                                    Eye { class: None }
                                }
                            }
                        }),
                        on_change: {
                            let mut next = values.clone();
                            move |text: String| {
                                next.password = text.clone();
                                set_value(next.clone(), controlled, status_prop);
                            }
                        },
                        on_blur: move |_| touched.write().password = true,
                    }
                    if show_strength {
                        div {
                            class: "flex flex-col gap-1.5 px-1",
                            style: if reduce_value {
                                String::new()
                            } else {
                                "animation:beui-signup-in 180ms ease-out".to_string()
                            },
                            div { "aria-hidden": "true", class: "flex gap-1.5",
                                for index in 0..4usize {
                                    StrengthBar {
                                        key: "{index}",
                                        filled: index < strength,
                                        color: STRENGTH_COLORS[strength],
                                        reduce: reduce_value,
                                    }
                                }
                            }
                            p { aria_live: "polite", class: "text-xs text-muted-foreground",
                                "Password strength: {STRENGTH_LABELS[strength]}"
                            }
                        }
                    }
                }
                FieldInput {
                    label: "Confirm password".to_string(),
                    r#type: (if revealed() { "text" } else { "password" }).to_string(),
                    placeholder: "Re-enter your password".to_string(),
                    autocomplete: "new-password".to_string(),
                    value: values.confirm_password.clone(),
                    error: shown_error("confirm_password"),
                    success: is_valid("confirm_password"),
                    disabled: is_submitting,
                    reduce: reduce_value,
                    left_icon: rsx! { Lock { class: None } },
                    on_change: {
                        let mut next = values.clone();
                        move |text: String| {
                            next.confirm_password = text.clone();
                            set_value(next.clone(), controlled, status_prop);
                        }
                    },
                    on_blur: move |_| touched.write().confirm_password = true,
                }
            }
            div { class: "flex flex-col gap-1.5",
                TermsCheckbox {
                    checked: values.terms,
                    disabled: is_submitting,
                    label: "I agree to the Terms and Privacy Policy".to_string(),
                    on_change: {
                        let mut next = values.clone();
                        move |checked: bool| {
                            next.terms = checked;
                            touched.write().terms = true;
                            set_value(next.clone(), controlled, status_prop);
                        }
                    },
                }
                if let Some(message) = shown_error("terms") {
                    p {
                        key: "{message}",
                        role: "alert",
                        class: "px-1 text-xs text-destructive",
                        style: if reduce_value {
                            String::new()
                        } else {
                            "animation:beui-signup-in 200ms ease-out".to_string()
                        },
                        "{message}"
                    }
                }
            }
            if let Some(message) = error_message.clone() {
                p {
                    key: "{message}",
                    role: "alert",
                    class: "rounded-2xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive",
                    style: if reduce_value {
                        String::new()
                    } else {
                        "animation:beui-signup-in 200ms ease-out".to_string()
                    },
                    "{message}"
                }
            }
            button {
                r#type: "submit",
                disabled: is_submitting,
                class: "flex w-full items-center justify-center gap-2 rounded-2xl bg-primary px-5 py-3 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-70",
                if is_submitting {
                    span {
                        class: "inline-flex",
                        style: if reduce_value { String::new() } else { "animation:beui-signup-spin 900ms linear infinite".to_string() },
                        ArrowSpinner {}
                    }
                }
                "{submit_label_dynamic}"
            }
        }
    }
}

/// One of the four strength bars.
#[component]
fn StrengthBar(filled: bool, color: &'static str, reduce: bool) -> Element {
    let mut scale = use_motion(if filled { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    use_effect(use_reactive((&filled,), move |(filled,)| {
        let target = if filled { 1.0 } else { 0.0 };
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(SPRING_LAYOUT)
        };
        let _ = scale.animate_to(target, config);
    }));

    let scale_value = f64::from(current(scale));

    rsx! {
        span { class: "h-1 flex-1 overflow-hidden rounded-full bg-muted-foreground/20",
            span {
                class: cn!("block h-full w-full origin-left rounded-full", color),
                style: "transform:scaleX({scale_value})",
            }
        }
    }
}

/// The submit spinner.
#[component]
fn ArrowSpinner() -> Element {
    rsx! {
        svg {
            class: "size-4",
            view_box: "0 0 24 24",
            fill: "none",
            "aria-hidden": "true",
            circle {
                cx: "12",
                cy: "12",
                r: "9",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_opacity: "0.3",
            }
            path {
                d: "M21 12a9 9 0 0 0-9-9",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
            }
        }
    }
}

/// A labelled field with icons, a success tick and a reserved error line.
#[component]
fn FieldInput(
    label: String,
    #[props(default = "text".to_string())] r#type: String,
    #[props(default)] input_mode: Option<String>,
    #[props(default)] placeholder: String,
    #[props(default)] autocomplete: String,
    value: String,
    #[props(default)] error: Option<String>,
    #[props(default)] success: bool,
    #[props(default)] disabled: bool,
    #[props(default)] reduce: bool,
    #[props(default)] left_icon: Option<Element>,
    #[props(default)] right_icon: Option<Element>,
    on_change: EventHandler<String>,
    on_blur: EventHandler<()>,
) -> Element {
    let border = if error.is_some() {
        "border-destructive/60"
    } else {
        "border-border"
    };

    rsx! {
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-muted-foreground", "{label}" }
            div {
                class: cn!(
                    "flex h-11 items-center gap-2 rounded-2xl border bg-background px-3",
                    border,
                ),
                if let Some(icon) = left_icon {
                    span { class: "inline-flex size-4 shrink-0 items-center justify-center text-muted-foreground [&>svg]:size-4",
                        {icon}
                    }
                }
                input {
                    r#type,
                    value: "{value}",
                    placeholder,
                    autocomplete,
                    inputmode: input_mode,
                    disabled,
                    class: "min-w-0 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground/60",
                    oninput: move |event: Event<FormData>| on_change.call(event.value()),
                    onblur: move |_| on_blur.call(()),
                }
                if success {
                    Check { class: Some("size-4 shrink-0 text-(--color-success)".to_string()) }
                }
                if let Some(icon) = right_icon {
                    span { class: "inline-flex shrink-0 items-center [&>svg]:size-4", {icon} }
                }
            }
            // `reserveErrorLine`: the row keeps its height so the form never jumps.
            div { class: "min-h-4 px-1",
                if let Some(message) = error.clone() {
                    p {
                        key: "{message}",
                        role: "alert",
                        class: "text-xs text-destructive",
                        style: if reduce {
                            String::new()
                        } else {
                            "animation:beui-signup-in 200ms ease-out".to_string()
                        },
                        "{message}"
                    }
                }
            }
        }
    }
}

/// The terms checkbox.
#[component]
fn TermsCheckbox(
    checked: bool,
    disabled: bool,
    label: String,
    on_change: EventHandler<bool>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            role: "checkbox",
            aria_checked: if checked { "true" } else { "false" },
            disabled,
            class: "flex items-start gap-2.5 text-left disabled:opacity-60",
            onclick: move |_| on_change.call(!checked),
            span {
                class: cn!(
                    "mt-0.5 flex size-4.5 shrink-0 items-center justify-center rounded-md border transition-colors",
                    if checked {
                        "border-primary bg-primary text-primary-foreground"
                    } else {
                        "border-border bg-background"
                    },
                ),
                if checked {
                    Check { class: Some("size-3".to_string()) }
                }
            }
            span { class: "text-sm text-foreground", "{label}" }
        }
    }
}

//! `agents/prompt-input.tsx` — the composer.
//!
//! Same structure as the source: a `<form>` with a hidden measurement mirror, a
//! self-growing `<textarea>`, an actions popover (only when `actions` is non-
//! empty), the optional leading action, a model picker (only when `models` is
//! non-empty) and the send/stop button. Enter submits, Shift+Enter newlines, and
//! the textarea height is recomputed on every value change by the same hidden
//! mirror the source measures.
//!
//! `Button` / `MorphPopover` / `Select` come from [`crate::standins`] because
//! those motion components are not in this lane; see the module notes.

use beui::prelude::*;

use crate::common::{use_unique_id, Icon};
use crate::icons::IconName;
use crate::standins::{Button, ButtonSize, ButtonVariant};

/// `PromptModel`.
#[derive(Clone, Debug, PartialEq)]
pub struct PromptModel {
    pub value: String,
    pub label: Element,
    pub icon: Option<Element>,
    pub disabled: bool,
}

impl PromptModel {
    pub fn new(value: impl Into<String>, label: Element) -> Self {
        Self {
            value: value.into(),
            label,
            icon: None,
            disabled: false,
        }
    }

    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// `PromptAction`.
#[derive(Clone, Debug, PartialEq)]
pub struct PromptAction {
    pub value: String,
    pub label: Element,
    pub description: Option<Element>,
    pub icon: Option<Element>,
    pub disabled: bool,
}

impl PromptAction {
    pub fn new(value: impl Into<String>, label: Element) -> Self {
        Self {
            value: value.into(),
            label,
            description: None,
            icon: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: Element) -> Self {
        self.description = Some(description);
        self
    }

    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// `PromptInput`.
#[component]
pub fn PromptInput(
    #[props(default)] value: Option<String>,
    #[props(default)] default_value: String,
    #[props(default)] on_value_change: Option<EventHandler<String>>,
    #[props(default)] models: Vec<PromptModel>,
    #[props(default)] model: Option<String>,
    #[props(default)] default_model: Option<String>,
    #[props(default)] on_model_change: Option<EventHandler<String>>,
    #[props(default)] actions: Vec<PromptAction>,
    #[props(default)] on_action: Option<EventHandler<String>>,
    #[props(default)] on_submit: Option<EventHandler<(String, Option<String>)>>,
    #[props(default)] loading: bool,
    #[props(default)] on_stop: Option<EventHandler<()>>,
    #[props(default = 2)] min_rows: i32,
    #[props(default = 8)] max_rows: i32,
    #[props(default)] leading_action: Option<Element>,
    #[props(default = "Ask the agent to do something…".to_string())] placeholder: String,
    #[props(default = "Prompt".to_string())] aria_label: String,
    #[props(default)] disabled: bool,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let reduce = use_reduced_motion()();
    let base = use_unique_id("prompt");
    let textarea_id = format!("{base}-textarea");
    let measure_id = format!("{base}-measure");
    let mut internal_value = use_signal(|| default_value.clone());
    let mut internal_model = use_signal(|| {
        default_model
            .clone()
            .or_else(|| models.first().map(|option| option.value.clone()))
    });
    let mut actions_open = use_signal(|| false);
    let mut model_open = use_signal(|| false);

    let current_value = value.clone().unwrap_or(internal_value());
    let current_model_value = model.clone().or(internal_model());
    let controlled = value.is_some();
    let model_controlled = model.is_some();
    let current_model = models
        .iter()
        .find(|option| Some(&option.value) == current_model_value.as_ref())
        .cloned();
    let can_submit = !current_value.trim().is_empty() && !disabled && !loading;

    // Mirror the source's `resizeTextarea`: measure the hidden div, clamp to
    // `minRows..maxRows` at 24px line height, write the height.
    let resize_textarea = textarea_id.clone();
    let resize_measure = measure_id.clone();
    use_effect(use_reactive((&current_value,), move |_| {
        let script = format!(
            "(function(){{var ta=document.getElementById({ta});var m=document.getElementById({m});if(!ta||!m)return;var lh=24;var h=Math.min(Math.max(m.scrollHeight,{min}*lh),{max}*lh);ta.style.height=h+'px';}})();",
            ta = beui_core::platform::js_string(&resize_textarea),
            m = beui_core::platform::js_string(&resize_measure),
            min = min_rows,
            max = max_rows,
        );
        beui_core::platform::fire(&script);
    }));

    let mut set_value = move |next: String| {
        if !controlled {
            internal_value.set(next.clone());
        }
        if let Some(handler) = on_value_change {
            handler.call(next);
        }
    };

    let focus_textarea = textarea_id.clone();
    let dispatch_value = current_value.clone();
    let dispatch_model = current_model_value.clone();
    let mut dispatch_submit = move || {
        let prompt = dispatch_value.trim().to_string();
        if prompt.is_empty() || disabled || loading {
            return;
        }
        if let Some(handler) = on_submit {
            handler.call((prompt, dispatch_model.clone()));
        }
        if !controlled {
            internal_value.set(String::new());
        }
        beui_core::platform::fire(&format!(
            "var ta=document.getElementById({});if(ta)ta.focus({{preventScroll:true}});",
            beui_core::platform::js_string(&focus_textarea),
        ));
    };
    // A second copy for the textarea's `keydown`: one `FnMut` cannot serve two
    // handlers. The captured inputs are all `Copy` or cheap clones.
    let focus_textarea_key = textarea_id.clone();
    let key_value = current_value.clone();
    let key_model = current_model_value.clone();
    let mut submit_on_enter = move || {
        let prompt = key_value.trim().to_string();
        if prompt.is_empty() || disabled || loading {
            return;
        }
        if let Some(handler) = on_submit {
            handler.call((prompt, key_model.clone()));
        }
        if !controlled {
            internal_value.set(String::new());
        }
        beui_core::platform::fire(&format!(
            "var ta=document.getElementById({});if(ta)ta.focus({{preventScroll:true}});",
            beui_core::platform::js_string(&focus_textarea_key),
        ));
    };

    let show_actions = !actions.is_empty();
    let show_models = !models.is_empty();
    let action_rotate = if actions_open() { 45 } else { 0 };
    let swap_duration = if reduce { 0 } else { 200 };
    let footer_actions = actions.clone();
    let footer_models = models.clone();
    let submit_on_stop = on_stop;

    rsx! {
        form {
            onsubmit: move |event| {
                event.prevent_default();
                dispatch_submit();
            },
            class: cn!(
                "relative w-full rounded-2xl border border-border/80 bg-background p-2 transition-colors focus-within:border-foreground/25",
                if disabled { "opacity-60" } else { "" },
                class,
            ),
            ..attributes,
            div {
                id: "{measure_id}",
                "aria-hidden": "true",
                class: "pointer-events-none invisible absolute inset-x-2 top-0 whitespace-pre-wrap px-2 text-sm leading-6 [overflow-wrap:break-word]",
                "{current_value}\u{200b}"
            }
            textarea {
                id: "{textarea_id}",
                value: "{current_value}",
                disabled,
                placeholder: "{placeholder}",
                "aria-label": "{aria_label}",
                rows: "{min_rows}",
                oninput: move |event| set_value(event.value()),
                onkeydown: move |event| {
                    if event.key() == Key::Enter && !event.modifiers().shift() && !event.is_composing() {
                        event.prevent_default();
                        submit_on_enter();
                    }
                },
                class: "scrollbar-hide block w-full resize-none overflow-y-auto bg-transparent px-2 pt-1.5 text-sm leading-6 text-foreground outline-none placeholder:text-muted-foreground/55",
            }

            div { class: "mt-1 flex min-h-8 items-center gap-1",
                if show_actions {
                    div { class: "relative inline-flex",
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Icon,
                            disabled: disabled || loading,
                            aria_label: "Add to prompt",
                            class: "size-8 rounded-full",
                            onclick: move |_| actions_open.set(!actions_open()),
                            span {
                                "aria-hidden": "true",
                                style: "display:grid;place-items:center;transform:rotate({action_rotate}deg);transition:transform {swap_duration}ms cubic-bezier(0.34,1.56,0.64,1);",
                                Icon { name: IconName::Plus, class: "size-4" }
                            }
                        }
                        if actions_open() {
                            div { class: "absolute bottom-full left-0 z-50 mb-2 w-56 rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-md",
                                for action in footer_actions.iter() {
                                    button {
                                        key: "{action.value}",
                                        r#type: "button",
                                        disabled: action.disabled,
                                        onclick: {
                                            let value = action.value.clone();
                                            move |_| {
                                                if let Some(handler) = on_action {
                                                    handler.call(value.clone());
                                                }
                                                actions_open.set(false);
                                            }
                                        },
                                        class: "flex w-full items-start gap-2.5 rounded-lg px-2.5 py-2 text-left outline-none transition-colors hover:bg-muted focus-visible:bg-muted disabled:pointer-events-none disabled:opacity-50",
                                        if let Some(icon) = action.icon.clone() {
                                            span { class: "mt-0.5 grid size-5 shrink-0 place-items-center text-muted-foreground [&_svg]:size-4", {icon} }
                                        }
                                        span { class: "min-w-0",
                                            span { class: "block text-sm text-foreground", {action.label.clone()} }
                                            if let Some(description) = action.description.clone() {
                                                span { class: "mt-0.5 block text-xs leading-4 text-muted-foreground", {description} }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(leading) = leading_action {
                    {leading}
                }
                if show_models {
                    div { class: "relative inline-flex min-w-0",
                        button {
                            r#type: "button",
                            disabled: disabled || loading,
                            "aria-haspopup": "listbox",
                            "aria-expanded": if model_open() { "true" } else { "false" },
                            onclick: move |_| model_open.set(!model_open()),
                            class: "inline-flex h-8 w-auto max-w-52 items-center rounded-xl border-0 bg-transparent px-2 py-0 text-xs outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
                            span { class: "flex min-w-0 items-center gap-1.5",
                                if let Some(model) = current_model.clone() {
                                    if let Some(icon) = model.icon.clone() {
                                        span { class: "grid size-4 shrink-0 place-items-center text-muted-foreground [&_svg]:size-3.5", {icon} }
                                    }
                                    span { class: "truncate text-muted-foreground", {model.label} }
                                } else {
                                    span { class: "truncate text-muted-foreground", "Choose model" }
                                }
                            }
                        }
                        if model_open() {
                            div { class: "absolute bottom-full left-0 z-50 mb-1 w-52 overflow-hidden rounded-xl border border-border bg-popover p-1 text-popover-foreground shadow-none",
                                for option in footer_models.iter() {
                                    button {
                                        key: "{option.value}",
                                        r#type: "button",
                                        disabled: option.disabled,
                                        onclick: {
                                            let next = option.value.clone();
                                            move |_| {
                                                if !model_controlled {
                                                    internal_model.set(Some(next.clone()));
                                                }
                                                if let Some(handler) = on_model_change {
                                                    handler.call(next.clone());
                                                }
                                                model_open.set(false);
                                            }
                                        },
                                        class: "flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left outline-none transition-colors hover:bg-muted focus-visible:bg-muted disabled:pointer-events-none disabled:opacity-50",
                                        if let Some(icon) = option.icon.clone() {
                                            span { class: "grid size-5 shrink-0 place-items-center text-muted-foreground [&_svg]:size-4", {icon} }
                                        }
                                        span { class: "min-w-0 truncate text-sm text-foreground", {option.label.clone()} }
                                    }
                                }
                            }
                        }
                    }
                }

                Button {
                    r#type: if loading { "button".to_string() } else { "submit".to_string() },
                    size: ButtonSize::Icon,
                    disabled: if loading { submit_on_stop.is_none() } else { !can_submit },
                    aria_label: if loading { "Stop generating".to_string() } else { "Send prompt".to_string() },
                    onclick: move |_| {
                        if loading {
                            if let Some(handler) = submit_on_stop {
                                handler.call(());
                            }
                        }
                    },
                    class: "ml-auto size-8 rounded-full",
                    span {
                        class: "grid place-items-center",
                        if loading {
                            Icon { name: IconName::Square, class: "size-3 fill-current" }
                        } else {
                            Icon { name: IconName::ArrowUp, class: "size-4" }
                        }
                    }
                }
            }
        }
    }
}

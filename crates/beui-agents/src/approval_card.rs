//! `agents/approval-card/` — the multi-question approval card.
//!
//! Same structure as the source: a status glyph and title, an
//! [`AgentDisclosure`] holding either the question flow (one question per step,
//! checkboxes or radios plus an optional custom input) or the plain
//! description/children with approve–reject actions, and a status line once the
//! card is no longer interactive.
//!
//! Not ported: the 240 ms single-select auto-advance (it needs a page timer) and
//! the per-question slide (`AnimatePresence mode="wait"`). Both deliberate. `Checkbox` / `RadioGroup` / `Input` come from
//! [`crate::standins`].

use std::collections::BTreeMap;

use beui::prelude::*;

use crate::common::Icon;
use crate::disclosure::AgentDisclosure;
use crate::icons::IconName;
use crate::standins::{
    Button, ButtonSize, ButtonVariant, Checkbox, Input, RadioGroup, RadioGroupItem,
};

/// `ApprovalCardStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ApprovalCardStatus {
    #[default]
    Pending,
    Submitting,
    Approved,
    Rejected,
    ChangesRequested,
    Answered,
}

/// `ApprovalCardOption`.
#[derive(Clone, Debug, PartialEq)]
pub struct ApprovalCardOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub disabled: bool,
}

impl ApprovalCardOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// `ApprovalCardQuestion`.
#[derive(Clone, Debug, PartialEq)]
pub struct ApprovalCardQuestion {
    pub id: String,
    pub title: Element,
    pub description: Option<Element>,
    pub options: Vec<ApprovalCardOption>,
    pub multiple: bool,
    pub auto_advance: bool,
    pub allow_custom: bool,
    pub custom_placeholder: Option<String>,
}

impl ApprovalCardQuestion {
    pub fn new(id: impl Into<String>, title: Element) -> Self {
        Self {
            id: id.into(),
            title,
            description: None,
            options: Vec::new(),
            multiple: false,
            auto_advance: true,
            allow_custom: false,
            custom_placeholder: None,
        }
    }

    pub fn description(mut self, description: Element) -> Self {
        self.description = Some(description);
        self
    }

    pub fn options(mut self, options: Vec<ApprovalCardOption>) -> Self {
        self.options = options;
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    pub fn allow_custom(mut self, allow_custom: bool) -> Self {
        self.allow_custom = allow_custom;
        self
    }
}

/// `ApprovalCardAnswer`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApprovalCardAnswer {
    pub selected: Vec<String>,
    pub custom: String,
}

impl ApprovalCardAnswer {
    fn is_answered(&self) -> bool {
        !self.selected.is_empty() || !self.custom.trim().is_empty()
    }
}

/// `ApprovalCardAnswers`.
pub type ApprovalCardAnswers = BTreeMap<String, ApprovalCardAnswer>;

fn status_label(status: ApprovalCardStatus) -> &'static str {
    match status {
        ApprovalCardStatus::Pending => "Input required",
        ApprovalCardStatus::Submitting => "Submitting",
        ApprovalCardStatus::Approved => "Approved",
        ApprovalCardStatus::Rejected => "Rejected",
        ApprovalCardStatus::ChangesRequested => "Changes requested",
        ApprovalCardStatus::Answered => "Response submitted",
    }
}

fn status_class(status: ApprovalCardStatus) -> &'static str {
    match status {
        ApprovalCardStatus::Approved | ApprovalCardStatus::Answered => {
            "text-emerald-600 dark:text-emerald-400"
        }
        ApprovalCardStatus::Rejected => "text-rose-600 dark:text-rose-400",
        ApprovalCardStatus::ChangesRequested => "text-amber-600 dark:text-amber-400",
        _ => "text-muted-foreground",
    }
}

fn status_badge_class(status: ApprovalCardStatus) -> &'static str {
    match status {
        ApprovalCardStatus::Pending | ApprovalCardStatus::ChangesRequested => {
            "border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400"
        }
        ApprovalCardStatus::Submitting => {
            "border-blue-500/30 bg-blue-500/10 text-blue-600 dark:text-blue-400"
        }
        ApprovalCardStatus::Approved | ApprovalCardStatus::Answered => {
            "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
        }
        ApprovalCardStatus::Rejected => {
            "border-rose-500/30 bg-rose-500/10 text-rose-600 dark:text-rose-400"
        }
    }
}

/// `ApprovalCard`.
#[component]
pub fn ApprovalCard(
    #[props(default)] title: Option<Element>,
    #[props(default)] description: Option<Element>,
    #[props(default)] children: Option<Element>,
    #[props(default)] questions: Vec<ApprovalCardQuestion>,
    #[props(default)] status: ApprovalCardStatus,
    #[props(default)] answers: Option<ApprovalCardAnswers>,
    #[props(default)] default_answers: ApprovalCardAnswers,
    #[props(default)] on_answers_change: Option<EventHandler<ApprovalCardAnswers>>,
    #[props(default)] step: Option<usize>,
    #[props(default)] default_step: usize,
    #[props(default)] on_step_change: Option<EventHandler<usize>>,
    #[props(default)] on_submit: Option<EventHandler<ApprovalCardAnswers>>,
    #[props(default)] on_approve: Option<EventHandler<()>>,
    #[props(default)] on_reject: Option<EventHandler<()>>,
    #[props(default)] on_request_changes: Option<EventHandler<()>>,
    #[props(default)] on_dismiss: Option<EventHandler<()>>,
    #[props(default)] approve_label: Option<Element>,
    #[props(default)] submit_label: Option<Element>,
    #[props(default)] result: Option<Element>,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal_answers = use_signal(|| default_answers.clone());
    let mut internal_step = use_signal(|| default_step);
    let current_answers = answers.clone().unwrap_or(internal_answers());
    let question_count = questions.len();
    let current_step = (step.unwrap_or(internal_step())).min(question_count.saturating_sub(1));
    let question = questions.get(current_step).cloned();
    let question_mode = question_count > 0;
    let multiple_questions = question_count > 1;
    let pending = status == ApprovalCardStatus::Pending;
    let busy = status == ApprovalCardStatus::Submitting;
    let interactive = pending || busy;
    let empty = ApprovalCardAnswer::default();
    let current_answer = question
        .as_ref()
        .and_then(|question| current_answers.get(&question.id))
        .unwrap_or(&empty)
        .clone();
    let display_title = question
        .as_ref()
        .map(|question| question.title.clone())
        .or(title)
        .unwrap_or_else(|| rsx! { "Approval required" });
    let label = status_label(status);

    let answers_controlled = answers.is_some();
    let set_answers = use_callback(move |next: ApprovalCardAnswers| {
        if !answers_controlled {
            internal_answers.set(next.clone());
        }
        if let Some(handler) = on_answers_change {
            handler.call(next);
        }
    });
    let step_controlled = step.is_some();
    let set_step = use_callback(move |next: usize| {
        if !step_controlled {
            internal_step.set(next);
        }
        if let Some(handler) = on_step_change {
            handler.call(next);
        }
    });

    let glyph = if busy {
        IconName::LoaderCircle
    } else if interactive {
        if question_mode {
            IconName::CircleHelp
        } else {
            IconName::MessageSquareText
        }
    } else if status == ApprovalCardStatus::Rejected {
        IconName::X
    } else {
        IconName::Check
    };

    // The question body: options (checkboxes or radios) plus the custom input.
    let question_body = question.as_ref().map(|question| {
        let q = question.clone();
        let question_id = q.id.clone();
        let answer = current_answer.clone();
        let disabled = busy;
        let answers_for_options = current_answers.clone();
        let answers_for_radio = current_answers.clone();
        let answers_for_input = current_answers.clone();
        let multiple = q.multiple;
        rsx! {
            div {
                if let Some(description) = q.description.clone() {
                    p { class: "mt-1 leading-5 text-muted-foreground", {description} }
                }
                div { class: "mt-3",
                    if !q.options.is_empty() {
                        if multiple {
                            div { class: "grid gap-0.5",
                                for option in q.options.iter() {
                                    {
                                        let option_value = option.value.clone();
                                        let selected = answer.selected.contains(&option_value);
                                        let mut map = answers_for_options.clone();
                                        let question_id = question_id.clone();
                                        let selected_state = answer.selected.clone();
                                        rsx! {
                                            Checkbox {
                                                key: "{option.value}",
                                                checked: selected,
                                                disabled: disabled || option.disabled,
                                                label: Some(option.label.clone()),
                                                description: option.description.clone(),
                                                class: "min-h-9 rounded-lg px-1.5 py-1",
                                                on_checked_change: move |checked| {
                                                    let mut selected = selected_state.clone();
                                                    if checked {
                                                        selected.push(option_value.clone());
                                                    } else {
                                                        selected.retain(|value| value != &option_value);
                                                    }
                                                    map.insert(
                                                        question_id.clone(),
                                                        ApprovalCardAnswer { selected, custom: String::new() },
                                                    );
                                                    set_answers.call(map.clone());
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            {
                                let mut radio_map = answers_for_radio.clone();
                                let question_id = question_id.clone();
                                rsx! {
                                    RadioGroup {
                                        value: answer.selected.first().cloned().unwrap_or_default(),
                                        class: "gap-0.5",
                                        on_value_change: move |value: String| {
                                            radio_map.insert(
                                                question_id.clone(),
                                                ApprovalCardAnswer { selected: vec![value], custom: String::new() },
                                            );
                                            set_answers.call(radio_map.clone());
                                        },
                                        for option in q.options.iter() {
                                            RadioGroupItem {
                                                key: "{option.value}",
                                                value: option.value.clone(),
                                                label: Some(option.label.clone()),
                                                description: option.description.clone(),
                                                disabled: disabled || option.disabled,
                                                class: "min-h-9 rounded-lg px-1.5 py-1",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if q.allow_custom {
                        {
                            let custom_value = answer.custom.clone();
                            let selected = answer.selected.clone();
                            let options_empty = q.options.is_empty();
                            let root_class = cn!("p-0.5", if !options_empty { "mt-1.5" } else { "" });
                            let placeholder = q
                                .custom_placeholder
                                .clone()
                                .unwrap_or_else(|| "Add another response…".to_string());
                            let multiple = q.multiple;
                            let mut input_map = answers_for_input.clone();
                            let question_id = question_id.clone();
                            rsx! {
                                Input {
                                    value: custom_value,
                                    disabled,
                                    placeholder,
                                    class: root_class,
                                    field_class: "h-10 rounded-xl border-0 bg-background/70 focus-within:bg-background".to_string(),
                                    input_class: "px-3 text-sm".to_string(),
                                    on_change: move |value: String| {
                                        input_map.insert(
                                            question_id.clone(),
                                            ApprovalCardAnswer {
                                                selected: if multiple { selected.clone() } else { Vec::new() },
                                                custom: value,
                                            },
                                        );
                                        set_answers.call(input_map.clone());
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    rsx! {
        div {
            "data-state": match status {
                ApprovalCardStatus::Pending => "pending",
                ApprovalCardStatus::Submitting => "submitting",
                ApprovalCardStatus::Approved => "approved",
                ApprovalCardStatus::Rejected => "rejected",
                ApprovalCardStatus::ChangesRequested => "changes-requested",
                ApprovalCardStatus::Answered => "answered",
            },
            "aria-busy": if busy { "true" } else { "false" },
            class: cn!("w-full overflow-hidden rounded-2xl bg-muted p-4 text-sm", class),
            div { class: "flex items-start gap-3",
                span {
                    "aria-hidden": "true",
                    class: cn!("grid size-5 shrink-0 place-items-center text-muted-foreground", status_class(status)),
                    Icon {
                        name: glyph,
                        class: cn!("size-4", if busy && !reduce { "animate-spin" } else { "" }),
                    }
                }

                div { class: "min-w-0 flex-1",
                    div { class: "flex min-w-0 items-start gap-3",
                        h3 { class: "min-w-0 flex-1 text-base font-medium leading-5 text-foreground",
                            {display_title}
                        }
                        if question_mode && interactive {
                            if multiple_questions {
                                span { class: "shrink-0 text-xs tabular-nums text-muted-foreground/65",
                                    "{current_step + 1}/{question_count}"
                                }
                            }
                        } else {
                            span {
                                class: cn!(
                                    "shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-medium transition-colors",
                                    status_badge_class(status),
                                ),
                                "{label}"
                            }
                        }
                        if let Some(on_dismiss) = on_dismiss {
                            button {
                                r#type: "button",
                                "aria-label": "Dismiss",
                                onclick: move |_| on_dismiss.call(()),
                                class: "grid size-5 shrink-0 place-items-center rounded-full text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
                                Icon { name: IconName::X, class: "size-4" }
                            }
                        }
                    }

                    AgentDisclosure { open: interactive,
                        if question_mode {
                            {question_body}
                        } else {
                            div {
                                if let Some(description) = description.clone() {
                                    p { class: "mt-1 leading-5 text-muted-foreground", {description} }
                                }
                                if let Some(children) = children.clone() {
                                    div { class: "mt-3", {children} }
                                }
                            }
                        }

                        if question_mode {
                            div { class: "mt-4 flex items-center gap-3",
                                if multiple_questions {
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        size: ButtonSize::Icon,
                                        aria_label: Some("Previous question".to_string()),
                                        disabled: busy || current_step == 0,
                                        class: "rounded-full",
                                        onclick: move |_| {
                                            if current_step > 0 {
                                                set_step.call(current_step - 1);
                                            }
                                        },
                                        Icon { name: IconName::ArrowLeft, class: "size-4" }
                                    }
                                    span { class: "flex gap-1.5",
                                        span { class: "sr-only", "Question {current_step + 1} of {question_count}" }
                                        for (index, question) in questions.iter().enumerate() {
                                            {
                                                let scale = if index == current_step { 1.0 } else { 0.75 };
                                                let opacity = if index <= current_step { 1.0 } else { 0.35 };
                                                rsx! {
                                                    span {
                                                        key: "{question.id}",
                                                        "aria-hidden": "true",
                                                        class: "size-1.5 rounded-full bg-foreground",
                                                        style: "transform:scale({scale});opacity:{opacity};",
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Button {
                                    size: if current_step + 1 == question_count { ButtonSize::Sm } else { ButtonSize::Icon },
                                    aria_label: Some(if current_step + 1 == question_count { "Submit response".to_string() } else { "Next question".to_string() }),
                                    disabled: busy || !current_answer.is_answered(),
                                    class: "ml-auto rounded-full",
                                    onclick: {
                                        let answers = current_answers.clone();
                                        move |_| {
                                            if current_step + 1 < question_count {
                                                set_step.call(current_step + 1);
                                            } else if let Some(handler) = on_submit {
                                                handler.call(answers.clone());
                                            }
                                        }
                                    },
                                    if busy {
                                        Icon { name: IconName::LoaderCircle, class: "size-4" }
                                    } else if current_step + 1 == question_count {
                                        {submit_label.clone().unwrap_or_else(|| rsx! { "Submit response" })}
                                        Icon { name: IconName::ArrowRight, class: "size-3.5" }
                                    } else {
                                        Icon { name: IconName::ArrowRight, class: "size-4" }
                                    }
                                }
                            }
                        } else {
                            div { class: "mt-4 flex flex-wrap items-center gap-2",
                                Button {
                                    size: ButtonSize::Sm,
                                    disabled: busy,
                                    class: "rounded-full",
                                    onclick: move |_| {
                                        if let Some(handler) = on_approve {
                                            handler.call(());
                                        }
                                    },
                                    {approve_label.clone().unwrap_or_else(|| rsx! { "Approve" })}
                                }
                                if let Some(on_request_changes) = on_request_changes {
                                    Button {
                                        variant: ButtonVariant::Secondary,
                                        size: ButtonSize::Sm,
                                        disabled: busy,
                                        class: "rounded-full",
                                        onclick: move |_| on_request_changes.call(()),
                                        "Request changes"
                                    }
                                }
                                if let Some(on_reject) = on_reject {
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        size: ButtonSize::Sm,
                                        disabled: busy,
                                        class: "rounded-full text-muted-foreground hover:text-rose-600 dark:hover:text-rose-400",
                                        onclick: move |_| on_reject.call(()),
                                        "Reject"
                                    }
                                }
                            }
                        }
                    }

                    if !interactive {
                        p { class: "mt-1 text-sm text-muted-foreground",
                            {result.clone().unwrap_or_else(|| rsx! { "{label}" })}
                        }
                    }
                }
            }
        }
    }
}

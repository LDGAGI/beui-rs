//! Lane-local stand-ins for motion components the agents set composes with.
//!
//! `prompt-input.tsx` imports `Button` (motion/button), `MorphPopover`
//! (motion/popover-morph) and `Select` (motion/select). Those live in the motion
//! lane and are not ported here, so this module carries the smallest faithful
//! versions: the same class strings and DOM, without the morph/select
//! choreography. When the motion lane lands them, the call sites switch over and
//! this module goes away (see the module notes → 需合并).

use beui::prelude::*;

/// `ButtonVariant`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Outline,
}

/// `ButtonSize`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
    Icon,
}

fn variant_class(variant: ButtonVariant) -> &'static str {
    match variant {
        ButtonVariant::Primary => "bg-primary text-primary-foreground hover:bg-primary/90",
        ButtonVariant::Secondary => {
            "border border-border bg-card text-foreground hover:border-border"
        }
        ButtonVariant::Ghost => "text-muted-foreground hover:text-foreground hover:bg-muted/60",
        ButtonVariant::Outline => {
            "border border-border bg-transparent text-foreground hover:bg-muted/60"
        }
    }
}

fn size_class(size: ButtonSize) -> &'static str {
    match size {
        ButtonSize::Sm => "h-8 px-3 text-xs gap-1.5 rounded-full",
        ButtonSize::Md => "h-10 px-5 text-sm gap-2 rounded-full",
        ButtonSize::Lg => "h-12 px-6 text-base gap-2 rounded-full",
        ButtonSize::Icon => "h-8 w-8 rounded-lg",
    }
}

/// `Button` — the base motion button's markup minus the spring press.
#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default = "button".to_string())] r#type: String,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        button {
            r#type: "{r#type}",
            disabled,
            "aria-label": aria_label,
            onclick: move |event| {
                if let Some(handler) = onclick {
                    handler.call(event);
                }
            },
            class: cn!(
                "inline-flex items-center justify-center font-medium select-none transition-colors disabled:pointer-events-none disabled:opacity-50 active:scale-[0.93]",
                variant_class(variant),
                size_class(size),
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `lib/text-shimmer.ts` — the sweep keyframes and the class that carries the
/// gradient. The reduced-motion reset travels with the component in the source
/// for the same reason: an installed copy may never see `globals.css`.
pub const SHIMMER_KEYFRAMES: &str = "@keyframes beui-text-shimmer{from{background-position:200% 0}to{background-position:-200% 0}}@media (prefers-reduced-motion: reduce){.beui-text-shimmer{animation:none !important}}";

/// The shimmer's gradient/clip class (source `TEXT_SHIMMER_CLASS_NAME`).
pub const SHIMMER_CLASS: &str = "beui-text-shimmer bg-[length:200%_100%] bg-clip-text text-transparent bg-[linear-gradient(110deg,var(--muted-foreground)_30%,var(--foreground)_50%,var(--muted-foreground)_70%)]";

/// `textShimmerStyle(duration)`.
pub fn shimmer_style(duration: f64) -> String {
    format!("animation:beui-text-shimmer {duration}s linear infinite;")
}

/// `TextShimmer` — a shimmering inline run.
#[component]
pub fn ShimmerText(
    #[props(default = 1.8)] duration: f64,
    #[props(default)] class: String,
    children: Element,
) -> Element {
    let style = shimmer_style(duration);
    rsx! {
        style { dangerous_inner_html: "{SHIMMER_KEYFRAMES}" }
        span { class: cn!(SHIMMER_CLASS, class), style: "{style}", {children} }
    }
}

/// `Loader` with `variant="ascii-line"` — the `| / - \` terminal spinner.
///
/// The source's `Loader` has eleven variants; the agents set uses only this one,
/// so the stand-in covers it and nothing else.
#[component]
pub fn AsciiLoader(#[props(default = 100)] interval_ms: u32) -> Element {
    let index = crate::timers::use_interval_index(4, interval_ms);
    let frame = ["|", "/", "-", "\\"][index() % 4];
    rsx! {
        span { class: "inline-block tabular-nums", "{frame}" }
    }
}

/// `Checkbox` — the motion checkbox's markup without the path-draw spring.
#[component]
pub fn Checkbox(
    checked: bool,
    on_checked_change: EventHandler<bool>,
    #[props(default)] disabled: bool,
    #[props(default)] indeterminate: bool,
    #[props(default)] label: Option<String>,
    #[props(default)] description: Option<String>,
    #[props(default)] class: String,
) -> Element {
    let id = crate::common::use_unique_id("checkbox");
    let has_description = description.is_some();
    let has_label = label.is_some();
    let show_mark = checked || indeterminate;
    let description_id = format!("{id}-description");
    let label_id = format!("{id}-label");
    let path = if indeterminate {
        "M6 12h12"
    } else {
        "M5 13l4 4L19 7"
    };
    let described_by = description.as_ref().map(|_| description_id.clone());
    rsx! {
        label {
            r#for: "{id}",
            class: cn!(
                "inline-flex gap-3",
                if has_description { "items-start" } else { "items-center" },
                if disabled { "cursor-not-allowed" } else { "cursor-pointer" },
                class,
            ),
            button {
                id: "{id}",
                r#type: "button",
                role: "checkbox",
                "aria-checked": if indeterminate { "mixed" } else if checked { "true" } else { "false" },
                "aria-labelledby": if has_description && has_label { Some(label_id.clone()) } else { None },
                "aria-describedby": described_by,
                disabled,
                onclick: move |_| {
                    if !disabled {
                        on_checked_change.call(!checked);
                    }
                },
                "data-state": if checked { "checked" } else if indeterminate { "indeterminate" } else { "unchecked" },
                class: cn!(
                    "inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-md border-2 outline-none transition-colors duration-200",
                    "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    "disabled:cursor-not-allowed disabled:opacity-60 active:scale-[0.92]",
                    if show_mark { "border-primary bg-primary text-primary-foreground" } else { "border-muted-foreground/50 bg-background hover:border-muted-foreground" },
                ),
                if show_mark {
                    svg {
                        width: "12", height: "12", view_box: "0 0 24 24", fill: "none",
                        stroke: "currentColor", stroke_width: "3", "stroke-linecap": "round", "stroke-linejoin": "round",
                        "aria-hidden": "true",
                        path { d: "{path}" }
                    }
                }
            }
            if has_label || has_description {
                span {
                    class: cn!("flex min-w-0 flex-col gap-0.5 select-none", if disabled { "opacity-60" } else { "" }),
                    if let Some(label) = label {
                        span {
                            id: "{label_id}",
                            class: cn!("text-sm text-foreground", if has_description { "leading-4" } else { "" }),
                            "{label}"
                        }
                    }
                    if let Some(description) = description {
                        span { id: "{description_id}", class: "text-xs text-muted-foreground", "{description}" }
                    }
                }
            }
        }
    }
}

/// `RadioGroup` context.
#[derive(Clone, Copy)]
struct RadioContext {
    value: Signal<String>,
    controlled: bool,
    on_change: Option<EventHandler<String>>,
}

/// `RadioGroup`.
#[component]
pub fn RadioGroup(
    #[props(default)] value: Option<String>,
    #[props(default)] default_value: String,
    #[props(default)] on_value_change: Option<EventHandler<String>>,
    #[props(default = RadioOrientation::Vertical)] orientation: RadioOrientation,
    #[props(default)] class: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut internal = use_signal(|| value.clone().unwrap_or(default_value));
    let controlled = value.is_some();
    use_effect(use_reactive((&value,), move |(value,)| {
        if let Some(value) = value {
            internal.set(value);
        }
    }));
    use_context_provider(|| RadioContext {
        value: internal,
        controlled,
        on_change: on_value_change,
    });
    rsx! {
        div {
            role: "radiogroup",
            class: cn!(
                "flex gap-3",
                if orientation == RadioOrientation::Vertical { "flex-col" } else { "flex-row flex-wrap" },
                class,
            ),
            ..attributes,
            {children}
        }
    }
}

/// `RadioGroupProps["orientation"]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RadioOrientation {
    #[default]
    Vertical,
    Horizontal,
}

/// `RadioGroupItem`.
#[component]
pub fn RadioGroupItem(
    value: String,
    #[props(default)] label: Option<String>,
    #[props(default)] description: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] class: String,
) -> Element {
    let ctx = use_context::<RadioContext>();
    let id = crate::common::use_unique_id("radio");
    let has_description = description.is_some();
    let has_label = label.is_some();
    let selected = (ctx.value)() == value;
    let description_id = format!("{id}-description");
    let label_id = format!("{id}-label");
    let described_by = description.as_ref().map(|_| description_id.clone());
    let next = value.clone();
    rsx! {
        label {
            r#for: "{id}",
            class: cn!(
                "inline-flex gap-3",
                if has_description { "items-start" } else { "items-center" },
                if disabled { "cursor-not-allowed" } else { "cursor-pointer" },
                class,
            ),
            button {
                id: "{id}",
                r#type: "button",
                role: "radio",
                "aria-checked": if selected { "true" } else { "false" },
                "aria-labelledby": if has_description && has_label { Some(label_id.clone()) } else { None },
                "aria-describedby": described_by,
                disabled,
                onclick: move |_| {
                    if !disabled {
                        if !ctx.controlled {
                            ctx.value.clone().set(next.clone());
                        }
                        if let Some(handler) = ctx.on_change {
                            handler.call(next.clone());
                        }
                    }
                },
                "data-state": if selected { "checked" } else { "unchecked" },
                class: cn!(
                    "relative inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-2 outline-none transition-colors duration-200",
                    "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    "disabled:cursor-not-allowed disabled:opacity-60 active:scale-[0.92]",
                    if selected { "border-primary" } else { "border-muted-foreground/50 hover:border-muted-foreground" },
                ),
                if selected {
                    span { class: "absolute inset-1 rounded-full bg-primary" }
                }
            }
            if has_label || has_description {
                span {
                    class: cn!("flex min-w-0 flex-col gap-0.5 select-none", if disabled { "opacity-60" } else { "" }),
                    if let Some(label) = label {
                        span {
                            id: "{label_id}",
                            class: cn!("text-sm text-foreground", if has_description { "leading-4" } else { "" }),
                            "{label}"
                        }
                    }
                    if let Some(description) = description {
                        span { id: "{description_id}", class: "text-xs text-muted-foreground", "{description}" }
                    }
                }
            }
        }
    }
}

/// `Input` — the field wrapper plus `<input>`, minus the shake/success choreography.
#[component]
pub fn Input(
    #[props(default)] value: Option<String>,
    #[props(default)] default_value: String,
    #[props(default)] disabled: bool,
    #[props(default)] placeholder: String,
    #[props(default)] on_change: Option<EventHandler<String>>,
    #[props(default)] class: String,
    #[props(default)] field_class: String,
    #[props(default)] input_class: String,
) -> Element {
    let mut internal = use_signal(|| value.clone().unwrap_or(default_value));
    let controlled = value.is_some();
    let current = value.clone().unwrap_or(internal());
    rsx! {
        div { class: cn!("flex flex-col gap-1.5", class),
            div {
                "data-state": "idle",
                class: cn!(
                    "relative h-11 overflow-hidden rounded-full border transition-colors duration-200 border-border",
                    if disabled { "opacity-60" } else { "" },
                    field_class,
                ),
                input {
                    r#type: "text",
                    value: "{current}",
                    disabled,
                    placeholder: "{placeholder}",
                    oninput: move |event| {
                        let next = event.value();
                        if !controlled {
                            internal.set(next.clone());
                        }
                        if let Some(handler) = on_change {
                            handler.call(next);
                        }
                    },
                    class: cn!(
                        "peer h-full w-full bg-transparent text-base leading-6 text-foreground caret-foreground outline-none",
                        "placeholder:text-muted-foreground/60 pl-3.5 pr-3.5",
                        if disabled { "cursor-not-allowed" } else { "" },
                        input_class,
                    ),
                }
            }
        }
    }
}

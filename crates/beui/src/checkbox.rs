//! `motion/checkbox.tsx` — the checkbox and its check mark.
//!
//! Two animations: the control's `whileTap` press (`0.92` on `SPRING_PRESS`) and
//! the mark. The mark is the interesting one — it fades and scales in, draws its
//! path with a `pathLength` sweep, and on the way out fades, shrinks and blurs.
//! The mark key changes with the state (`checked` / `indeterminate`), so
//! switching states exits one mark while the next enters, which is what
//! `AnimatePresence` is here for.

use dioxus::prelude::*;
use dioxus_motion::prelude::{AnimationConfig, Duration};

use beui_core::cn;
use beui_core::ease::SPRING_PRESS;
use beui_core::hooks::use_reduced_motion;
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};

use crate::motion::{
    animate_number, number, scale_style, spring, tween, use_number_motion, Ease, MotionStyle,
};

const CHECK_PATH: &str = "M5 13l4 4L19 7";
const INDETERMINATE_PATH: &str = "M6 12h12";

#[component]
pub fn Checkbox(
    checked: bool,
    on_checked_change: EventHandler<bool>,
    #[props(default)] disabled: bool,
    #[props(default)] indeterminate: bool,
    #[props(default)] label: Option<String>,
    #[props(default)] description: Option<String>,
    #[props(default)] class: Option<String>,
    #[props(default)] id: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
) -> Element {
    let auto_id = use_hook(|| format!("beui-checkbox-{}", platform::next_scope_id()));
    let id = id.unwrap_or(auto_id);
    let reduce = use_reduced_motion();
    let mut scale = use_number_motion(1.0);

    let show_mark = checked || indeterminate;
    let mark_key = if indeterminate {
        "indeterminate"
    } else {
        "checked"
    };
    let description_id = description.as_ref().map(|_| format!("{id}-description"));
    // The wrapping <label> would fold the description into the name, so name
    // from the label text alone.
    let label_id = if description.is_some() && label.is_some() && aria_label.is_none() {
        Some(format!("{id}-label"))
    } else {
        None
    };
    let described_by = {
        let mut parts: Vec<String> = Vec::new();
        if let Some(value) = aria_describedby.clone() {
            parts.push(value);
        }
        if let Some(value) = description_id.clone() {
            parts.push(value);
        }
        (!parts.is_empty()).then(|| parts.join(" "))
    };

    let marks: Vec<&str> = if show_mark {
        vec![mark_key]
    } else {
        Vec::new()
    };

    let release = move |_event: Event<PointerData>| {
        if !disabled && !reduce() {
            animate_number(&mut scale, 1.0, spring(SPRING_PRESS));
        }
    };

    rsx! {
        label {
            r#for: id.clone(),
            class: cn!(
                "inline-flex gap-3",
                if description.is_some() { "items-start" } else { "items-center" },
                if disabled { "cursor-not-allowed" } else { "cursor-pointer" },
                class,
            ),
            button {
                id: id.clone(),
                r#type: "button",
                role: "checkbox",
                "aria-checked": if indeterminate { "mixed" } else if checked { "true" } else { "false" },
                "aria-label": aria_label,
                "aria-labelledby": label_id.clone(),
                "aria-describedby": described_by,
                disabled,
                "data-state": if checked { "checked" } else if indeterminate { "indeterminate" } else { "unchecked" },
                style: scale_style(number(&scale)),
                class: cn!(
                    "inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-md border-2 outline-none transition-colors duration-200",
                    "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    "disabled:cursor-not-allowed disabled:opacity-60",
                    if show_mark {
                        "border-primary bg-primary text-primary-foreground"
                    } else {
                        "border-muted-foreground/50 bg-background hover:border-muted-foreground"
                    },
                ),
                onpointerdown: move |_| {
                    if !disabled && !reduce() {
                        animate_number(&mut scale, 0.92, spring(SPRING_PRESS));
                    }
                },
                onpointerup: release,
                onpointercancel: release,
                onclick: move |_| {
                    if !disabled {
                        on_checked_change.call(!checked);
                    }
                },
                AnimatePresence {
                    for key in marks {
                        CheckboxMark { key: "{key}", indeterminate }
                    }
                }
            }
            if label.is_some() || description.is_some() {
                span {
                    class: cn!(
                        "flex min-w-0 flex-col gap-0.5 select-none",
                        if disabled { "opacity-60" } else { "" },
                    ),
                    if let Some(label) = label.clone() {
                        span {
                            id: label_id.clone(),
                            // Geist sits low in a 20px line; a 16px line lifts the text onto the control.
                            class: cn!("text-sm text-foreground", if description.is_some() { "leading-4" } else { "" }),
                            "{label}"
                        }
                    }
                    if let Some(description) = description.clone() {
                        span { id: description_id.clone(), class: "text-xs text-muted-foreground", "{description}" }
                    }
                }
            }
        }
    }
}

fn mark_style(opacity: f32, scale: f32, blur: Option<f32>) -> MotionStyle {
    let mut style = MotionStyle {
        opacity,
        scale,
        ..Default::default()
    };
    if let Some(px) = blur {
        style.add_css_property("filter", format!("blur({px}px)"));
    }
    style
}

fn mark_config(indeterminate: bool, reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween(0, Ease::Out),
            tween(0, Ease::Out),
        );
    }
    let _ = indeterminate;
    PresenceConfig::with_transitions(
        mark_style(0.0, 0.5, None),
        mark_style(1.0, 1.0, Some(0.0)),
        mark_style(0.0, 0.5, Some(4.0)),
        tween(160, Ease::Out),
        tween(160, Ease::Out),
    )
}

/// The check mark itself. It only exists while a mark should show; the presence
/// boundary keeps it around for the exit.
#[component]
fn CheckboxMark(indeterminate: bool) -> Element {
    let reduce = use_reduced_motion();
    let gate = use_gate();
    let style =
        use_presence_style(mark_config(indeterminate, reduce())).expect("static presence config");
    let css = presence_css(&style);

    let mut draw = use_number_motion(if reduce() { 1.0 } else { 0.0 });
    use_effect(move || {
        if reduce() {
            return;
        }
        let duration = if indeterminate { 200 } else { 300 };
        let config: AnimationConfig =
            tween(duration, Ease::Out).with_delay(Duration::from_millis(40));
        animate_number(&mut draw, 1.0, config);
    });

    let path = if indeterminate {
        INDETERMINATE_PATH
    } else {
        CHECK_PATH
    };
    let offset = 1.0 - number(&draw);

    rsx! {
        svg {
            width: "12",
            height: "12",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "3",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            style: gate.style_with(&css),
            title { if indeterminate { "Partially selected" } else { "Selected" } }
            path {
                d: path,
                path_length: "1",
                style: "stroke-dasharray:1;stroke-dashoffset:{offset}",
            }
        }
    }
}

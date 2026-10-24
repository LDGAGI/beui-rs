//! `motion/action-swap.tsx` — a control whose label/icon swaps with a blur, a
//! roll, or a per-letter cascade.
//!
//! The source uses `AnimatePresence` around a keyed `motion.span`; here the
//! incoming and outgoing layers are two keyed components, each of which starts
//! at its own variant's initial values and animates once on mount. That is the
//! same enter/exit pair without a per-swap presence boundary, and it means a
//! layer never has to be reset mid-flight — a swap that arrives while one is
//! running simply mounts a fresh pair.
//!
//! Cascade rolls the label one letter at a time, left to right. Leaving and
//! landing strings overlap as independent layers (no shared cells), so
//! proportional glyph widths never jitter. Exits cascade at half the enter
//! stagger so the tail of the old label lingers briefly.

use crate::prelude::*;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

const SWAP_BLUR: f64 = 8.0;
const ROLL_BLUR: f64 = 3.0;
const CASCADE_STAGGER: f64 = 0.025;

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The CSS `easeInOut` keyword as an `easer`-shaped function.
fn ease_in_out_keyword_easer(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if d == 0.0 {
        return b + c;
    }
    b + c * beui_core::ease::cubic_bezier_value(0.42, 0.0, 0.58, 1.0, f64::from(t / d)) as f32
}

/// Which animation a swap uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionSwapAnimation {
    #[default]
    Blur,
    Roll,
    Cascade,
}

/// Button surface variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionSwapButtonVariant {
    Primary,
    #[default]
    Secondary,
    Outline,
    Ghost,
}

/// Button sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionSwapButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
    Icon,
}

/// One item a button can cycle through.
#[derive(Clone, PartialEq)]
pub struct ActionSwapItem {
    pub id: String,
    pub label: String,
    pub icon: Option<Element>,
    pub aria_label: Option<String>,
}

impl ActionSwapItem {
    /// An item with a label and no icon.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            aria_label: None,
        }
    }

    /// Attach an icon.
    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Attach an aria label.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SwapMode {
    Enter,
    Exit,
}

fn variant_class(variant: ActionSwapButtonVariant) -> &'static str {
    match variant {
        ActionSwapButtonVariant::Primary => {
            "bg-primary text-primary-foreground hover:bg-primary/90"
        }
        ActionSwapButtonVariant::Secondary => {
            "border border-border bg-card text-foreground hover:border-border"
        }
        ActionSwapButtonVariant::Outline => {
            "border border-border bg-transparent text-foreground hover:bg-muted/60"
        }
        ActionSwapButtonVariant::Ghost => {
            "text-muted-foreground hover:bg-muted/60 hover:text-foreground"
        }
    }
}

fn size_class(size: ActionSwapButtonSize) -> &'static str {
    match size {
        ActionSwapButtonSize::Sm => "h-8 gap-1.5 rounded-full px-3 text-xs",
        ActionSwapButtonSize::Md => "h-10 gap-2 rounded-full px-4 text-sm",
        ActionSwapButtonSize::Lg => "h-12 gap-2.5 rounded-full px-5 text-base",
        ActionSwapButtonSize::Icon => "h-10 w-10 rounded-full",
    }
}

/// One text layer (blur or roll), keyed by the value it shows.
#[component]
fn SwapTextLayer(
    label: String,
    animation: ActionSwapAnimation,
    reduce: bool,
    mode: SwapMode,
    class: Option<String>,
) -> Element {
    let mut progress = use_motion(if mode == SwapMode::Enter {
        0.0f32
    } else {
        1.0f32
    })
    .expect("0/1 are finite");
    use_effect(move || {
        let target = if mode == SwapMode::Enter { 1.0 } else { 0.0 };
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            match animation {
                ActionSwapAnimation::Roll if mode == SwapMode::Exit => {
                    beui_core::ease::tween_ease_out(140)
                }
                ActionSwapAnimation::Roll => AnimationConfig::spring(SPRING_SWAP),
                _ => AnimationConfig::new(dioxus_motion::prelude::AnimationMode::Tween(
                    dioxus_motion::prelude::Tween::new(dioxus_motion::Duration::from_millis(200))
                        .with_easing(ease_in_out_keyword_easer),
                )),
            }
        };
        let _ = progress.animate_to(target, config);
    });

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if reduce {
        format!("opacity:{p};transform:none;filter:none")
    } else if animation == ActionSwapAnimation::Roll {
        let offset = if mode == SwapMode::Enter { 90.0 } else { -90.0 } * (1.0 - p);
        format!(
            "opacity:{p};transform:translateY({offset}%);filter:blur({}px)",
            ROLL_BLUR * (1.0 - p)
        )
    } else {
        format!(
            "opacity:{p};transform:scale({});filter:blur({}px)",
            0.94 + 0.06 * p,
            SWAP_BLUR * (1.0 - p)
        )
    };
    let class = cn!(
        "absolute left-0 top-[0.08em] inline-block max-w-full truncate will-change-[opacity,filter,transform]",
        class,
    );

    rsx! {
        span { class, style, "{label}" }
    }
}

/// One letter of a cascade layer.
#[component]
fn CascadeLetter(character: String, index: usize, reduce: bool, mode: SwapMode) -> Element {
    let mut progress = use_motion(if mode == SwapMode::Enter {
        0.0f32
    } else {
        1.0f32
    })
    .expect("0/1 are finite");
    use_effect(move || {
        let target = if mode == SwapMode::Enter { 1.0 } else { 0.0 };
        let delay = index as f64 * CASCADE_STAGGER;
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else if mode == SwapMode::Enter {
            AnimationConfig::spring(SPRING_SWAP).with_delay(dioxus_motion::Duration::from_millis(
                (delay * 1000.0).round() as u64,
            ))
        } else {
            beui_core::ease::tween_ease_out(160).with_delay(dioxus_motion::Duration::from_millis(
                (delay * 0.5 * 1000.0).round() as u64,
            ))
        };
        let _ = progress.animate_to(target, config);
    });

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if reduce {
        format!("opacity:{p};transform:none;filter:none")
    } else {
        let offset = if mode == SwapMode::Enter {
            105.0
        } else {
            -105.0
        } * (1.0 - p);
        format!(
            "opacity:{p};transform:translateY({offset}%);filter:blur({}px)",
            ROLL_BLUR * (1.0 - p)
        )
    };

    rsx! {
        span {
            class: "inline-block whitespace-pre will-change-[opacity,filter,transform]",
            style,
            "{character}"
        }
    }
}

/// A whole cascade layer, one letter per child.
#[component]
fn CascadeLayer(label: String, reduce: bool, mode: SwapMode) -> Element {
    rsx! {
        span {
            aria_hidden: "true",
            class: "absolute left-0 top-[0.08em] inline-block whitespace-pre",
            for (index, character) in label.chars().enumerate() {
                CascadeLetter {
                    key: "{index}",
                    character: character.to_string(),
                    index,
                    reduce,
                    mode,
                }
            }
        }
    }
}

/// Props for [`ActionSwapText`].
#[derive(Props, Clone, PartialEq)]
pub struct ActionSwapTextProps {
    /// Changing it swaps the label.
    pub value: String,
    /// The label to show.
    pub label: String,
    #[props(default)]
    pub animation: ActionSwapAnimation,
    #[props(default)]
    pub class: Option<String>,
}

/// A standalone label that swaps when `value` changes.
#[component]
pub fn ActionSwapText(props: ActionSwapTextProps) -> Element {
    let ActionSwapTextProps {
        value,
        label,
        animation,
        class,
    } = props;
    let reduce = use_reduced_motion();

    let mut current = use_signal(|| (value.clone(), label.clone()));
    let mut outgoing = use_signal(|| None::<(String, String)>);
    let next_label = label.clone();
    use_effect(use_reactive((&value,), move |(value,)| {
        if current().0 != value {
            let previous = current();
            outgoing.set(Some(previous));
            current.set((value, next_label.clone()));
        }
    }));

    let cascade = animation == ActionSwapAnimation::Cascade && !reduce();
    let class = cn!(
        "relative -my-[0.08em] inline-block max-w-full whitespace-nowrap py-[0.08em] align-bottom",
        class,
    );

    // Keys belong on the first node of a block, so the incoming/outgoing pair is
    // a keyed iterator — one node per iteration — rather than two static roots.
    let mut layers: Vec<(String, String, SwapMode)> = Vec::new();
    if let Some((key, previous)) = outgoing() {
        layers.push((format!("out-{key}"), previous, SwapMode::Exit));
    }
    let (current_key, current_label) = current();
    layers.push((format!("in-{current_key}"), current_label, SwapMode::Enter));

    let slot = label.clone();
    let reduce_value = reduce();

    rsx! {
        span {
            class,
            style: "clip-path:inset(0 -999px);-webkit-clip-path:inset(0 -999px)",
            span { aria_hidden: "true", class: "invisible inline-block whitespace-nowrap",
                if cascade {
                    for (index, character) in slot.chars().enumerate() {
                        span { key: "{index}", class: "inline-block whitespace-pre", "{character}" }
                    }
                } else {
                    "{slot}"
                }
            }
            if cascade {
                span { class: "sr-only", "{slot}" }
            }
            for (key, layer_label, mode) in layers {
                if cascade {
                    CascadeLayer { key: "{key}", label: layer_label, reduce: reduce_value, mode }
                } else {
                    SwapTextLayer { key: "{key}", label: layer_label, animation, reduce: reduce_value, mode, class: None }
                }
            }
        }
    }
}

/// One icon layer.
#[component]
fn SwapIconLayer(
    animation: ActionSwapAnimation,
    reduce: bool,
    mode: SwapMode,
    children: Element,
) -> Element {
    let mut progress = use_motion(if mode == SwapMode::Enter {
        0.0f32
    } else {
        1.0f32
    })
    .expect("0/1 are finite");
    use_effect(move || {
        let target = if mode == SwapMode::Enter { 1.0 } else { 0.0 };
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else if animation == ActionSwapAnimation::Roll {
            if mode == SwapMode::Exit {
                beui_core::ease::tween_ease_out(140)
            } else {
                AnimationConfig::spring(SPRING_SWAP)
            }
        } else {
            beui_core::ease::tween_ease_out(200)
        };
        let _ = progress.animate_to(target, config);
    });

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if reduce {
        format!("opacity:{p};transform:none;filter:none")
    } else if animation == ActionSwapAnimation::Roll {
        let offset = if mode == SwapMode::Enter { 12.0 } else { -12.0 } * (1.0 - p);
        format!(
            "opacity:{p};transform:translateY({offset}px);filter:blur({}px)",
            ROLL_BLUR * (1.0 - p)
        )
    } else {
        format!(
            "opacity:{p};transform:scale({});filter:blur({}px)",
            0.25 + 0.75 * p,
            SWAP_BLUR * (1.0 - p)
        )
    };

    rsx! {
        span {
            aria_hidden: "true",
            class: "col-start-1 row-start-1 inline-flex items-center justify-center will-change-[opacity,filter,transform]",
            style,
            {children}
        }
    }
}

/// Props for [`ActionSwapIcon`].
#[derive(Props, Clone, PartialEq)]
pub struct ActionSwapIconProps {
    pub value: String,
    pub children: Element,
    #[props(default)]
    pub animation: ActionSwapAnimation,
    #[props(default)]
    pub class: Option<String>,
}

/// A single icon that swaps when `value` changes.
#[component]
pub fn ActionSwapIcon(props: ActionSwapIconProps) -> Element {
    let ActionSwapIconProps {
        value,
        children,
        animation,
        class,
    } = props;
    let reduce = use_reduced_motion();
    let animation = if animation == ActionSwapAnimation::Cascade {
        ActionSwapAnimation::Roll
    } else {
        animation
    };

    // Icons are single elements; each value change mounts a fresh pair, keyed by
    // the value, so the outgoing icon is a snapshot of the previous child.
    let mut current = use_signal(|| (value.clone(), children.clone()));
    let mut outgoing = use_signal(|| None::<(String, Element)>);
    let next_children = children.clone();
    use_effect(use_reactive((&value,), move |(value,)| {
        if current().0 != value {
            let previous = current();
            outgoing.set(Some(previous));
            current.set((value, next_children.clone()));
        }
    }));

    let class = cn!(
        "relative inline-grid shrink-0 place-items-center overflow-hidden",
        class,
    );

    let mut layers: Vec<(String, SwapMode, Element)> = Vec::new();
    if let Some((key, icon)) = outgoing() {
        layers.push((format!("out-{key}"), SwapMode::Exit, icon));
    }
    let (current_key, current_icon) = current();
    layers.push((format!("in-{current_key}"), SwapMode::Enter, current_icon));
    let reduce_value = reduce();

    rsx! {
        span { class,
            for (key, mode, icon) in layers {
                SwapIconLayer { key: "{key}", animation, reduce: reduce_value, mode, {icon} }
            }
        }
    }
}

/// Props for [`ActionSwapButton`].
#[derive(Props, Clone, PartialEq)]
pub struct ActionSwapButtonProps {
    pub items: Vec<ActionSwapItem>,
    #[props(default)]
    pub value: Option<String>,
    #[props(default)]
    pub default_value: Option<String>,
    #[props(default)]
    pub on_value_change: Option<EventHandler<ActionSwapItem>>,
    #[props(default)]
    pub variant: ActionSwapButtonVariant,
    #[props(default)]
    pub size: ActionSwapButtonSize,
    #[props(default)]
    pub animation: ActionSwapAnimation,
    #[props(default)]
    pub icon_only: bool,
    /// Cycle to the next item on click. Defaults to `true`.
    #[props(default = true)]
    pub cycle: bool,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub disabled: bool,
}

/// A button whose label and icon swap as it cycles through items.
#[component]
pub fn ActionSwapButton(props: ActionSwapButtonProps) -> Element {
    let ActionSwapButtonProps {
        items,
        value,
        default_value,
        on_value_change,
        variant,
        size,
        animation,
        icon_only,
        cycle,
        class,
        disabled,
    } = props;

    let reduce = use_reduced_motion();
    let icon_only = icon_only || size == ActionSwapButtonSize::Icon;
    let mut internal = use_signal(|| {
        default_value
            .clone()
            .or_else(|| items.first().map(|item| item.id.clone()))
    });
    let current_value = match &value {
        Some(value) => Some(value.clone()),
        None => internal(),
    };
    let active_index = items
        .iter()
        .position(|item| Some(&item.id) == current_value.as_ref())
        .unwrap_or(0);
    let Some(active_item) = items.get(active_index) else {
        return VNode::empty();
    };
    let has_icon = items.iter().any(|item| item.icon.is_some());
    let next_item = if cycle && !items.is_empty() {
        items.get((active_index + 1) % items.len()).cloned()
    } else {
        None
    };

    let mut scale = use_motion(1.0f32).expect("1.0 is finite");

    let accessible_label = active_item
        .aria_label
        .clone()
        .or_else(|| (icon_only).then(|| active_item.label.clone()));

    let class = cn!(
        "inline-flex items-center justify-center overflow-hidden font-medium transition-colors",
        "disabled:pointer-events-none disabled:opacity-50",
        variant_class(variant),
        size_class(size),
        class,
    );
    let style = format!("transform:scale({})", current(scale));
    let active_id = active_item.id.clone();

    rsx! {
        button {
            r#type: "button",
            disabled,
            class,
            style,
            aria_label: accessible_label,
            onpointerdown: move |_| {
                if !reduce() && !disabled {
                    let _ = scale.animate_to(0.97, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
            },
            onpointerleave: move |_| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
            },
            onclick: move |_| {
                if disabled || !cycle {
                    return;
                }
                if let Some(next) = &next_item {
                    if value.is_none() {
                        internal.set(Some(next.id.clone()));
                    }
                    if let Some(handler) = &on_value_change {
                        handler.call(next.clone());
                    }
                }
            },
            if has_icon {
                ActionSwapIcon {
                    value: active_id.clone(),
                    animation,
                    class: "h-4 w-4",
                    {active_item.icon.clone().unwrap_or_else(|| rsx! {})}
                }
            }
            if !icon_only {
                ActionSwapText { value: active_id, label: active_item.label.clone(), animation }
            }
        }
    }
}

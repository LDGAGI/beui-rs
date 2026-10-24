//! `components/motion/alert.tsx` — a status banner that exits before it unmounts.
//!
//! The surface is a presence child (`AnimatePresence` + the gate), so when
//! `open` flips false the node stays mounted for the exit and is inert from the
//! first commit of it. Each variant has its own icon, and switching variants
//! crossfades the glyph through a `popLayout` presence — the old icon is popped
//! out of layout while the new one takes its place, which is what the source's
//! nested `AnimatePresence mode="popLayout"` does.
//!
//! Deviations from the source:
//!
//! - the source gives opacity (0.18 s) and transform (0.2 s) different tween
//!   durations; a `PresenceConfig` has one enter transition, so both use 0.18 s;
//! - `layout` on the surface and the row is one `beui-core::flip` on the row
//!   (position mode), not motion's nested projection — a scaled parent would
//!   distort its children and this port has no nested projection yet;
//! - `returnFocusRef` becomes `return_focus`, a CSS selector, because Dioxus has
//!   no cross-component element ref.

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use crate::icons::{CircleAlertIcon, CircleCheckIcon, InfoIcon, TriangleAlertIcon, XIcon};
use crate::motion::Slot;
use beui_core::ease::{tween_ease_out, EASE_OUT_CSS, SPRING_LAYOUT, SPRING_PRESS};
use beui_core::flip::{use_flip, FlipMode, FlipOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};
use beui_core::{cn, use_reduced_motion};

/// `AlertVariant` — the five banners. `Default` is the plain neutral surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertVariant {
    #[default]
    Default,
    Info,
    Success,
    Warning,
    Destructive,
}

impl AlertVariant {
    /// The `data-variant` value and the icon key.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Destructive => "destructive",
        }
    }

    /// `VARIANT_CLASS` from the source, verbatim.
    const fn class(self) -> &'static str {
        match self {
            Self::Default => "bg-background text-foreground",
            Self::Info => "bg-primary/5 text-primary",
            Self::Success => "bg-emerald-500/5 text-emerald-700 dark:text-emerald-400",
            Self::Warning => "bg-amber-500/5 text-amber-800 dark:text-amber-300",
            Self::Destructive => "bg-destructive/5 text-destructive",
        }
    }

    /// The default `role`: an alert for the two assertive variants, a status
    /// otherwise — exactly the source's ternary.
    const fn default_role(self) -> &'static str {
        match self {
            Self::Warning | Self::Destructive => "alert",
            _ => "status",
        }
    }
}

/// `AlertState` — visibility and variant, plus the two actions.
#[derive(Clone, Copy)]
pub struct AlertState {
    pub open: bool,
    pub variant: AlertVariant,
    set_open: EventHandler<bool>,
    dismiss: EventHandler<()>,
}

impl AlertState {
    /// `setOpen(open)`.
    pub fn set_open(&self, open: bool) {
        self.set_open.call(open);
    }

    /// `dismiss()`.
    pub fn dismiss(&self) {
        self.dismiss.call(());
    }
}

#[derive(Clone, Copy)]
struct AlertContext {
    state: Signal<AlertState>,
}

/// `useAlert()` — shared visibility and variant for custom alert children.
///
/// Must be called inside an [`Alert`]; panics otherwise, like the source.
pub fn use_alert() -> AlertState {
    let context = use_context::<AlertContext>();
    (context.state)()
}

fn ctx() -> AlertContext {
    use_context::<AlertContext>()
}

/// The surface's enter/exit. Enter fades in over the transform's 0.18 s (the
/// source splits 0.18/0.2 across two properties); exit is the source's 0.12 s.
fn surface_config(reduce: bool) -> PresenceConfig {
    let shift = if reduce { 0.0 } else { -6.0 };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, y: shift },
        motion_style! { opacity: 1.0, y: 0.0 },
        motion_style! { opacity: 0.0, y: shift },
        tween_ease_out(180),
        tween_ease_out(120),
    )
}

/// The glyph crossfade: up-side down and back, 0.18 s in and 0.1 s out.
fn icon_config(reduce: bool) -> PresenceConfig {
    let (from, out, scale) = if reduce {
        (0.0, 0.0, 1.0)
    } else {
        (4.0, -4.0, 0.9)
    };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, y: from, scale: scale },
        motion_style! { opacity: 1.0, y: 0.0, scale: 1.0 },
        motion_style! { opacity: 0.0, y: out, scale: scale },
        tween_ease_out(180),
        tween_ease_out(100),
    )
}

/// `Alert`.
#[component]
pub fn Alert(
    #[props(default)] variant: AlertVariant,
    /// Controlled visibility. `None` means uncontrolled (`default_open`).
    #[props(default = None)]
    open: Option<bool>,
    #[props(default = true)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    /// `returnFocusRef`: focused when closing hides the focused control. A CSS
    /// selector — Dioxus has no ref to another component's element.
    #[props(default = None)]
    return_focus: Option<String>,
    #[props(default = None)] content_class: Option<String>,
    #[props(default = None)] class: Option<String>,
    /// Defaults to `alert` for warning/destructive and `status` otherwise.
    #[props(default = None)]
    role: Option<String>,
    #[props(default = None)] style: Option<String>,
    children: Element,
) -> Element {
    let mut internal_open = use_signal(|| default_open);
    let controlled = open.is_some();
    let fallback_open = internal_open();
    let current_open = open.unwrap_or(fallback_open);
    let reduce = use_reduced_motion()();

    let set_open = EventHandler::new(move |next: bool| {
        if next == current_open {
            return;
        }
        if !controlled {
            internal_open.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
    });
    let dismiss = EventHandler::new(move |_| set_open.call(false));

    // The context is provided once, so it holds a `Signal` rather than a value:
    // a later `variant` prop change has to reach `AlertIcon` through the signal.
    let mut state = use_signal(|| AlertState {
        open: current_open,
        variant,
        set_open,
        dismiss,
    });
    if state.peek().open != current_open || state.peek().variant != variant {
        state.set(AlertState {
            open: current_open,
            variant,
            set_open,
            dismiss,
        });
    }
    use_context_provider(|| AlertContext { state });

    // Focus return: only when focus was actually inside the surface, exactly the
    // source's `surfaceRef.current?.contains(document.activeElement)` guard.
    let surface_id = use_hook(|| format!("beui-alert-{}", platform::next_scope_id()));
    let return_focus = use_signal(|| return_focus);
    let effect_surface_id = surface_id.clone();
    use_effect(move || {
        let is_open = state().open;
        if is_open {
            return;
        }
        let Some(selector) = return_focus() else {
            return;
        };
        let id = platform::js_string(&effect_surface_id);
        let target = platform::js_string(&selector);
        platform::fire(&format!(
            "(() => {{ const s = document.getElementById({id}); \
             if (s && s.contains(document.activeElement)) \
             document.querySelector({target})?.focus({{ preventScroll: true }}); }})()"
        ));
    });

    let role = role.unwrap_or_else(|| variant.default_role().to_string());
    let layers: Vec<AlertVariant> = if current_open {
        vec![variant]
    } else {
        Vec::new()
    };
    let layer_children = children;

    rsx! {
        AnimatePresence {
            initial: false,
            for key in layers {
                AlertSurface {
                    key: "{key.as_str()}",
                    variant: key,
                    role: role.clone(),
                    class: class.clone(),
                    style: style.clone(),
                    content_class: content_class.clone(),
                    surface_id: surface_id.clone(),
                    reduce,
                    children: layer_children.clone(),
                }
            }
        }
    }
}

/// One presence child: the animated surface plus its inner row.
#[component]
fn AlertSurface(
    variant: AlertVariant,
    role: String,
    class: Option<String>,
    style: Option<String>,
    content_class: Option<String>,
    surface_id: String,
    reduce: bool,
    children: Element,
) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(surface_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    let mut base = String::new();
    if let Some(extra) = &style {
        base.push_str(extra);
        base.push(';');
    }
    base.push_str("transform-origin:50% 0%;transition-timing-function:");
    base.push_str(EASE_OUT_CSS);
    base.push(';');
    base.push_str(&css);

    // `layout` on the row: a position-only flip so a variant with a different
    // height slides its content instead of jumping. The surface owns the enter
    // transform; the row owns the flip transform, so they never fight.
    let flip = use_flip(
        variant,
        FlipOptions::default()
            .mode(FlipMode::Position)
            .transition(if reduce {
                tween_ease_out(1)
            } else {
                AnimationConfig::spring(SPRING_LAYOUT)
            }),
    );

    rsx! {
        div {
            id: surface_id,
            role,
            "aria-hidden": if gate.is_present() { None } else { Some("true") },
            "data-slot": "alert",
            "data-variant": variant.as_str(),
            "data-state": if gate.is_present() { "open" } else { "closed" },
            inert: gate.inert_attr(),
            style: gate.style_with(&base),
            class: cn!(
                "relative w-full rounded-xl p-4 text-sm transition-colors duration-200",
                variant.class(),
                class.as_deref().unwrap_or(""),
            ),
            div {
                "data-beui-flip": flip.scope(),
                style: flip.style_with("transform-origin:50% 0%"),
                class: cn!("flex min-w-0 items-start gap-3", content_class.as_deref().unwrap_or("")),
                {children}
            }
        }
    }
}

/// `AlertIcon` — the variant glyph, or the caller's own.
#[component]
pub fn AlertIcon(#[props(default = None)] class: Option<String>, children: Element) -> Element {
    let state = (ctx().state)();
    let reduce = use_reduced_motion()();
    let variants = [state.variant];

    rsx! {
        span {
            "aria-hidden": "true",
            "data-slot": "alert-icon",
            class: cn!("mt-0.5 inline-flex size-4 shrink-0 items-center justify-center", class.as_deref().unwrap_or("")),
            Slot {
                default: rsx! {
                    AnimatePresence {
                        mode: PresenceMode::PopLayout,
                        for variant in variants {
                            AlertGlyph { key: "{variant.as_str()}", variant, reduce }
                        }
                    }
                },
                {children}
            }
        }
    }
}

#[component]
fn AlertGlyph(variant: AlertVariant, reduce: bool) -> Element {
    let handle = use_presence_style(icon_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    rsx! {
        span {
            style: css,
            class: "inline-flex [&>svg]:size-4",
            {variant_glyph(variant)}
        }
    }
}

fn variant_glyph(variant: AlertVariant) -> Element {
    match variant {
        AlertVariant::Default | AlertVariant::Info => rsx! { InfoIcon {} },
        AlertVariant::Success => rsx! { CircleCheckIcon {} },
        AlertVariant::Warning => rsx! { TriangleAlertIcon {} },
        AlertVariant::Destructive => rsx! { CircleAlertIcon {} },
    }
}

/// `AlertContent`.
#[component]
pub fn AlertContent(#[props(default = None)] class: Option<String>, children: Element) -> Element {
    rsx! {
        div {
            "data-slot": "alert-content",
            class: cn!("min-w-0 flex-1 space-y-1", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `AlertTitle`.
#[component]
pub fn AlertTitle(#[props(default = None)] class: Option<String>, children: Element) -> Element {
    rsx! {
        div {
            "data-slot": "alert-title",
            class: cn!("font-medium leading-5 text-pretty", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `AlertDescription`.
#[component]
pub fn AlertDescription(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-slot": "alert-description",
            class: cn!(
                "text-pretty leading-relaxed text-muted-foreground [&_a]:underline [&_a]:underline-offset-4",
                class.as_deref().unwrap_or(""),
            ),
            {children}
        }
    }
}

/// `AlertAction` — a flex row; never a nested button.
#[component]
pub fn AlertAction(#[props(default = None)] class: Option<String>, children: Element) -> Element {
    rsx! {
        div {
            "data-slot": "alert-action",
            class: cn!("flex flex-wrap items-center gap-2 pt-2", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `AlertClose`.
#[component]
pub fn AlertClose(
    #[props(default = None)] class: Option<String>,
    #[props(default = "Dismiss alert")] label: &'static str,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let state = (ctx().state)();
    let reduce = use_reduced_motion()();
    let scale = use_motion(1.0f32).expect("1.0 is a finite animation value");

    // A closure over two `Copy` captures is itself `Copy`, so the four pointer
    // handlers can each take their own copy.
    let press = move |to: f32| {
        if reduce {
            return;
        }
        let mut handle = scale;
        let _ = handle.animate_to(to, AnimationConfig::spring(SPRING_PRESS));
    };

    rsx! {
        button {
            r#type: "button",
            "aria-label": label,
            disabled,
            "data-slot": "alert-close",
            style: format!("transform:scale({})", scale.get_value()),
            class: cn!(
                "-m-1.5 inline-flex size-8 shrink-0 items-center justify-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-foreground/5 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
                class.as_deref().unwrap_or(""),
            ),
            onpointerdown: move |_| press(0.94),
            onpointerup: move |_| press(1.0),
            onpointerleave: move |_| press(1.0),
            onpointercancel: move |_| press(1.0),
            onclick: move |event| {
                let enabled = event.default_action_enabled();
                if let Some(callback) = onclick {
                    callback.call(event);
                }
                if enabled {
                    state.dismiss();
                }
            },
            Slot { default: rsx! { XIcon { class: "size-4" } }, {children} }
        }
    }
}

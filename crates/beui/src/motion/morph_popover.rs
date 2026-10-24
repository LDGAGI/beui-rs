//! `components/motion/popover-morph.tsx` — a panel that grows out of the trigger
//! corner.
//!
//! The panel is laid out at full size and clipped to the corner nearest the
//! trigger (`inset(... 92% ...)`); the clip tweens to zero over 0.32 s while the
//! wrapper scales on `SPRING_PANEL` and fades. It picks the roomier side when the
//! requested one cannot fit and keeps the panel inside a 12 px gutter.
//!
//! Deviations from the source:
//!
//! - the portalled surface replaces `AnimatePresence` + `usePresence` with the
//!   same presence config (`PresenceGate`), so the exit and the gate are the
//!   port's usual pair;
//! - `MorphPopoverTrigger` renders its own `<button>` (no `cloneElement`), and
//!   `onOpenAutoFocus` is not ported.

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use crate::motion::PortalHost;
use beui_core::ease::{tween_ease_out, SPRING_PANEL};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};
use beui_core::{cn, use_reduced_motion};

/// Which side of the trigger the panel opens on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MorphPopoverSide {
    #[default]
    Bottom,
    Top,
}

/// Which edge of the trigger the panel lines up with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MorphPopoverAlign {
    Start,
    #[default]
    End,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MorphLayout {
    trigger_left: f64,
    trigger_top: f64,
    trigger_width: f64,
    trigger_height: f64,
    content_width: f64,
    content_height: f64,
    viewport_width: f64,
    viewport_height: f64,
    measured: bool,
}

fn parse_layout(message: &str) -> Option<MorphLayout> {
    let mut parts = message.split('|');
    let mut next = || parts.next()?.parse::<f64>().ok();
    Some(MorphLayout {
        trigger_left: next()?,
        trigger_top: next()?,
        trigger_width: next()?,
        trigger_height: next()?,
        content_width: next()?,
        content_height: next()?,
        viewport_width: next()?,
        viewport_height: next()?,
        measured: true,
    })
}

const GUTTER: f64 = 12.0;

/// `clipAt` — everything hidden but the corner nearest the trigger.
fn clip_at(side: MorphPopoverSide, align: MorphPopoverAlign, radius: f64, inset: f64) -> String {
    let (top, bottom) = match side {
        MorphPopoverSide::Bottom => ("0%".to_string(), format!("{inset}%")),
        MorphPopoverSide::Top => (format!("{inset}%"), "0%".to_string()),
    };
    let (right, left) = match align {
        MorphPopoverAlign::End => ("0%".to_string(), format!("{inset}%")),
        MorphPopoverAlign::Start => (format!("{inset}%"), "0%".to_string()),
    };
    format!("inset({top} {right} {bottom} {left} round {radius}px)")
}

fn origin_for(side: MorphPopoverSide, align: MorphPopoverAlign) -> String {
    format!(
        "{} {}",
        match side {
            MorphPopoverSide::Bottom => "top",
            MorphPopoverSide::Top => "bottom",
        },
        match align {
            MorphPopoverAlign::End => "right",
            MorphPopoverAlign::Start => "left",
        }
    )
}

#[derive(Clone, Copy)]
struct MorphContext {
    open: Signal<bool>,
    set_open: EventHandler<bool>,
    uid: u32,
    layout: Signal<MorphLayout>,
    reduce: bool,
}

fn use_morph() -> MorphContext {
    use_context::<MorphContext>()
}

/// `MorphPopover`.
#[component]
pub fn MorphPopover(
    #[props(default = None)] open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal_open = use_signal(|| default_open);
    let controlled = open.is_some();
    let fallback = internal_open();
    let current_open = open.unwrap_or(fallback);
    let open_signal = use_signal(|| current_open);
    let mut open_signal_sync = open_signal;
    if *open_signal_sync.peek() != current_open {
        open_signal_sync.set(current_open);
    }

    let set_open = EventHandler::new(move |next: bool| {
        if !controlled {
            internal_open.set(next);
        }
        if let Some(callback) = on_open_change {
            callback.call(next);
        }
    });

    let uid = use_hook(platform::next_scope_id);
    let content_selector = format!("[data-beui-morph-content=\"{uid}\"]");

    let close = EventHandler::new(move |_| set_open.call(false));
    let scope = use_dismiss(
        open_signal,
        close,
        DismissOptions::default().ignore(content_selector),
    );

    let layout = use_signal(MorphLayout::default);
    use_context_provider(|| MorphContext {
        open: open_signal,
        set_open,
        uid,
        layout,
        reduce,
    });

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            class: cn!("relative inline-flex", class.as_deref().unwrap_or("")),
            {children}
        }
    }
}

/// `MorphPopoverTrigger` — the focusable control the panel grows out of.
#[component]
pub fn MorphPopoverTrigger(
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] aria_label: Option<String>,
    #[props(default = None)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    children: Element,
) -> Element {
    let context = use_morph();
    let open = (context.open)();

    rsx! {
        button {
            r#type: "button",
            id: "beui-morph-{context.uid}-trigger",
            "data-beui-morph-trigger": "{context.uid}",
            "aria-label": aria_label.as_deref().unwrap_or(""),
            "aria-haspopup": "dialog",
            "aria-expanded": open,
            "aria-controls": if open { Some(format!("beui-morph-{}-content", context.uid)) } else { None },
            class: cn!("relative inline-flex items-center", class.as_deref().unwrap_or("")),
            onclick: move |_| {
                let next = !(context.open)();
                context.set_open.call(next);
            },
            onkeydown: move |event| {
                if let Some(callback) = onkeydown {
                    callback.call(event);
                }
            },
            {children}
        }
    }
}

/// `MorphPopoverContent`.
#[component]
pub fn MorphPopoverContent(
    #[props(default)] side: MorphPopoverSide,
    #[props(default)] align: MorphPopoverAlign,
    /// Gap between trigger and panel, in px.
    #[props(default = 8.0)]
    side_offset: f64,
    /// Panel corner radius, in px.
    #[props(default = 16.0)]
    radius: f64,
    /// Draw the surface shadow.
    #[props(default = true)]
    shadow: bool,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_morph();
    let open = (context.open)();
    let layers: Vec<()> = if open { vec![()] } else { Vec::new() };
    let surface_children = children;

    rsx! {
        PortalHost {
            AnimatePresence {
                for _ in layers {
                    MorphPopoverSurface {
                        key: "surface-{context.uid}",
                        side,
                        align,
                        side_offset,
                        radius,
                        shadow,
                        class: class.clone(),
                        children: surface_children.clone(),
                    }
                }
            }
        }
    }
}

fn surface_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(120),
            tween_ease_out(120),
        );
    }
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, scale: 0.96 },
        motion_style! { opacity: 1.0, scale: 1.0 },
        motion_style! { opacity: 0.0, scale: 0.96 },
        AnimationConfig::spring(SPRING_PANEL),
        AnimationConfig::spring(SPRING_PANEL),
    )
}

#[component]
#[allow(clippy::too_many_arguments)]
fn MorphPopoverSurface(
    side: MorphPopoverSide,
    align: MorphPopoverAlign,
    side_offset: f64,
    radius: f64,
    shadow: bool,
    class: Option<String>,
    children: Element,
) -> Element {
    let context = use_morph();
    let uid = context.uid;
    let reduce = context.reduce;
    let gate = use_gate();
    let handle = use_presence_style(surface_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    // Start hidden, then let the CSS clip transition run on the next render.
    // Start hidden, then let the next render's motion carry the clip open.

    let trigger_selector = format!("[data-beui-morph-trigger=\"{uid}\"]");
    let content_selector = format!("[data-beui-morph-content=\"{uid}\"]");
    let layout_signal = context.layout;
    let follow_trigger = trigger_selector.clone();
    let follow_content = content_selector.clone();
    use_effect(move || {
        let script = format!(
            "{}\nwindow.__beuiPopoverStart({uid}, {}, {});",
            include_str!("../js/popover.js"),
            platform::js_string(&follow_trigger),
            platform::js_string(&follow_content),
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        let mut layout = layout_signal;
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if let Some(next) = parse_layout(&message) {
                    if *layout.peek() != next {
                        layout.set(next);
                    }
                }
            }
        });
    });
    use_drop(move || {
        platform::fire(&format!(
            "window.__beuiPopoverStop && window.__beuiPopoverStop({uid})"
        ));
    });

    let layout = (context.layout)();
    let is_present = gate.is_present();

    // The clip is a per-frame value, not a CSS transition: the mount commit and
    // a class flip can land in the same frame, and the browser coalesces them
    // into no transition at all.
    let clip_progress = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let clip_signal = clip_progress;
    let mut present_signal = use_signal(|| is_present);
    if *present_signal.peek() != is_present {
        present_signal.set(is_present);
    }
    use_effect(move || {
        let present = present_signal();
        let mut handle = clip_signal;
        let _ = handle.animate_to(if present { 1.0 } else { 0.0 }, tween_ease_out(320));
    });

    let computed = layout.measured.then(|| {
        let trigger_bottom = layout.trigger_top + layout.trigger_height;
        let below = (layout.viewport_height - trigger_bottom - side_offset - GUTTER).max(0.0);
        let above = (layout.trigger_top - side_offset - GUTTER).max(0.0);
        let requested = match side {
            MorphPopoverSide::Bottom => below,
            MorphPopoverSide::Top => above,
        };
        let other = match side {
            MorphPopoverSide::Bottom => above,
            MorphPopoverSide::Top => below,
        };
        let resolved = if layout.content_height > requested && other > requested {
            match side {
                MorphPopoverSide::Bottom => MorphPopoverSide::Top,
                MorphPopoverSide::Top => MorphPopoverSide::Bottom,
            }
        } else {
            side
        };
        let available = match resolved {
            MorphPopoverSide::Bottom => below,
            MorphPopoverSide::Top => above,
        };
        let preferred_left = match align {
            MorphPopoverAlign::End => {
                layout.trigger_left + layout.trigger_width - layout.content_width
            }
            MorphPopoverAlign::Start => layout.trigger_left,
        };
        let left = preferred_left.clamp(
            GUTTER,
            (layout.viewport_width - layout.content_width - GUTTER).max(GUTTER),
        );
        let top = match resolved {
            MorphPopoverSide::Bottom => trigger_bottom + side_offset,
            MorphPopoverSide::Top => {
                (layout.trigger_top - layout.content_height - side_offset).max(GUTTER)
            }
        };
        (resolved, available, left, top)
    });

    let clip_inset = 92.0 * (1.0 - f64::from(clip_progress.get_value()));
    let (resolved_side, available, left, top) = computed.unwrap_or((
        side,
        f64::INFINITY,
        layout.trigger_left,
        layout.trigger_top + layout.trigger_height + side_offset,
    ));
    let clip = clip_at(resolved_side, align, radius, clip_inset);
    let origin = origin_for(resolved_side, align);
    let filter = if shadow {
        "filter:drop-shadow(0 10px 18px rgba(0, 0, 0, 0.14));"
    } else {
        ""
    };
    let outer_style = format!(
        "{css};position:fixed;left:{left}px;top:{top}px;transform-origin:{origin};{filter}visibility:{}",
        if layout.measured { "visible" } else { "hidden" },
    );
    let max_height = if available.is_finite() {
        format!("max-height:{available}px;")
    } else {
        String::new()
    };
    let inner_style =
        format!("clip-path:{clip};border-radius:{radius}px;{max_height}overflow-y:auto",);

    rsx! {
        div {
            inert: gate.inert_attr(),
            style: gate.style_with(&outer_style),
            class: cn!("fixed z-[9999]"),
            div {
                id: "beui-morph-{uid}-content",
                "data-beui-morph-content": "{uid}",
                role: "dialog",
                "aria-labelledby": "beui-morph-{uid}-trigger",
                style: inner_style,
                class: cn!("overflow-hidden border border-border bg-background", class.as_deref().unwrap_or("")),
                {children}
            }
        }
    }
}

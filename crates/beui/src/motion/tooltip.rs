//! `components/motion/tooltip.tsx` — a hover/focus label.
//!
//! Positioning is geometry, not animation: a JS helper measures the anchor and
//! the surface, applies the same middleware the source configures with
//! floating-ui (offset, flip, shift, arrow, hide) at a `fixed` strategy, writes
//! the transform, and reports the placed side back over `dioxus.send` so the
//! arrow path can match.
//!
//! Deviations from the source:
//!
//! - the grace corridor is not ported: leaving the trigger closes the label
//!   rather than keeping it open while the pointer crosses to the surface;
//! - `followCursor` / `anchorPoint` / `asChild` are not ported;
//! - keyboard modality is inferred from the last pointerdown rather than the
//!   shared `watchModality` listener;
//! - the only-one-tooltip-at-a-time singleton is not ported (`skipDelay` is, via
//!   a module clock).

use std::cell::Cell;

use dioxus::prelude::*;
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

use crate::motion::PortalHost;
use beui_core::ease::tween_ease_out;
use beui_core::hooks::{use_dismiss, use_hover_gesture, use_tap_gesture, DismissOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};
use beui_core::{cn, use_reduced_motion};

thread_local! {
    /// When the last label closed — the `skipDelay` clock.
    static LAST_CLOSED_AT: Cell<f64> = const { Cell::new(-1.0e12) };
    /// When the last pointerdown landed — used to tell pointer focus from key focus.
    static LAST_POINTER_DOWN_AT: Cell<f64> = const { Cell::new(-1.0e12) };
}

/// Which side of the anchor the label sits on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipSide {
    #[default]
    Top,
    Right,
    Bottom,
    Left,
}

impl TooltipSide {
    const fn key(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
        }
    }

    const fn arrow_path(self) -> &'static str {
        match self {
            Self::Top => "M0 6L6 12L12 6",
            Self::Bottom => "M0 6L6 0L12 6",
            Self::Left => "M6 0L12 6L6 12",
            Self::Right => "M6 0L0 6L6 12",
        }
    }
}

/// Alignment along the anchor's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipAlign {
    Start,
    #[default]
    Center,
    End,
}

impl TooltipAlign {
    const fn key(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}

/// Shared timing for the tooltips inside a [`TooltipProvider`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipTiming {
    pub delay: u64,
    pub close_delay: u64,
    pub skip_delay: u64,
}

impl Default for TooltipTiming {
    fn default() -> Self {
        Self {
            delay: 120,
            close_delay: 0,
            skip_delay: 300,
        }
    }
}

/// `TooltipProvider`.
#[component]
pub fn TooltipProvider(
    #[props(default = None)] delay: Option<u64>,
    #[props(default = None)] close_delay: Option<u64>,
    #[props(default = None)] skip_delay: Option<u64>,
    children: Element,
) -> Element {
    let parent = try_consume_context::<TooltipTiming>().unwrap_or_default();
    use_context_provider(|| TooltipTiming {
        delay: delay.unwrap_or(parent.delay),
        close_delay: close_delay.unwrap_or(parent.close_delay),
        skip_delay: skip_delay.unwrap_or(parent.skip_delay),
    });
    rsx! {
        {children}
    }
}

#[derive(Clone)]
struct TooltipContext {
    open: Signal<bool>,
    set_open: EventHandler<bool>,
    empty: Signal<bool>,
    disabled: Signal<bool>,
    as_label: bool,
    trigger_id: String,
    floating_id: String,
    content_id: String,
    label_id: String,
    side: TooltipSide,
    align: TooltipAlign,
    side_offset: f64,
    arrow: bool,
    placed_side: Signal<TooltipSide>,
    timing: TooltipTiming,
    last_pointer_down: Signal<f64>,
}

fn use_tooltip() -> TooltipContext {
    use_context::<TooltipContext>()
}

/// `Tooltip` (root).
#[component]
pub fn Tooltip(
    #[props(default)] side: TooltipSide,
    #[props(default)] align: TooltipAlign,
    #[props(default = 8.0)] side_offset: f64,
    #[props(default = false)] arrow: bool,
    /// Never show, e.g. while the label it carries is already visible.
    #[props(default = false)]
    disabled: bool,
    /// Use the tooltip as the trigger's accessible name rather than its description.
    #[props(default = false)]
    as_label: bool,
    #[props(default = None)] open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default = None)] delay: Option<u64>,
    #[props(default = None)] close_delay: Option<u64>,
    children: Element,
) -> Element {
    let timing = try_consume_context::<TooltipTiming>().unwrap_or_default();
    let timing = TooltipTiming {
        delay: delay.unwrap_or(timing.delay),
        close_delay: close_delay.unwrap_or(timing.close_delay),
        skip_delay: timing.skip_delay,
    };
    let mut internal_open = use_signal(|| default_open);
    let controlled = open.is_some();
    let fallback = internal_open();
    let current_open = open.unwrap_or(fallback);
    let open_signal = use_signal(|| current_open);
    let mut open_sync = open_signal;
    if *open_sync.peek() != current_open {
        open_sync.set(current_open);
    }
    let uid = use_hook(platform::next_scope_id);
    let context = TooltipContext {
        open: open_signal,
        set_open: EventHandler::new(move |next: bool| {
            if !controlled {
                internal_open.set(next);
            }
            if let Some(callback) = on_open_change {
                callback.call(next);
            }
        }),
        empty: use_signal(|| false),
        disabled: use_signal(|| disabled),
        as_label,
        trigger_id: format!("beui-tooltip-{uid}-trigger"),
        floating_id: format!("beui-tooltip-{uid}-floating"),
        content_id: format!("beui-tooltip-{uid}-content"),
        label_id: format!("beui-tooltip-{uid}-label"),
        side,
        align,
        side_offset,
        arrow,
        placed_side: use_signal(|| side),
        timing,
        last_pointer_down: use_signal(|| -1.0e12),
    };
    {
        let mut slot = context.disabled;
        if *slot.peek() != disabled {
            slot.set(disabled);
        }
    }
    use_context_provider(|| context);

    rsx! {
        {children}
    }
}

/// `TooltipTrigger`.
#[component]
pub fn TooltipTrigger(
    #[props(default = None)] class: Option<String>,
    #[props(default = false)] disabled: bool,
    children: Element,
) -> Element {
    let context = use_tooltip();
    let open = (context.open)();
    let empty = (context.empty)();
    let context_disabled = (context.disabled)();
    let hover = use_hover_gesture();
    let tap = use_tap_gesture::<bool>();
    let hover_enter = hover.clone();
    let hover_leave = hover;
    let tap_down = tap.clone();
    let tap_click = tap.clone();
    let tap_key = tap.clone();
    let tap_cancel = tap;

    let close = {
        let context = context.clone();
        EventHandler::new(move |_| {
            LAST_CLOSED_AT.with(|clock| clock.set(platform::now_ms()));
            context.set_open.call(false);
        })
    };
    let scope = use_dismiss(
        context.open,
        close,
        DismissOptions::default().ignore(format!("#{}", context.floating_id)),
    );

    let timing = context.timing;
    let open_now = {
        let context = context.clone();
        EventHandler::new(move |_| {
            if (context.open)() || (context.disabled)() || (context.empty)() {
                return;
            }
            context.set_open.call(true);
        })
    };
    let request_open = {
        let context = context.clone();
        EventHandler::new(move |_| {
            if (context.open)() || (context.disabled)() || (context.empty)() {
                return;
            }
            let warm =
                platform::now_ms() - LAST_CLOSED_AT.with(Cell::get) < timing.skip_delay as f64;
            if warm || timing.delay == 0 {
                open_now.call(());
            } else {
                let delay = timing.delay;
                let set_open = context.set_open;
                spawn(async move {
                    if Time::delay(Duration::from_millis(delay)).await.is_ok() {
                        set_open.call(true);
                    }
                });
            }
        })
    };
    let request_close = {
        let context = context.clone();
        EventHandler::new(move |_| {
            if timing.close_delay > 0 {
                let delay = timing.close_delay;
                let set_open = context.set_open;
                spawn(async move {
                    if Time::delay(Duration::from_millis(delay)).await.is_ok() {
                        set_open.call(false);
                        LAST_CLOSED_AT.with(|clock| clock.set(platform::now_ms()));
                    }
                });
            } else {
                context.set_open.call(false);
                LAST_CLOSED_AT.with(|clock| clock.set(platform::now_ms()));
            }
        })
    };

    let trigger_disabled = disabled || context_disabled;

    rsx! {
        button {
            r#type: "button",
            id: "{context.trigger_id}",
            "data-beui-scope": scope.attr(),
            disabled: trigger_disabled,
            "data-state": if open { "open" } else { "closed" },
            "aria-labelledby": if context.as_label && !empty { Some(context.label_id.clone()) } else { None },
            "aria-describedby": if !context.as_label && open && !empty { Some(context.content_id.clone()) } else { None },
            class: cn!("relative", class.as_deref().unwrap_or("")),
            onpointerenter: move |event: Event<PointerData>| {
                if !hover_enter.enter(&event.data()) {
                    return;
                }
                request_open.call(());
            },
            onpointerleave: move |event: Event<PointerData>| {
                if !hover_leave.leave(&event.data()) {
                    return;
                }
                request_close.call(());
            },
            onpointerdown: move |event: Event<PointerData>| {
                tap_down.start(&event.data(), (context.open)());
                let mut last = context.last_pointer_down;
                last.set(platform::now_ms());
                if event.pointer_type() == "mouse" {
                    close.call(());
                }
            },
            onpointercancel: move |_| tap_cancel.drop_record(),
            onkeydown: move |_| tap_key.drop_record(),
            onclick: move |_| {
                let gesture = tap_click.take();
                let Some(gesture) = gesture else {
                    return;
                };
                if gesture.pointer_type == "mouse" {
                    return;
                }
                if gesture.state {
                    close.call(());
                } else {
                    open_now.call(());
                }
            },
            onfocusin: move |_| {
                let since_pointer = platform::now_ms() - (context.last_pointer_down)();
                if since_pointer < 100.0 {
                    return;
                }
                open_now.call(());
            },
            onfocusout: move |_| close.call(()),
            {children}
        }
    }
}

fn surface_config(reduce: bool) -> PresenceConfig {
    let scale = if reduce { 1.0 } else { 0.94 };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, scale: scale },
        motion_style! { opacity: 1.0, scale: 1.0 },
        motion_style! { opacity: 0.0, scale: scale },
        tween_ease_out(180),
        tween_ease_out(120),
    )
}

/// `TooltipContent`.
#[component]
pub fn TooltipContent(text: String, #[props(default = None)] class: Option<String>) -> Element {
    let context = use_tooltip();
    let reduce = use_reduced_motion()();
    let empty = text.is_empty();
    {
        let mut slot = context.empty;
        if *slot.peek() != empty {
            slot.set(empty);
        }
    }
    let open = (context.open)() && !empty;

    let theme_context = context.clone();
    let layer_text = text.clone();
    let layers: Vec<()> = if open { vec![()] } else { Vec::new() };

    rsx! {
        if context.as_label && !empty {
            span { id: "{context.label_id}", hidden: true, "{text}" }
        }
        PortalHost {
            AnimatePresence {
                initial: false,
                for _ in layers {
                    TooltipLayer {
                        key: "tooltip-{theme_context.trigger_id}",
                        text: layer_text.clone(),
                        class: class.clone(),
                        arrow: theme_context.arrow,
                        reduce,
                    }
                }
            }
        }
    }
}

#[component]
fn TooltipLayer(text: String, class: Option<String>, arrow: bool, reduce: bool) -> Element {
    let context = use_tooltip();
    let gate = use_gate();
    let handle = use_presence_style(surface_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);

    // Position the layer while it is open.
    let uid = use_hook(platform::next_scope_id);
    let trigger_id = context.trigger_id.clone();
    let floating_id = context.floating_id.clone();
    let side = context.side;
    let align = context.align;
    let offset = context.side_offset;
    let placed = context.placed_side;
    use_effect(move || {
        let script = format!(
            "{}\nwindow.__beuiTooltipStart({uid}, {}, {}, {{ side: {}, align: {}, offset: {}, arrow: {}, origin: {{ top: 'center bottom', bottom: 'center top', left: 'right center', right: 'left center' }} }});",
            include_str!("../js/tooltip.js"),
            platform::js_string(&format!("#{trigger_id}")),
            platform::js_string(&format!("#{floating_id}")),
            platform::js_string(side.key()),
            platform::js_string(align.key()),
            offset,
            arrow,
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        let mut placed_side = placed;
        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if let Some(rest) = message.strip_prefix("placement:") {
                    let next = match rest {
                        "bottom" => TooltipSide::Bottom,
                        "left" => TooltipSide::Left,
                        "right" => TooltipSide::Right,
                        _ => TooltipSide::Top,
                    };
                    if *placed_side.peek() != next {
                        placed_side.set(next);
                    }
                }
            }
        });
    });
    use_drop(move || {
        platform::fire(&format!(
            "window.__beuiTooltipStop && window.__beuiTooltipStop({uid})"
        ));
    });

    let side = (context.placed_side)();

    rsx! {
        span {
            id: "{context.floating_id}",
            inert: gate.inert_attr(),
            "aria-hidden": if gate.is_present() { None } else { Some("true") },
            style: "visibility:hidden;max-width:calc(100vw - 16px)",
            class: "pointer-events-none fixed left-0 top-0 z-[9999] w-max",
            span {
                id: if context.as_label { None } else { Some(context.content_id.clone()) },
                role: "tooltip",
                style: gate.style_with(&format!("{css};transform-origin:var(--tooltip-origin, center)")),
                class: cn!(
                    "block whitespace-nowrap rounded-lg border border-border bg-background px-2.5 py-1 text-xs font-medium text-foreground shadow-lg",
                    "max-w-[min(20rem,calc(100vw-16px))] whitespace-normal break-words",
                    if arrow { "relative" } else { "" },
                    class.as_deref().unwrap_or(""),
                ),
                "{text}"
                if arrow {
                    svg {
                        "data-tooltip-arrow": "",
                        "aria-hidden": "true",
                        width: "12",
                        height: "12",
                        "viewBox": "0 0 12 12",
                        class: "pointer-events-none absolute overflow-visible",
                        path {
                            d: "{side.arrow_path()}Z",
                            style: "fill:var(--tooltip-arrow-fill, transparent)",
                        }
                        path {
                            d: "{side.arrow_path()}",
                            fill: "none",
                            "stroke-width": "1",
                            style: "stroke:var(--tooltip-arrow-stroke, transparent)",
                        }
                    }
                }
            }
        }
    }
}

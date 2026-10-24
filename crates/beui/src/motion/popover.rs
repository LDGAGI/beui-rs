//! `components/motion/popover.tsx` — a panel that oozes out of its trigger.
//!
//! The trick is that the *clip*, not the panel, is what moves. `build_geo`
//! places the trigger rect and the panel rect in one local box; `progress` runs
//! 0→1 on the goo spring; and every frame both the goo body and the content are
//! clipped to the rounded rectangle interpolated between the two. A blur +
//! alpha-sharpen SVG filter over that body welds the trigger pill and the panel
//! into one liquid shape while the neck is short, and the trigger is punched
//! back out of the layer so the real label and focus ring stay visible.
//!
//! Deviations from the source:
//!
//! - the clip is always the `inset(... round r)` form; motion picks the smoother
//!   CSS `shape()` when the engine supports it (the source keeps the same
//!   fallback);
//! - `PopoverTrigger` renders its own `<button>` — Dioxus has no `cloneElement`
//!   — and the touch-tap half of the hover trigger is dropped (hover and click
//!   both work).

use dioxus::prelude::*;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationManager, Duration, Time, TimeProvider,
};

use crate::motion::spring_from_visual_duration;
use crate::motion::PortalHost;
use beui_core::ease::tween_ease_out;
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::{cn, use_reduced_motion};

/// Which side of the trigger the panel oozes out of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopoverSide {
    #[default]
    Bottom,
    Top,
}

/// Alignment along the trigger's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopoverAlign {
    Start,
    #[default]
    Center,
    End,
}

/// How the popover is summoned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopoverTriggerMode {
    #[default]
    Click,
    Hover,
}

/// A rectangle in the popover's local coordinate box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub r: f64,
}

/// The trigger rect and the panel rect, plus the box that contains both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PopoverGeo {
    pub layer_w: f64,
    pub layer_h: f64,
    pub left: f64,
    pub top: f64,
    pub trigger: Rect,
    pub panel: Rect,
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// `buildGeo` — trigger and panel in a shared local coordinate box.
#[allow(clippy::too_many_arguments)]
pub fn build_geo(
    trigger_w: f64,
    trigger_h: f64,
    content_w: f64,
    content_h: f64,
    side: PopoverSide,
    align: PopoverAlign,
    gap: f64,
    panel_radius: f64,
) -> PopoverGeo {
    let py = match side {
        PopoverSide::Bottom => trigger_h + gap,
        PopoverSide::Top => -(gap + content_h),
    };
    let px = match align {
        PopoverAlign::Start => 0.0,
        PopoverAlign::End => trigger_w - content_w,
        PopoverAlign::Center => (trigger_w - content_w) / 2.0,
    };

    let left = px.min(0.0);
    let top = py.min(0.0);
    let layer_w = (trigger_w).max(px + content_w) - left;
    let layer_h = (trigger_h).max(py + content_h) - top;

    let trigger_radius = (trigger_h / 2.0).min(panel_radius);

    PopoverGeo {
        layer_w,
        layer_h,
        left,
        top,
        trigger: Rect {
            x: -left,
            y: -top,
            w: trigger_w,
            h: trigger_h,
            r: trigger_radius,
        },
        panel: Rect {
            x: px - left,
            y: py - top,
            w: content_w,
            h: content_h,
            r: panel_radius,
        },
    }
}

fn rect_at_progress(geo: PopoverGeo, progress: f64) -> Rect {
    let trigger = geo.trigger;
    let panel = geo.panel;
    Rect {
        x: lerp(trigger.x, panel.x, progress),
        y: lerp(trigger.y, panel.y, progress),
        w: lerp(trigger.w, panel.w, progress),
        h: lerp(trigger.h, panel.h, progress),
        r: lerp(trigger.r, panel.r, progress),
    }
}

fn inset_for(rect: Rect, layer_w: f64, layer_h: f64) -> String {
    let top = rect.y;
    let right = layer_w - (rect.x + rect.w);
    let bottom = layer_h - (rect.y + rect.h);
    let left = rect.x;
    format!(
        "inset({top}px {right}px {bottom}px {left}px round {}px)",
        rect.r
    )
}

/// The morph clip: the rounded rect half-way between trigger and panel.
pub fn clip_for_progress(geo: PopoverGeo, progress: f64) -> String {
    if geo.layer_w == 0.0 {
        return "inset(0 100% 0 0)".to_string();
    }
    inset_for(rect_at_progress(geo, progress), geo.layer_w, geo.layer_h)
}

fn rounded_rect_path(rect: Rect) -> String {
    let radius = rect.r.max(0.0).min(rect.w / 2.0).min(rect.h / 2.0);
    let n = |value: f64| format!("{value:.3}");
    let x1 = rect.x;
    let y1 = rect.y;
    let x2 = rect.x + rect.w;
    let y2 = rect.y + rect.h;
    let arc = format!("A{} {} 0 0 1", n(radius), n(radius));

    format!(
        "M{} {}H{}{arc} {} {}V{}{arc} {} {}H{}{arc} {} {}V{}{arc} {} {}Z",
        n(x1 + radius),
        n(y1),
        n(x2 - radius),
        n(x2),
        n(y1 + radius),
        n(y2 - radius),
        n(x2 - radius),
        n(y2),
        n(x1 + radius),
        n(x1),
        n(y2 - radius),
        n(y1 + radius),
        n(x1 + radius),
        n(y1),
    )
}

/// Punch the real trigger out of the goo layer.
pub fn trigger_cutout(geo: PopoverGeo) -> String {
    let layer = Rect {
        x: 0.0,
        y: 0.0,
        w: geo.layer_w,
        h: geo.layer_h,
        r: 0.0,
    };
    format!(
        "path(evenodd, \"{} {}\")",
        rounded_rect_path(layer),
        rounded_rect_path(geo.trigger)
    )
}

/// The measured trigger and panel boxes, kept in a signal.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PopoverLayout {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    content_width: f64,
    content_height: f64,
    measured: bool,
}

fn parse_layout(message: &str) -> Option<PopoverLayout> {
    let mut parts = message.split('|');
    let mut next = || parts.next()?.parse::<f64>().ok();
    Some(PopoverLayout {
        left: next()?,
        top: next()?,
        width: next()?,
        height: next()?,
        content_width: next()?,
        content_height: next()?,
        measured: true,
    })
}

fn goo_defs(strength: f64, uid: u32) -> String {
    format!(
        "<defs><filter id=\"beui-goo-{uid}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">\
         <feGaussianBlur in=\"SourceGraphic\" stdDeviation=\"{strength}\" result=\"blur\"/>\
         <feColorMatrix in=\"blur\" mode=\"matrix\" values=\"1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 22 -10\" result=\"goo\"/>\
         <feComposite in=\"SourceGraphic\" in2=\"goo\" operator=\"atop\"/></filter></defs>"
    )
}

#[derive(Clone, Copy)]
struct PopoverContext {
    open: Signal<bool>,
    set_open: EventHandler<bool>,
    open_hover: EventHandler<()>,
    schedule_close: EventHandler<()>,
    trigger_mode: PopoverTriggerMode,
    side: PopoverSide,
    align: PopoverAlign,
    gap: f64,
    panel_radius: f64,
    goo_strength: f64,
    reduce: bool,
    uid: u32,
    layout: Signal<PopoverLayout>,
}

fn use_popover() -> PopoverContext {
    use_context::<PopoverContext>()
}

/// `Popover`.
#[component]
pub fn Popover(
    #[props(default = None)] open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default)] trigger: PopoverTriggerMode,
    #[props(default)] side: PopoverSide,
    #[props(default)] align: PopoverAlign,
    /// Gap between trigger and panel, in px — the length of the gooey neck.
    #[props(default = 14.0)]
    side_offset: f64,
    /// Corner radius of the open panel, in px.
    #[props(default = 16.0)]
    panel_radius: f64,
    /// Blur radius feeding the goo filter — higher melts more.
    #[props(default = 8.0)]
    goo_strength: f64,
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
    let trigger_selector = format!("[data-beui-popover-trigger=\"{uid}\"]");
    let content_selector = format!("[data-beui-popover-content=\"{uid}\"]");

    // Hover timing: open at once, close 120 ms later unless something cancels.
    let close_token = use_signal(|| 0u32);
    let open_hover = EventHandler::new(move |_| {
        let mut token = close_token;
        token += 1;
        set_open.call(true);
    });
    let schedule_close = EventHandler::new(move |_| {
        let mut token = close_token;
        token += 1;
        let current = *token.peek();
        spawn(async move {
            if Time::delay(Duration::from_millis(120)).await.is_ok() && *token.peek() == current {
                set_open.call(false);
            }
        });
    });

    let close_content = content_selector.clone();
    let close_trigger = trigger_selector.clone();
    let close = EventHandler::new(move |_| {
        set_open.call(false);
        // Escape hands focus back to the trigger if it was inside the panel.
        let content = platform::js_string(&close_content);
        let trigger = platform::js_string(&close_trigger);
        platform::fire(&format!(
            "(() => {{ const c = document.querySelector({content}); const a = document.activeElement; \
             if (c && a && c.contains(a)) {{ const t = document.querySelector({trigger}); t && t.focus(); }} }})()"
        ));
    });
    let scope = use_dismiss(
        open_signal,
        close,
        DismissOptions::default().ignore(content_selector.clone()),
    );

    let layout = use_signal(PopoverLayout::default);
    use_context_provider(|| PopoverContext {
        open: open_signal,
        set_open,
        open_hover,
        schedule_close,
        trigger_mode: trigger,
        side,
        align,
        gap: side_offset,
        panel_radius,
        goo_strength,
        reduce,
        uid,
        layout,
    });

    let hover_enter = move |_| {
        if trigger == PopoverTriggerMode::Hover {
            open_hover.call(());
        }
    };
    let hover_leave = move |_| {
        if trigger == PopoverTriggerMode::Hover {
            schedule_close.call(());
        }
    };

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            class: cn!("relative inline-flex isolate", class.as_deref().unwrap_or("")),
            onpointerenter: hover_enter,
            onpointerleave: hover_leave,
            {children}
        }
    }
}

/// `PopoverTrigger` — renders the focusable control the panel oozes out of.
#[component]
pub fn PopoverTrigger(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_popover();
    let open = (context.open)();

    rsx! {
        button {
            r#type: "button",
            "data-beui-popover-trigger": "{context.uid}",
            "aria-haspopup": "dialog",
            "aria-expanded": open,
            "aria-controls": if open { Some(format!("beui-popover-{}-content", context.uid)) } else { None },
            "data-state": if open { "open" } else { "closed" },
            class: cn!("relative z-0", class.as_deref().unwrap_or("")),
            onclick: move |_| {
                let next = !(context.open)();
                context.set_open.call(next);
            },
            onpointerenter: move |_| {
                if context.trigger_mode == PopoverTriggerMode::Hover {
                    context.open_hover.call(());
                }
            },
            onpointerleave: move |_| {
                if context.trigger_mode == PopoverTriggerMode::Hover {
                    context.schedule_close.call(());
                }
            },
            {children}
        }
    }
}

/// `PopoverContent` — the goo layer and the clipped panel.
#[component]
pub fn PopoverContent(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_popover();
    let uid = context.uid;
    let trigger_selector = format!("[data-beui-popover-trigger=\"{uid}\"]");
    let content_selector = format!("[data-beui-popover-content=\"{uid}\"]");

    // Follow the trigger and the panel's own size.
    let layout_signal = context.layout;
    let follow_trigger = trigger_selector.clone();
    let follow_content = content_selector.clone();
    use_effect(move || {
        // The module and the start call share one eval: `dioxus.send` inside the
        // module resolves to *this* eval's channel, which is the one the loop
        // below reads. Defining the module in its own eval would send into a
        // channel that is already gone.
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

    // The progress motion lives here, not in the root: `dioxus-motion`'s frame
    // updates mark the scope that created the handle, so a different component
    // reading it would never see a new value.
    let progress_motion = use_motion(0.0f32).expect("0.0 is a finite animation value");
    let progress_signal = progress_motion;
    use_effect(move || {
        let is_open = (context.open)();
        let mut handle = progress_signal;
        let config = if context.reduce {
            tween_ease_out(1)
        } else if is_open {
            AnimationConfig::spring(spring_from_visual_duration(0.3, 0.15))
        } else {
            AnimationConfig::spring(spring_from_visual_duration(0.21, 0.15))
        };
        let _ = handle.animate_to(if is_open { 1.0 } else { 0.0 }, config);
    });

    let layout = (context.layout)();
    let open = (context.open)();
    let geo = build_geo(
        layout.width,
        layout.height,
        layout.content_width,
        layout.content_height,
        context.side,
        context.align,
        context.gap,
        context.panel_radius,
    );
    let progress = f64::from(progress_motion.get_value());
    let clip = clip_for_progress(geo, progress);
    let cutout = trigger_cutout(geo);
    let goo = if context.reduce {
        String::new()
    } else {
        format!("filter:url(#beui-goo-{uid});")
    };
    let align_origin = match context.align {
        PopoverAlign::Start => "left",
        PopoverAlign::Center => "center",
        PopoverAlign::End => "right",
    };
    let vertical_origin = match context.side {
        PopoverSide::Bottom => "top",
        PopoverSide::Top => "bottom",
    };
    let content_style = format!(
        "position:absolute;left:{}px;top:{}px;transform-origin:{align_origin} {vertical_origin}",
        geo.panel.x, geo.panel.y,
    );

    rsx! {
        PortalHost {
            div {
                "data-popover-portal": "",
                class: "pointer-events-none fixed left-0 top-0 z-[9999] isolate size-0",
                style: format!(
                    "visibility:{};transform:translate3d({}px, {}px, 0)",
                    if layout.measured { "visible" } else { "hidden" },
                    layout.left,
                    layout.top,
                ),
                svg {
                    "aria-hidden": "true",
                    width: "0",
                    height: "0",
                    class: "absolute",
                    dangerous_inner_html: goo_defs(context.goo_strength, uid),
                }
                div {
                    "aria-hidden": "true",
                    class: "pointer-events-none absolute z-[-1]",
                    style: format!(
                        "left:{}px;top:{}px;width:{}px;height:{}px;{goo}clip-path:{cutout}",
                        geo.left, geo.top, geo.layer_w, geo.layer_h,
                    ),
                    div {
                        class: "absolute bg-popover",
                        style: format!(
                            "left:{}px;top:{}px;width:{}px;height:{}px;border-radius:{}px",
                            geo.trigger.x, geo.trigger.y, geo.trigger.w, geo.trigger.h, geo.trigger.r,
                        ),
                    }
                    div { class: "absolute inset-0 bg-popover", style: format!("clip-path:{clip}") }
                }
                div {
                    class: "pointer-events-none absolute z-10",
                    style: format!(
                        "left:{}px;top:{}px;width:{}px;height:{}px",
                        geo.left, geo.top, geo.layer_w, geo.layer_h,
                    ),
                    div {
                        inert: if open { None } else { Some("true") },
                        class: "absolute inset-0",
                        style: format!(
                            "clip-path:{clip};pointer-events:{}",
                            if open { "auto" } else { "none" },
                        ),
                        div {
                            "data-beui-popover-content": "{uid}",
                            id: "beui-popover-{uid}-content",
                            role: "dialog",
                            style: content_style,
                            class: cn!(
                                "w-max max-w-[min(92vw,20rem)] p-4 text-popover-foreground outline-none",
                                class.as_deref().unwrap_or(""),
                            ),
                            onpointerenter: move |_| {
                                if context.trigger_mode == PopoverTriggerMode::Hover {
                                    context.open_hover.call(());
                                }
                            },
                            onpointerleave: move |_| {
                                if context.trigger_mode == PopoverTriggerMode::Hover {
                                    context.schedule_close.call(());
                                }
                            },
                            {children}
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_center_geometry() {
        // 100x40 trigger, 200x80 panel, 14 gap, centered below.
        let geo = build_geo(
            100.0,
            40.0,
            200.0,
            80.0,
            PopoverSide::Bottom,
            PopoverAlign::Center,
            14.0,
            16.0,
        );
        // panel x = (100 - 200)/2 = -50 → layer starts 50px left of the trigger.
        assert_eq!(geo.left, -50.0);
        assert_eq!(geo.top, 0.0);
        assert_eq!(geo.layer_w, 200.0);
        assert_eq!(geo.layer_h, 134.0);
        assert_eq!(geo.trigger.x, 50.0);
        assert_eq!(geo.panel.x, 0.0);
        assert_eq!(geo.panel.y, 54.0);
        // trigger radius = min(40/2, 16) = 16.
        assert_eq!(geo.trigger.r, 16.0);
    }

    #[test]
    fn top_align_end_geometry() {
        let geo = build_geo(
            100.0,
            40.0,
            80.0,
            60.0,
            PopoverSide::Top,
            PopoverAlign::End,
            10.0,
            12.0,
        );
        // px = 100 - 80 = 20; py = -(10 + 60) = -70.
        assert_eq!(geo.left, 0.0);
        assert_eq!(geo.top, -70.0);
        assert_eq!(geo.layer_w, 100.0);
        assert_eq!(geo.layer_h, 110.0);
        assert_eq!(geo.panel.x, 20.0);
        assert_eq!(geo.panel.y, 0.0);
    }

    #[test]
    fn clip_is_the_trigger_at_zero() {
        let geo = build_geo(
            100.0,
            40.0,
            200.0,
            80.0,
            PopoverSide::Bottom,
            PopoverAlign::Center,
            14.0,
            16.0,
        );
        let clip = clip_for_progress(geo, 0.0);
        // Trigger rect is x=50 y=0 w=100 h=40 in a 250x134 layer.
        assert_eq!(clip, "inset(0px 50px 94px 50px round 16px)");
    }
}

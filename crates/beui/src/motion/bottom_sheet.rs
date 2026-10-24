//! `components/motion/bottom-sheet.tsx` — a draggable sheet with snap points.
//!
//! A Vaul-style glide: the 0.5 s fully-damped `EASE_DRAWER` tween, not a spring,
//! and the same curve drives the backdrop so the surface and the scrim move as
//! one. Only the drag pill takes the gesture (`dragListener={false}` plus
//! `dragControls`), which is the port's `data-beui-drag` handle.
//!
//! Deviations from the source:
//!
//! - the sheet slides on the CSS `transform` property with a `vh` value
//!   (`translateY(50vh)` ⇄ `translateY(0vh)`), because `MotionStyle`'s `x`/`y`
//!   fields are pixels and the source's `"100%"` is the sheet's own height;
//!   that is 100% for the fraction snap points and an approximation for `"auto"`;
//! - draggable-without-a-handle, the `layout` between snap points, and the
//!   source's `onAnimationComplete` height cache are not ported — the height is
//!   a vh value the browser resolves.

use dioxus::prelude::*;

use beui_core::drag::{use_drag, DragAxis, DragEnd, DragOptions, DragRange};
use beui_core::ease::tween_ease_drawer;
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};
use beui_core::touch::TOUCH_GESTURE_CONTENT_CLASS;
use beui_core::{cn, use_reduced_motion};

use crate::motion::PortalHost;

/// Vaul-style glide: `{ duration: 0.5, ease: EASE_DRAWER }`.
const DRAWER_MS: u64 = 500;

/// A snap point: a fraction of the viewport, or the content's own height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SnapPoint {
    Fraction(f64),
    Auto,
}

impl SnapPoint {
    fn height_style(self) -> String {
        match self {
            Self::Auto => "max-height:92vh".to_string(),
            Self::Fraction(fraction) => format!("height:{}vh", fraction * 100.0),
        }
    }

    fn offscreen_vh(self) -> f32 {
        match self {
            Self::Auto => 92.0,
            Self::Fraction(fraction) => (fraction * 100.0) as f32,
        }
    }
}

fn default_snap_points() -> Vec<SnapPoint> {
    vec![SnapPoint::Fraction(0.5), SnapPoint::Fraction(0.92)]
}

fn backdrop_config() -> PresenceConfig {
    let drawer = tween_ease_drawer(DRAWER_MS);
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        drawer.clone(),
        drawer,
    )
}

/// The sheet's enter/exit. The glide is a transform whose offscreen distance is
/// the sheet's own height — written as a `vh` value so the browser resolves it.
fn sheet_config(offscreen_vh: f32, reduce: bool) -> PresenceConfig {
    if reduce {
        let drawer = tween_ease_drawer(180);
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            drawer.clone(),
            drawer,
        );
    }
    let offscreen = format!("translateY({offscreen_vh}vh)");
    let mut from = motion_style! {};
    from.add_css_property("transform", offscreen.as_str());
    let mut to = motion_style! {};
    to.add_css_property("transform", "translateY(0vh)");
    let drawer = tween_ease_drawer(DRAWER_MS);
    PresenceConfig::with_transitions(from.clone(), to, from, drawer.clone(), drawer)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Backdrop,
    Sheet,
}

impl Layer {
    const fn key(self) -> &'static str {
        match self {
            Self::Backdrop => "backdrop",
            Self::Sheet => "sheet",
        }
    }
}

/// `BottomSheet`.
#[component]
pub fn BottomSheet(
    open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    /// Heights (0-1 = fraction of viewport, or `Auto`). First entry is default.
    #[props(default = None)]
    snap_points: Option<Vec<SnapPoint>>,
    #[props(default = 0)] default_snap: usize,
    #[props(default = None)] title: Option<String>,
    #[props(default = None)] description: Option<String>,
    #[props(default = None)] class: Option<String>,
    /// Min drag distance (px) past the current snap to dismiss.
    #[props(default = 120.0)]
    dismiss_threshold: f64,
    children: Element,
) -> Element {
    let points = snap_points.unwrap_or_else(default_snap_points);
    let mut snap = use_signal(|| default_snap);
    let reduce = use_reduced_motion()();

    // `use_dismiss` reads a signal in its own effect, so the prop becomes one.
    let mut open_signal = use_signal(|| open);
    if *open_signal.peek() != open {
        open_signal.set(open);
    }
    let dismiss = EventHandler::new(move |_| {
        if let Some(callback) = on_open_change {
            callback.call(false);
        }
    });
    let scope = use_dismiss(open_signal, dismiss, DismissOptions::default());

    // Reopening resets to the default snap, as the source's effect does.
    use_effect(move || {
        if open_signal() {
            snap.set(default_snap);
        }
    });

    // The iOS-safe lock: `overflow:hidden` alone lets the page scroll under the
    // sheet; `position:fixed` holds, and the scroll position is restored.
    use_effect(move || {
        if open_signal() {
            platform::fire(
                "const b = document.body; \
                 window.__beuiSheet = { position: b.style.position, top: b.style.top, left: b.style.left, right: b.style.right, overflow: b.style.overflow, y: window.scrollY }; \
                 b.style.position = 'fixed'; b.style.top = '-' + window.__beuiSheet.y + 'px'; \
                 b.style.left = '0'; b.style.right = '0'; b.style.overflow = 'hidden';",
            );
        } else {
            platform::fire(
                "if (window.__beuiSheet) { const s = window.__beuiSheet; const b = document.body; \
                 b.style.position = s.position; b.style.top = s.top; b.style.left = s.left; b.style.right = s.right; b.style.overflow = s.overflow; \
                 window.scrollTo(0, s.y); delete window.__beuiSheet; }",
            );
        }
    });

    let points_for_drag = points.clone();
    let on_drag_end = EventHandler::new(move |end: DragEnd| {
        let velocity = end.velocity.y;
        let offset = end.offset.y;
        let current = *snap.peek();
        let last = points_for_drag.len().saturating_sub(1);

        let dismiss_sheet = EventHandler::new(move |_| {
            if let Some(callback) = on_open_change {
                callback.call(false);
            }
        });

        // Strong downward fling or large drag.
        if velocity > 600.0 || offset > dismiss_threshold {
            let smaller: Vec<usize> = (0..current).filter(|index| *index < current).collect();
            if !smaller.is_empty() && velocity < 800.0 && offset < dismiss_threshold * 1.6 {
                snap.set(smaller[smaller.len() - 1]);
            } else {
                dismiss_sheet.call(());
            }
            return;
        }

        // Strong upward fling.
        if velocity < -500.0 {
            snap.set((current + 1).min(last));
            return;
        }

        // Otherwise nearest by offset.
        if offset > 80.0 && current > 0 {
            snap.set(current - 1);
        } else if offset < -80.0 && current < last {
            snap.set(current + 1);
        }
    });

    let selected = snap().min(points.len().saturating_sub(1));
    let point = points
        .get(selected)
        .copied()
        .unwrap_or(SnapPoint::Fraction(0.5));
    let layers: Vec<Layer> = if open {
        vec![Layer::Backdrop, Layer::Sheet]
    } else {
        Vec::new()
    };
    let layer_children = children;
    let scope_attr = scope.attr();

    rsx! {
        PortalHost {
            AnimatePresence {
                for layer in layers {
                    SheetLayer {
                        key: "{layer.key()}",
                        layer,
                        offscreen_vh: point.offscreen_vh(),
                        reduce,
                        scope_attr: scope_attr.clone(),
                        height_style: point.height_style(),
                        title: title.clone(),
                        description: description.clone(),
                        class: class.clone(),
                        on_close: dismiss,
                        on_drag_end,
                        children: layer_children.clone(),
                    }
                }
            }
        }
    }
}

#[component]
fn SheetLayer(
    layer: Layer,
    offscreen_vh: f32,
    reduce: bool,
    scope_attr: String,
    height_style: String,
    title: Option<String>,
    description: Option<String>,
    class: Option<String>,
    on_close: EventHandler<()>,
    on_drag_end: EventHandler<DragEnd>,
    children: Element,
) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(match layer {
        Layer::Backdrop => backdrop_config(),
        Layer::Sheet => sheet_config(offscreen_vh, reduce),
    })
    .expect("static presence config");
    let css = presence_css(&handle);

    match layer {
        Layer::Backdrop => rsx! {
            button {
                r#type: "button",
                "aria-label": "Close bottom sheet",
                inert: gate.inert_attr(),
                style: gate.style_with(&css),
                class: "pointer-events-auto fixed inset-0 z-50 bg-background/40 backdrop-blur-sm",
                onclick: move |_| on_close.call(()),
            }
        },
        Layer::Sheet => rsx! {
            SheetSurface {
                scope_attr,
                css,
                height_style,
                title,
                description,
                class,
                on_drag_end,
                gate,
                children,
            }
        },
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn SheetSurface(
    scope_attr: String,
    css: String,
    height_style: String,
    title: Option<String>,
    description: Option<String>,
    class: Option<String>,
    on_drag_end: EventHandler<DragEnd>,
    gate: beui_core::presence::Gate,
    children: Element,
) -> Element {
    let drag = use_drag(
        DragOptions::default()
            .axis(DragAxis::Y)
            .range(DragRange::FREE, DragRange::pinned(0.0))
            .elastic_y(0.02, 0.4)
            .without_momentum(),
        Some(on_drag_end),
    );

    // While the gesture runs (and while it settles) the drag transform has to
    // win over the presence transform; when idle it is exactly zero and the
    // presence value applies untouched.
    //
    // Reading `velocity` is what subscribes this render to the drag: `offset()`
    // tracks the settle motion (which only moves after release), while the base
    // offset written during the gesture is read untracked. `velocity` changes on
    // every move, so the render — and therefore `offset()` — is up to date.
    let _ = drag.velocity();
    let offset_y = drag.offset().y;
    let dragging = drag.is_dragging() || drag.is_settling();
    let mut style = format!("{height_style};{}", gate.style_with(&css));
    if dragging {
        style.push_str(&format!(";transform:translateY({offset_y}px)"));
    }

    let title_id = use_hook(|| format!("beui-sheet-title-{}", platform::next_scope_id()));
    let description_id = use_hook(|| format!("beui-sheet-desc-{}", platform::next_scope_id()));

    rsx! {
        div {
            "data-beui-scope": scope_attr,
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": title.as_ref().map(|_| title_id.clone()),
            "aria-describedby": description.as_ref().map(|_| description_id.clone()),
            "aria-label": if title.is_none() { Some("Bottom sheet") } else { None },
            inert: gate.inert_attr(),
            style,
            class: cn!(
                "pointer-events-auto fixed bottom-0 left-0 right-0 z-50 mx-auto flex max-w-2xl flex-col overflow-hidden rounded-t-3xl will-change-transform",
                "border border-border bg-background shadow-xl",
                class.as_deref().unwrap_or(""),
            ),
            div { class: "flex flex-col items-center px-4 pb-2 pt-3",
                div {
                    "data-sheet-handle": "",
                    "data-beui-drag": drag.scope(),
                    class: cn!(
                        "flex cursor-grab touch-none items-center justify-center py-1 active:cursor-grabbing",
                        TOUCH_GESTURE_CONTENT_CLASS,
                    ),
                    onpointerdown: move |event| drag.on_pointer_down(&event),
                    onpointermove: move |event| drag.on_pointer_move(&event),
                    onpointerup: move |event| drag.on_pointer_up(&event),
                    onpointercancel: move |event| drag.on_pointer_cancel(&event),
                    onlostpointercapture: move |event| drag.on_pointer_up(&event),
                    div { class: "h-1.5 w-10 rounded-full bg-muted-foreground/40" }
                }
                if title.is_some() || description.is_some() {
                    div { class: "mt-2 w-full",
                        if let Some(title) = &title {
                            h2 { id: "{title_id}", class: "text-base font-semibold text-foreground", "{title}" }
                        }
                        if let Some(description) = &description {
                            p { id: "{description_id}", class: "mt-0.5 text-sm text-muted-foreground", "{description}" }
                        }
                    }
                }
            }
            div { class: "flex-1 overflow-y-auto overscroll-contain px-4 pb-6", {children} }
        }
    }
}

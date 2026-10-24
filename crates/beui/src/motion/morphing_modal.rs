//! `components/motion/morphing-modal.tsx` — one panel, several views.
//!
//! The chrome is two fixed siblings inside one presence boundary: a backdrop
//! that spans the viewport and a layer that positions the panel. Switching
//! `view_id` crossfades the panel's body through a nested `popLayout` presence,
//! so the old view leaves while the new one enters.
//!
//! Deviations from the source:
//!
//! - the panel's `layout` size morph is not ported: the enter transform and the
//!   flip transform both write `transform` on the same element, so a size change
//!   between views snaps rather than springs.

use dioxus::prelude::*;
use dioxus_motion::prelude::AnimationConfig;

use crate::motion::PortalHost;
use beui_core::ease::{tween_ease_out, SPRING_PANEL};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
    PresenceMode,
};
use beui_core::{cn, use_reduced_motion};

/// Where the panel sits in the viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MorphingModalPlacement {
    #[default]
    Bottom,
    Center,
}

fn backdrop_config() -> PresenceConfig {
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        tween_ease_out(200),
        tween_ease_out(200),
    )
}

fn panel_config(reduce: bool, placement: MorphingModalPlacement) -> PresenceConfig {
    let enter_y = if reduce {
        0.0
    } else if placement == MorphingModalPlacement::Bottom {
        40.0
    } else {
        20.0
    };
    let enter_scale = if reduce { 1.0 } else { 0.97 };
    let exit_scale = if reduce { 1.0 } else { 0.98 };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0, y: enter_y, scale: enter_scale },
        motion_style! { opacity: 1.0, y: 0.0, scale: 1.0 },
        motion_style! { opacity: 0.0, y: enter_y, scale: exit_scale },
        AnimationConfig::spring(SPRING_PANEL),
        tween_ease_out(180),
    )
}

fn content_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(180),
            tween_ease_out(140),
        );
    }
    let mut enter = motion_style! { opacity: 0.0, y: 8.0 };
    enter.add_css_property("filter", "blur(4px)");
    let mut center = motion_style! { opacity: 1.0, y: 0.0 };
    center.add_css_property("filter", "blur(0px)");
    let mut exit = motion_style! { opacity: 0.0, y: -8.0 };
    exit.add_css_property("filter", "blur(4px)");
    PresenceConfig::with_transitions(
        enter,
        center,
        exit,
        tween_ease_out(240),
        tween_ease_out(160),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Backdrop,
    PanelLayer,
}

impl Layer {
    const fn key(self) -> &'static str {
        match self {
            Self::Backdrop => "backdrop",
            Self::PanelLayer => "panel-layer",
        }
    }
}

/// `MorphingModal`.
#[component]
pub fn MorphingModal(
    /// Which view is shown; `None` closes the modal.
    #[props(default = None)]
    view_id: Option<String>,
    #[props(default = None)] on_close: Option<EventHandler<()>>,
    #[props(default)] placement: MorphingModalPlacement,
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let reduce = use_reduced_motion()();
    let open = view_id.is_some();
    let mut open_signal = use_signal(|| open);
    if *open_signal.peek() != open {
        open_signal.set(open);
    }

    // Body scroll lock while open.
    use_effect(move || {
        if open_signal() {
            platform::fire(
                "window.__beuiMorphModalPrev = document.body.style.overflow; document.body.style.overflow = 'hidden';",
            );
        } else {
            platform::fire(
                "document.body.style.overflow = window.__beuiMorphModalPrev ?? ''; delete window.__beuiMorphModalPrev;",
            );
        }
    });

    let layers: Vec<Layer> = if open {
        vec![Layer::Backdrop, Layer::PanelLayer]
    } else {
        Vec::new()
    };
    let content = children;

    rsx! {
        PortalHost {
            AnimatePresence {
                initial: false,
                for layer in layers {
                    MorphingModalLayer {
                        key: "{layer.key()}",
                        layer,
                        placement,
                        reduce,
                        view_id: view_id.clone(),
                        class: class.clone(),
                        on_close,
                        children: content.clone(),
                    }
                }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn MorphingModalLayer(
    layer: Layer,
    placement: MorphingModalPlacement,
    reduce: bool,
    view_id: Option<String>,
    class: Option<String>,
    on_close: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(match layer {
        Layer::Backdrop => backdrop_config(),
        Layer::PanelLayer => panel_config(reduce, placement),
    })
    .expect("static presence config");
    let css = presence_css(&handle);

    match layer {
        Layer::Backdrop => rsx! {
            button {
                r#type: "button",
                "aria-label": "Close modal",
                inert: gate.inert_attr(),
                style: gate.style_with(&css),
                class: "pointer-events-auto fixed inset-0 z-[80] bg-background/5 [backdrop-filter:blur(14px)_saturate(140%)] [-webkit-backdrop-filter:blur(14px)_saturate(140%)]",
                onclick: move |_| {
                    if let Some(callback) = on_close {
                        callback.call(());
                    }
                },
            }
        },
        Layer::PanelLayer => rsx! {
            div {
                inert: gate.inert_attr(),
                class: cn!(
                    "pointer-events-none fixed inset-4 z-[80] flex justify-center",
                    if placement == MorphingModalPlacement::Bottom { "items-end pb-4" } else { "items-center" },
                ),
                div {
                    inert: gate.inert_attr(),
                    style: gate.style_with(&format!("{css};will-change:transform")),
                    class: cn!(
                        "pointer-events-auto relative w-full max-w-sm overflow-hidden rounded-3xl border border-border bg-background shadow-2xl",
                        class.as_deref().unwrap_or(""),
                    ),
                    div { class: "p-5",
                        AnimatePresence {
                            initial: false,
                            mode: PresenceMode::PopLayout,
                            for key in view_id.iter().cloned().collect::<Vec<_>>() {
                                MorphingModalView { key: "{key}", reduce, children: children.clone() }
                            }
                        }
                    }
                }
            }
        },
    }
}

#[component]
fn MorphingModalView(reduce: bool, children: Element) -> Element {
    let handle = use_presence_style(content_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);
    rsx! {
        div { style: css, {children} }
    }
}

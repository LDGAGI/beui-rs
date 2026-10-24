//! `components/motion/drawer.tsx` — a side panel, backdrop and surface as two
//! fixed siblings.
//!
//! Both layers are presence children of one `AnimatePresence`, each reading the
//! gate, so interaction releases in the commit that starts the exit. There is no
//! wrapper and no portal: the source renders two `position: fixed` siblings
//! inline, and this port keeps the same DOM.
//!
//! Deviations from the source:
//!
//! - the panel springs `x` from ±320 px rather than `"100%"` (dioxus-motion's
//!   `MotionStyle` has no percentage transform), which matches `w-80` on a
//!   normal viewport and over-travels only when `max-w-[85vw]` clamps it;
//! - `useDismiss` stands in for the source's window `keydown` listener; a click
//!   anywhere outside the panel is a click on the fixed backdrop, so the two
//!   are equivalent while `dismissable` decides whether the backdrop is ignored.

use dioxus::prelude::*;
use dioxus_motion::prelude::AnimationConfig;

use beui_core::ease::{tween_ease_out, SPRING_PANEL};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};
use beui_core::{cn, use_reduced_motion};

/// Which edge the panel is anchored to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrawerSide {
    Left,
    #[default]
    Right,
}

impl DrawerSide {
    const fn offscreen(self) -> f32 {
        match self {
            Self::Right => 320.0,
            Self::Left => -320.0,
        }
    }
}

fn backdrop_config() -> PresenceConfig {
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        tween_ease_out(250),
        tween_ease_out(250),
    )
}

fn panel_config(side: DrawerSide, reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(200),
            tween_ease_out(200),
        );
    }
    let offscreen = side.offscreen();
    PresenceConfig::with_transitions(
        motion_style! { x: offscreen },
        motion_style! { x: 0.0 },
        motion_style! { x: offscreen },
        AnimationConfig::spring(SPRING_PANEL),
        AnimationConfig::spring(SPRING_PANEL),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Backdrop,
    Panel,
}

impl Layer {
    const fn key(self) -> &'static str {
        match self {
            Self::Backdrop => "backdrop",
            Self::Panel => "panel",
        }
    }
}

/// `Drawer`.
#[component]
pub fn Drawer(
    open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    #[props(default)] side: DrawerSide,
    /// Class for the panel surface.
    #[props(default = None)]
    class: Option<String>,
    /// Class for the backdrop.
    #[props(default = None)]
    backdrop_class: Option<String>,
    #[props(default = None)] aria_label: Option<String>,
    /// Close when the backdrop is clicked. Default true.
    #[props(default = true)]
    dismissable: bool,
    children: Element,
) -> Element {
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
    let mut options = DismissOptions::default();
    if !dismissable {
        options = options.ignore("[data-beui-drawer-backdrop]");
    }
    let scope = use_dismiss(open_signal, dismiss, options);

    // Body scroll lock, the source's effect.
    use_effect(move || {
        if open_signal() {
            platform::fire(
                "if (document.body.dataset.beuiDrawerSaved === undefined) { \
                 document.body.dataset.beuiDrawerSaved = document.body.style.overflow; } \
                 document.body.style.overflow = 'hidden';",
            );
        } else {
            platform::fire(
                "document.body.style.overflow = document.body.dataset.beuiDrawerSaved ?? ''; \
                 delete document.body.dataset.beuiDrawerSaved;",
            );
        }
    });

    let layers: Vec<Layer> = if open {
        vec![Layer::Backdrop, Layer::Panel]
    } else {
        Vec::new()
    };
    let layer_children = children;
    let scope_attr = scope.attr();

    rsx! {
        AnimatePresence {
            for layer in layers {
                DrawerLayer {
                    key: "{layer.key()}",
                    layer,
                    side,
                    reduce,
                    dismissable,
                    scope_attr: scope_attr.clone(),
                    class: class.clone(),
                    backdrop_class: backdrop_class.clone(),
                    aria_label: aria_label.clone(),
                    on_close: dismiss,
                    children: layer_children.clone(),
                }
            }
        }
    }
}

#[component]
fn DrawerLayer(
    layer: Layer,
    side: DrawerSide,
    reduce: bool,
    dismissable: bool,
    scope_attr: String,
    class: Option<String>,
    backdrop_class: Option<String>,
    aria_label: Option<String>,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    let gate = use_gate();
    let handle = use_presence_style(match layer {
        Layer::Backdrop => backdrop_config(),
        Layer::Panel => panel_config(side, reduce),
    })
    .expect("static presence config");
    let css = presence_css(&handle);

    match layer {
        Layer::Backdrop => rsx! {
            button {
                r#type: "button",
                "aria-label": "Close",
                "tabindex": if dismissable { 0 } else { -1 },
                "data-beui-drawer-backdrop": "",
                inert: gate.inert_attr(),
                style: gate.style_with(&css),
                class: cn!(
                    "fixed inset-0 z-50 h-full w-full cursor-default bg-black/40 backdrop-blur-sm",
                    backdrop_class.as_deref().unwrap_or(""),
                ),
                onclick: move |_| {
                    if dismissable {
                        on_close.call(());
                    }
                },
            }
        },
        Layer::Panel => rsx! {
            aside {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": aria_label.as_deref().unwrap_or("Drawer"),
                "data-beui-scope": scope_attr,
                inert: gate.inert_attr(),
                style: gate.style_with(&css),
                class: cn!(
                    "fixed inset-y-0 z-50 flex w-80 max-w-[85vw] flex-col bg-background shadow-2xl",
                    match side {
                        DrawerSide::Right => "right-0 border-l border-border",
                        DrawerSide::Left => "left-0 border-r border-border",
                    },
                    class.as_deref().unwrap_or(""),
                ),
                {children}
            }
        },
    }
}

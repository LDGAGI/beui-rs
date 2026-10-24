//! `components/motion/center-morph-modal.tsx` — a panel that unfolds from its
//! own centre.
//!
//! The panel is laid out at full size and clipped to a 48% inset; the clip
//! tweens to zero over 0.43 s with `cubic-bezier(0.2, 0, 0.2, 1)`, and the
//! radius stays constant so the whole duration reads as surface unfolding. A
//! `drop-shadow` on the layer follows the clipped silhouette.
//!
//! Deviations from the source:
//!
//! - `CenterMorphModalTrigger`/`Close` render their own `<button>` (no
//!   `cloneElement`);
//! - the focus trap, initial focus, Escape and focus restoration live in
//!   `center_morph.js` (one listener with a `dioxus.send` bridge), not in Rust.

use dioxus::prelude::*;
use dioxus_motion::prelude::{AnimationConfig, AnimationMode, Duration, Tween};

use crate::icons::XIcon;
use crate::motion::PortalHost;
use beui_core::ease::{cubic_bezier_value, tween_ease_out, EASE_OUT_CSS};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_gate, use_presence_style, AnimatePresence, PresenceConfig,
};
use beui_core::{cn, use_reduced_motion};

const CENTER_FOLDED_CLIP: &str = "inset(48% 48% 48% 48% round 30px)";
const CENTER_OPEN_CLIP: &str = "inset(0% 0% 0% 0% round 30px)";

/// `CENTER_UNFOLD_EASE = [0.2, 0, 0.2, 1]` in the `easer` shape.
fn unfold_easer(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if d == 0.0 {
        return b + c;
    }
    b + c * cubic_bezier_value(0.2, 0.0, 0.2, 1.0, (t / d) as f64) as f32
}

fn unfold_config() -> AnimationConfig {
    AnimationConfig::new(AnimationMode::Tween(
        Tween::new(Duration::from_millis(430)).with_easing(unfold_easer),
    ))
}

fn backdrop_config(reduce: bool) -> PresenceConfig {
    let duration = if reduce { 100 } else { 280 };
    PresenceConfig::with_transitions(
        motion_style! { opacity: 0.0 },
        motion_style! { opacity: 1.0 },
        motion_style! { opacity: 0.0 },
        tween_ease_out(duration),
        tween_ease_out(duration),
    )
}

fn panel_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(140),
            tween_ease_out(140),
        );
    }
    let mut folded = motion_style! { opacity: 1.0 };
    folded.add_css_property("clip-path", CENTER_FOLDED_CLIP);
    let mut open = motion_style! { opacity: 1.0 };
    open.add_css_property("clip-path", CENTER_OPEN_CLIP);
    PresenceConfig::with_transitions(
        folded.clone(),
        open,
        folded,
        unfold_config(),
        unfold_config(),
    )
}

#[derive(Clone, Copy)]
struct CenterMorphContext {
    open: Signal<bool>,
    set_open: EventHandler<bool>,
    uid: u32,
}

fn use_center_morph() -> CenterMorphContext {
    use_context::<CenterMorphContext>()
}

/// `CenterMorphModal`.
#[component]
pub fn CenterMorphModal(
    #[props(default = None)] open: Option<bool>,
    #[props(default = false)] default_open: bool,
    #[props(default = None)] on_open_change: Option<EventHandler<bool>>,
    children: Element,
) -> Element {
    let mut internal_open = use_signal(|| default_open);
    let controlled = open.is_some();
    let fallback = internal_open();
    let current_open = open.unwrap_or(fallback);
    let open_signal = use_signal(|| current_open);
    let mut open_sync = open_signal;
    if *open_sync.peek() != current_open {
        open_sync.set(current_open);
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
    use_context_provider(|| CenterMorphContext {
        open: open_signal,
        set_open,
        uid,
    });

    rsx! {
        {children}
    }
}

/// `CenterMorphModalTrigger`.
#[component]
pub fn CenterMorphModalTrigger(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_center_morph();
    let open = (context.open)();

    rsx! {
        button {
            r#type: "button",
            id: "beui-center-morph-{context.uid}-trigger",
            "aria-haspopup": "dialog",
            "aria-expanded": open,
            "aria-controls": if open { Some(format!("beui-center-morph-{}-content", context.uid)) } else { None },
            class: cn!("relative inline-flex", class.as_deref().unwrap_or("")),
            onclick: move |_| context.set_open.call(!(context.open)()),
            {children}
        }
    }
}

/// `CenterMorphModalClose`.
#[component]
pub fn CenterMorphModalClose(
    #[props(default = None)] class: Option<String>,
    children: Element,
) -> Element {
    let context = use_center_morph();
    rsx! {
        button {
            r#type: "button",
            class: cn!("relative inline-flex", class.as_deref().unwrap_or("")),
            onclick: move |_| context.set_open.call(false),
            {children}
        }
    }
}

/// `CenterMorphModalContent`.
#[component]
pub fn CenterMorphModalContent(
    aria_label: &'static str,
    #[props(default = None)] aria_described_by: Option<String>,
    /// Close on Escape or backdrop press.
    #[props(default = true)]
    dismissible: bool,
    /// Render the close control inside the panel's top-right corner.
    #[props(default = true)]
    show_close_button: bool,
    #[props(default = "Close modal")] close_button_label: &'static str,
    #[props(default = None)] class: Option<String>,
    #[props(default = None)] backdrop_class: Option<String>,
    children: Element,
) -> Element {
    let context = use_center_morph();
    let reduce = use_reduced_motion()();
    let open = (context.open)();
    let uid = context.uid;
    let content_id = format!("beui-center-morph-{uid}-content");
    let trigger_id = format!("beui-center-morph-{uid}-trigger");

    // Focus, Escape and the Tab trap; body scroll lock.
    let effect_content = content_id.clone();
    let effect_trigger = trigger_id.clone();
    let set_open = context.set_open;
    let open_signal = context.open;
    use_effect(move || {
        if open_signal() {
            platform::fire("document.body.style.overflow='hidden';");
            let script = format!(
                "{}\nwindow.__beuiCenterMorphStart({uid}, {}, {}, {});",
                include_str!("../js/center_morph.js"),
                platform::js_string(&effect_content),
                platform::js_string(&effect_trigger),
                dismissible,
            );
            let Some(mut eval) = platform::eval(&script) else {
                return;
            };
            spawn(async move {
                while let Ok(message) = eval.recv::<String>().await {
                    if message == "escape" {
                        set_open.call(false);
                    }
                }
            });
        } else {
            platform::fire(&format!(
                "window.__beuiCenterMorphStop && window.__beuiCenterMorphStop({uid}); document.body.style.overflow='';"
            ));
        }
    });

    let layers: Vec<()> = if open { vec![()] } else { Vec::new() };
    let panel_children = children;

    rsx! {
        PortalHost {
            AnimatePresence {
                initial: false,
                for _ in layers {
                    CenterMorphLayer {
                        key: "center-morph-{uid}",
                        reduce,
                        dismissible,
                        show_close_button,
                        close_button_label,
                        aria_label,
                        aria_described_by: aria_described_by.clone(),
                        content_id: content_id.clone(),
                        class: class.clone(),
                        backdrop_class: backdrop_class.clone(),
                        on_close: context.set_open,
                        children: panel_children.clone(),
                    }
                }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn CenterMorphLayer(
    reduce: bool,
    dismissible: bool,
    show_close_button: bool,
    close_button_label: &'static str,
    aria_label: &'static str,
    aria_described_by: Option<String>,
    content_id: String,
    class: Option<String>,
    backdrop_class: Option<String>,
    on_close: EventHandler<bool>,
    children: Element,
) -> Element {
    let gate = use_gate();
    let backdrop_handle =
        use_presence_style(backdrop_config(reduce)).expect("static presence config");
    let backdrop_css = presence_css(&backdrop_handle);
    let handle = use_presence_style(panel_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);
    let is_present = gate.is_present();

    // The close button fades in 0.16 s after the unfold and leaves fast.
    let close_style = if is_present {
        format!(
            "opacity:1;transform:scale(1);transition:opacity 200ms {EASE_OUT_CSS} 160ms,transform 200ms {EASE_OUT_CSS} 160ms"
        )
    } else {
        format!(
            "opacity:0;transform:scale(0.88);transition:opacity 100ms {EASE_OUT_CSS},transform 100ms {EASE_OUT_CSS}"
        )
    };

    rsx! {
        button {
            r#type: "button",
            "aria-label": "Dismiss modal",
            tabindex: -1,
            disabled: !dismissible,
            inert: gate.inert_attr(),
            style: gate.style_with(&backdrop_css),
            class: cn!(
                "pointer-events-auto fixed inset-0 z-[100] h-full w-full cursor-default bg-background/10 backdrop-blur-sm",
                backdrop_class.as_deref().unwrap_or(""),
            ),
            onclick: move |_| on_close.call(false),
        }
        div {
            inert: gate.inert_attr(),
            class: "pointer-events-none fixed inset-4 z-[100] flex items-center justify-center overflow-y-auto drop-shadow-2xl",
            div { class: "flex w-full flex-col items-center py-8",
                div {
                    id: content_id,
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-label": aria_label,
                    "aria-describedby": aria_described_by,
                    tabindex: -1,
                    inert: gate.inert_attr(),
                    style: gate.style_with(&format!("{css};will-change:clip-path")),
                    class: cn!(
                        "pointer-events-auto relative w-full max-w-[26rem] origin-center overflow-hidden rounded-[30px] border border-border bg-background",
                        class.as_deref().unwrap_or(""),
                    ),
                    {children}
                    if show_close_button {
                        button {
                            r#type: "button",
                            "aria-label": close_button_label,
                            style: close_style,
                            class: "absolute right-4 top-4 inline-flex h-8 w-8 items-center justify-center rounded-full bg-foreground/[0.05] text-muted-foreground transition-colors hover:bg-foreground/[0.08] hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                            onclick: move |_| on_close.call(false),
                            XIcon { class: "h-4 w-4" }
                        }
                    }
                }
            }
        }
    }
}

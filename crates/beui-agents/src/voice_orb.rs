//! `agents/voice-orb/` — the voice-activity sphere.
//!
//! The source renders a WebGL/GLSL liquid sphere from `renderer.ts` (296 lines
//! of GLSL and WebGL setup) and springs a translate/scale from the activity
//! value. Shaders are the one dependency that has to go through JS interop, and
//! this port does not carry the renderer yet, so this is a
//! **stand-in**: the same DOM (`data-slot="voice-orb"`, `role="img"`, the
//! `data-render-state` contract, the canvas) with a CSS radial-gradient sphere
//! standing in for the material and a CSS transition standing in for the
//! `SPRING_GLIDE` transform. Replace the fallback with the interop renderer.

use beui::prelude::*;

/// The default pigment: base, highlight and shadow.
pub const DEFAULT_VOICE_ORB_COLORS: [&str; 3] = ["#e8754d", "#ffe0ac", "#346b52"];

/// `VoiceOrb`.
#[component]
pub fn VoiceOrb(
    /// Normalized activity, 0–1. A negative value means "no signal" and is
    /// clamped to 0.
    #[props(default = 0.0)]
    activity: f64,
    #[props(default = ["#e8754d".to_string(), "#ffe0ac".to_string(), "#346b52".to_string()])]
    colors: [String; 3],
    #[props(default = true)] active: bool,
    #[props(default = 1.0)] speed: f64,
    #[props(default)] label: Option<String>,
    #[props(default)] class: String,
    #[props(default)] style: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let reduce = use_reduced_motion()();
    let level = if active && !reduce {
        activity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let transform = if reduce {
        "none".to_string()
    } else {
        format!(
            "translate3d(0, {}px, 0) scale({}, {})",
            -level * 2.0,
            1.0 + level * 0.04,
            1.0 - level * 0.018,
        )
    };
    let gradient = format!(
        "radial-gradient(circle at 34% 28%, {highlight} 0%, {base} 46%, {shadow} 100%)",
        highlight = colors[1],
        base = colors[0],
        shadow = colors[2],
    );
    let _ = speed;
    let has_label = label.is_some();

    rsx! {
        div {
            role: "img",
            "aria-label": label,
            "aria-hidden": if has_label { None } else { Some("true") },
            "data-slot": "voice-orb",
            "data-render-state": "ready",
            class: cn!("relative aspect-square w-64 shrink-0", class),
            style: "{style}",
            ..attributes,
            div {
                class: "size-full",
                style: "transform:{transform};transition:transform 320ms cubic-bezier(0.32,0.72,0,1);",
                div {
                    class: "block size-full rounded-full",
                    style: "background:{gradient};filter:blur(0.5px);",
                }
            }
        }
    }
}

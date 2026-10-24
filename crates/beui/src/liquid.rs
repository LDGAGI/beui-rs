//! `motion/liquid.tsx` — the goo primitive other components build on: a
//! container with the SVG filter that fuses its children's silhouettes, plus
//! [`LiquidItem`], a box that morphs between positions inside it.
//!
//! The filter chain is the source's, attribute for attribute: `feGaussianBlur`
//! at `stdDeviation = blur`, the alpha `feColorMatrix`
//! (`… contrast intercept`, with `intercept = (0.5 - contrast * 5/12)` rounded
//! to two decimals), `feComposite operator="atop"` against the source graphic,
//! and the optional edge chain (`feMorphology erode radius = edgeWidth`,
//! `out`-composited into an `feFlood` of `edgeColor`/`edgeOpacity`, merged back
//! under the shape). The region is `size + padding * 2` with
//! `padding = ceil(blur * 3 + filterPadding)`.
//!
//! The source creates each item's `<rect>` imperatively and appends it to a
//! portal `<g>`, then drives both the wrapper and the rect from a `rAF` loop. A
//! Dioxus context cannot hand a component a DOM node to render into, so the
//! items register their animated box in a signal here and the container renders
//! the rects — which also removes the source's "keep the two on the same frame"
//! hazard by construction: both read the same box.
//!
//! `LiquidItem` interpolates the box itself with the source's cubic-bezier
//! (`cubic_bezier_value`), so a custom `ease` behaves exactly as it does there.

use crate::prelude::*;
use beui_core::ease::cubic_bezier_value;
use beui_core::platform::{eval, js_string, next_scope_id};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationManager, Duration, MotionHandle,
};
use std::collections::HashMap;

/// A box in the liquid layer's coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}

impl LiquidBox {
    fn mix(from: Self, to: Self, progress: f64) -> Self {
        let mix = |a: f64, b: f64| a + (b - a) * progress;
        Self {
            x: mix(from.x, to.x),
            y: mix(from.y, to.y),
            width: mix(from.width, to.width),
            height: mix(from.height, to.height),
            radius: mix(from.radius, to.radius),
        }
    }
}

/// What the items read from the container.
#[derive(Clone, Copy, PartialEq)]
struct LiquidContext {
    /// The id each item claims on mount.
    register: Signal<HashMap<u32, LiquidBox>>,
}

/// Props for [`Liquid`].
#[derive(Props, Clone, PartialEq)]
pub struct LiquidProps {
    /// Blur radius of the fusing pass. Defaults to `6`.
    #[props(default = 6.0)]
    pub blur: f64,
    /// Alpha contrast of the goo pass. Defaults to `18`.
    #[props(default = 18.0)]
    pub contrast: f64,
    /// The fill of the fusing layer. Defaults to `var(--background)`.
    #[props(default = "var(--background)".to_string())]
    pub fill: String,
    /// The outline color. Defaults to `var(--foreground)`.
    #[props(default = "var(--foreground)".to_string())]
    pub edge_color: String,
    #[props(default = 0.08)]
    pub edge_opacity: f64,
    /// Outline width in px. `0` disables the edge pass. Defaults to `1`.
    #[props(default = 1.0)]
    pub edge_width: f64,
    /// Extra filter-region padding. Defaults to `24`.
    #[props(default = 24.0)]
    pub filter_padding: f64,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    pub children: Element,
}

/// The goo container.
#[component]
pub fn Liquid(props: LiquidProps) -> Element {
    let LiquidProps {
        blur,
        contrast,
        fill,
        edge_color,
        edge_opacity,
        edge_width,
        filter_padding,
        class,
        style,
        children,
    } = props;

    let id = use_hook(next_scope_id);
    let filter_id = format!("liquid-{id}");
    let boxes = use_signal(HashMap::<u32, LiquidBox>::new);
    use_context_provider(|| LiquidContext { register: boxes });

    let mut size = use_signal(|| (0.0f64, 0.0f64));
    let selector = format!("[data-beui-liquid=\"{id}\"]");
    let script = format!(
        "const el = document.querySelector({selector});\n\
         if (!el) return;\n\
         const send = () => dioxus.send([el.offsetWidth, el.offsetHeight]);\n\
         send();\n\
         const observer = new ResizeObserver(send);\n\
         observer.observe(el);\n\
         for (;;) {{ if ((await dioxus.recv()) === \"close\") break; }}\n\
         observer.disconnect();",
        selector = js_string(&selector),
    );
    use_effect(move || {
        let Some(mut handle) = eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(next) = handle.recv::<Vec<f64>>().await {
                if next.len() >= 2 {
                    size.set((next[0], next[1]));
                }
            }
        });
    });

    let intercept = ((0.5 - contrast * (5.0 / 12.0)) * 100.0).round() / 100.0;
    let padding = (blur * 3.0 + filter_padding).ceil();
    let (width, height) = size();
    let class = cn!("relative isolate", class);
    let registered: Vec<LiquidBox> = boxes().into_values().collect();

    rsx! {
        div { "data-beui-liquid": "{id}", class, style,
            svg {
                "aria-hidden": "true",
                "focusable": "false",
                class: "pointer-events-none absolute inset-0 z-0 size-full overflow-visible",
                defs {
                    filter {
                        id: "{filter_id}",
                        x: "{-padding}",
                        y: "{-padding}",
                        width: "{width + padding * 2.0}",
                        height: "{height + padding * 2.0}",
                        "filterUnits": "userSpaceOnUse",
                        color_interpolation_filters: "sRGB",
                        feGaussianBlur { "in": "SourceGraphic", std_deviation: "{blur}", result: "blur" }
                        feColorMatrix {
                            "in": "blur",
                            r#type: "matrix",
                            values: "1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 {contrast} {intercept}",
                            result: "goo",
                        }
                        feComposite { "in": "SourceGraphic", "in2": "goo", operator: "atop", result: "shape" }
                        if edge_width > 0.0 {
                            feColorMatrix {
                                "in": "shape",
                                r#type: "matrix",
                                values: "1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 60 -29.5",
                                result: "solid-shape",
                            }
                            feMorphology {
                                "in": "solid-shape",
                                operator: "erode",
                                radius: "{edge_width}",
                                result: "inset-shape",
                            }
                            feComposite { "in": "solid-shape", "in2": "inset-shape", operator: "out", result: "edge-mask" }
                            feFlood { flood_color: "{edge_color}", flood_opacity: "{edge_opacity}", result: "edge-color" }
                            feComposite { "in": "edge-color", "in2": "edge-mask", operator: "in", result: "edge" }
                            feMerge {
                                feMergeNode { "in": "shape" }
                                feMergeNode { "in": "edge" }
                            }
                        }
                    }
                }
                g { fill: "{fill}", filter: "url(#{filter_id})",
                    for (index, box_) in registered.iter().enumerate() {
                        rect {
                            key: "{index}",
                            x: "{box_.x}",
                            y: "{box_.y}",
                            width: "{box_.width}",
                            height: "{box_.height}",
                            rx: "{box_.radius}",
                        }
                    }
                }
            }
            {children}
        }
    }
}

/// Props for [`LiquidItem`].
#[derive(Props, Clone, PartialEq)]
pub struct LiquidItemProps {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Corner radius. Defaults to `min(width, height) / 2`.
    #[props(default)]
    pub radius: Option<f64>,
    /// Morph duration in ms. Defaults to `280` (`0` under reduced motion).
    #[props(default)]
    pub duration: Option<f64>,
    /// `[x1, y1, x2, y2]`. Defaults to `EASE_OUT`.
    #[props(default)]
    pub ease: Option<[f64; 4]>,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    pub children: Element,
}

/// A box that morphs between positions inside [`Liquid`].
#[component]
pub fn LiquidItem(props: LiquidItemProps) -> Element {
    let LiquidItemProps {
        x,
        y,
        width,
        height,
        radius,
        duration,
        ease,
        class,
        style,
        children,
    } = props;

    let context = use_context::<LiquidContext>();
    let reduce = use_reduced_motion();
    let id = use_hook(next_scope_id);
    let target = LiquidBox {
        x,
        y,
        width,
        height,
        radius: radius.unwrap_or_else(|| width.min(height) / 2.0),
    };
    let duration_ms = if reduce() {
        0.0
    } else {
        duration.unwrap_or(280.0)
    };
    let ease = ease.unwrap_or(EASE_OUT);

    let mut progress = use_motion(1.0f32).expect("1 is finite");
    let mut from = use_signal(|| target);
    let mut displayed = use_signal(|| target);

    // A target change restarts the morph from wherever the box is right now.
    // `peek` keeps this effect off the box it writes.
    use_effect(use_reactive((&target,), move |(target,)| {
        if duration_ms <= 0.0 {
            from.set(target);
            displayed.set(target);
            return;
        }
        let origin = displayed.peek().to_owned();
        if origin == target {
            return;
        }
        from.set(origin);
        progress.reset();
        let _ = progress.animate_to(
            1.0,
            AnimationConfig::tween(Duration::from_millis(duration_ms as u64)),
        );
    }));

    let eased = cubic_bezier_value(
        ease[0],
        ease[1],
        ease[2],
        ease[3],
        f64::from(current_progress(progress)),
    );
    let box_now = LiquidBox::mix(from(), target, eased);

    // Publish the animated box so the container draws the matching blob in the
    // same frame the wrapper moves.
    use_effect(use_reactive((&box_now,), move |(box_now,)| {
        displayed.set(box_now);
        let mut map = context.register;
        map.write().insert(id, box_now);
    }));

    let class = cn!("absolute left-0 top-0 z-10", class);
    let item_style = format!(
        "{};transform:translate({}px, {}px);width:{}px;height:{}px;will-change:transform,width,height",
        style.clone().unwrap_or_default(),
        box_now.x,
        box_now.y,
        box_now.width,
        box_now.height,
    );

    rsx! {
        div { class, style: item_style, {children} }
    }
}

/// The morph's progress, read without subscribing the whole item again.
fn current_progress(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

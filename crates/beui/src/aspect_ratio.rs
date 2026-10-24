//! `motion/aspect-ratio.tsx` — a frame with a fixed ratio and an image that
//! covers it.
//!
//! The source's `layout` projection on the frame and the image is not ported
//! (motion's nested projection is a known gap in `beui-core::flip`); the
//! frame's `aspect-ratio` and the image's cover-crop choice are.

use crate::prelude::*;

/// The ratio and animation flag the image reads from its frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectRatioContext {
    pub ratio: f64,
    pub animated: bool,
}

/// Props for [`AspectRatio`].
#[derive(Props, Clone, PartialEq)]
pub struct AspectRatioProps {
    /// Width divided by height, e.g. `16.0 / 9.0`. Must be finite and positive.
    #[props(default = 1.0)]
    pub ratio: f64,
    /// Morph layout changes (layout projection is not ported; kept for API shape).
    #[props(default = true)]
    pub animated: bool,
    #[props(default)]
    pub class: Option<String>,
    /// Inline style merged with the ratio declaration.
    #[props(default)]
    pub style: Option<String>,
    /// Classes for the content layer.
    #[props(default)]
    pub content_class: Option<String>,
    #[props(default)]
    pub children: Option<Element>,
}

/// A frame with a fixed aspect ratio.
#[component]
pub fn AspectRatio(props: AspectRatioProps) -> Element {
    let AspectRatioProps {
        ratio,
        animated,
        class,
        style,
        content_class,
        children,
    } = props;
    if !ratio.is_finite() || ratio <= 0.0 {
        return VNode::empty();
    }
    let reduce = use_reduced_motion();
    let enabled = animated && !reduce();
    use_context_provider(|| AspectRatioContext {
        ratio,
        animated: enabled,
    });
    let class = cn!("relative w-full min-w-0 overflow-hidden", class);
    let base = style.unwrap_or_default();
    let base = base.trim().trim_end_matches(';');
    let style = if base.is_empty() {
        format!("aspect-ratio:{ratio}")
    } else {
        format!("{base};aspect-ratio:{ratio}")
    };
    rsx! {
        div { class, style,
            div { class: cn!("absolute inset-0", content_class), {children} }
        }
    }
}

/// Props for [`AspectRatioImage`].
#[derive(Props, Clone, PartialEq)]
pub struct AspectRatioImageProps {
    pub src: String,
    pub alt: String,
    /// Intrinsic width in pixels.
    pub width: f64,
    /// Intrinsic height in pixels.
    pub height: f64,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub draggable: bool,
}

/// An image that covers its [`AspectRatio`] frame.
#[component]
pub fn AspectRatioImage(props: AspectRatioImageProps) -> Element {
    let AspectRatioImageProps {
        src,
        alt,
        width,
        height,
        class,
        draggable,
    } = props;
    let context = use_context::<AspectRatioContext>();
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return VNode::empty();
    }
    let fill_width = context.ratio >= width / height;
    let style = if fill_width {
        "width:100%;height:auto"
    } else {
        "width:auto;height:100%"
    };
    let class = cn!("col-start-1 row-start-1 block max-w-none", class);
    rsx! {
        div { class: "absolute inset-0 grid grid-cols-1 grid-rows-1 place-items-center",
            img {
                src,
                alt,
                width: "{width}",
                height: "{height}",
                draggable: if draggable { "true" } else { "false" },
                class,
                style,
            }
        }
    }
}

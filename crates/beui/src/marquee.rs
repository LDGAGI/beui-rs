//! `motion/marquee.tsx` — two duplicated tracks drifting under a CSS keyframe.
//!
//! Pure CSS. The wrapper carries `gap` as well as each track, so the seam
//! between the two tracks matches the spacing between items and the loop stays
//! even. The second track is `aria-hidden` and `inert`, so a screen reader and
//! the tab order see one copy of the content.

use crate::prelude::*;

/// Which way the track drifts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarqueeDirection {
    #[default]
    Left,
    Right,
    Up,
    Down,
}

impl MarqueeDirection {
    fn vertical(self) -> bool {
        matches!(self, MarqueeDirection::Up | MarqueeDirection::Down)
    }

    fn reverse(self) -> bool {
        matches!(self, MarqueeDirection::Right | MarqueeDirection::Down)
    }
}

/// Props for [`Marquee`].
#[derive(Props, Clone, PartialEq)]
pub struct MarqueeProps {
    pub children: Element,
    #[props(default)]
    pub direction: MarqueeDirection,
    /// Seconds for one full loop. Defaults to `30`.
    #[props(default = 30.0)]
    pub speed: f64,
    /// Pause both tracks while the pointer is over the marquee. Defaults to `true`.
    #[props(default = true)]
    pub pause_on_hover: bool,
    /// Gap between items (and across the seam). Defaults to `1rem`.
    #[props(default = "1rem".to_string())]
    pub gap: String,
    #[props(default)]
    pub class: Option<String>,
    /// Fade the leading and trailing edges. Defaults to `true`.
    #[props(default = true)]
    pub fade: bool,
}

/// A seamless CSS marquee.
#[component]
pub fn Marquee(props: MarqueeProps) -> Element {
    let MarqueeProps {
        children,
        direction,
        speed,
        pause_on_hover,
        gap,
        class,
        fade,
    } = props;

    let vertical = direction.vertical();
    let reverse = direction.reverse();

    let wrapper_class = cn!(
        "group relative flex overflow-hidden",
        if vertical { "flex-col" } else { "flex-row" },
        if fade && !vertical {
            "[mask-image:linear-gradient(to_right,transparent,black_12%,black_88%,transparent)]"
        } else {
            ""
        },
        if fade && vertical {
            "[mask-image:linear-gradient(to_bottom,transparent,black_12%,black_88%,transparent)]"
        } else {
            ""
        },
        class,
    );

    let wrapper_style = format!("--gap:{gap};gap:{gap}");

    let track_class = cn!(
        "flex shrink-0 items-center",
        if vertical {
            "flex-col animate-marquee-vertical"
        } else {
            "flex-row animate-marquee"
        },
        if pause_on_hover {
            "group-hover:[animation-play-state:paused]"
        } else {
            ""
        },
    );

    let track_style = format!(
        "animation-duration:{speed}s;animation-direction:{};gap:{gap}",
        if reverse { "reverse" } else { "normal" }
    );

    rsx! {
        div { class: wrapper_class, style: wrapper_style,
            for dup in 0..2usize {
                div {
                    key: "{dup}",
                    aria_hidden: if dup == 1 { "true" } else { "false" },
                    // `inert` is a boolean attribute: `inert="false"` is still
                    // present, so the first track writes `None` to remove it.
                    inert: if dup == 1 { Some("true") } else { None },
                    class: track_class.clone(),
                    style: track_style.clone(),
                    {children.clone()}
                }
            }
        }
    }
}

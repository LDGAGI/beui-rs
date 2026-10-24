//! `motion/scroll-progress.tsx` — a bar or a ring tracking a scroller's
//! progress.
//!
//! `useSmoothScroll().progress` (the page) is the default source; a `progress`
//! signal overrides it for contained scroll areas. `spring` runs the value
//! through a soft follow (`PROGRESS_SPRING`), disabled under reduced motion.

use crate::prelude::*;
use crate::scroll::{use_scroll_y_progress, use_spring_value};
use dioxus_motion::prelude::Spring;

/// `PROGRESS_SPRING` — softer than the UI springs on purpose.
const PROGRESS_SPRING: Spring = Spring {
    stiffness: 120.0,
    damping: 30.0,
    mass: 0.6,
};

/// Bar or ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollProgressVariant {
    #[default]
    Bar,
    Circle,
}

/// Which edge a bar pins to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollProgressPosition {
    #[default]
    Top,
    Bottom,
}

/// Props for [`ScrollProgress`].
///
/// The variant is a field rather than a Rust enum-with-payload so the two
/// shapes share one props type; fields the selected variant does not use are
/// ignored, matching the source's discriminated union.
#[derive(Props, Clone, PartialEq)]
pub struct ScrollProgressProps {
    /// Override the scroll source. Defaults to the page.
    #[props(default)]
    pub progress: Option<Signal<f64>>,
    /// Spring-smooth the value. Defaults to `true`; disabled under reduced motion.
    #[props(default = true)]
    pub spring: bool,
    #[props(default)]
    pub variant: ScrollProgressVariant,
    #[props(default)]
    pub position: ScrollProgressPosition,
    /// Bar thickness in px. Defaults to `2`.
    #[props(default = 2.0)]
    pub height: f64,
    /// Position with `fixed` (page) or `absolute` (embedded). Defaults to `true`.
    #[props(default = true)]
    pub fixed: bool,
    /// Ring diameter in px. Defaults to `40`.
    #[props(default = 40.0)]
    pub size: f64,
    /// Ring stroke width in px. Defaults to `3`.
    #[props(default = 3.0)]
    pub thickness: f64,
    #[props(default)]
    pub class: Option<String>,
}

/// A scroll-progress indicator.
#[component]
pub fn ScrollProgress(props: ScrollProgressProps) -> Element {
    let ScrollProgressProps {
        progress,
        spring,
        variant,
        position,
        height,
        fixed,
        size,
        thickness,
        class,
    } = props;

    let reduce = use_reduced_motion();
    // Always read the page progress; a supplied source simply wins. Reading it
    // unconditionally keeps the hook order stable.
    let page = use_scroll_y_progress(None);
    let source = progress.unwrap_or(page);
    let value = use_spring_value(source, PROGRESS_SPRING, spring && !reduce());
    let value = f64::from(value.clamp(0.0, 1.0));

    match variant {
        ScrollProgressVariant::Bar => {
            let class = cn!(
                "left-0 right-0 z-50 origin-left bg-foreground",
                if fixed { "fixed" } else { "absolute" },
                if position == ScrollProgressPosition::Top {
                    "top-0"
                } else {
                    "bottom-0"
                },
                class,
            );
            let style = format!("height:{height}px;transform:scaleX({value})");
            rsx! {
                div { aria_hidden: "true", class, style }
            }
        }
        ScrollProgressVariant::Circle => {
            let radius = (size - thickness) / 2.0;
            let circumference = 2.0 * std::f64::consts::PI * radius;
            let offset = circumference * (1.0 - value);
            let center = size / 2.0;
            let class = cn!("text-foreground", class);
            rsx! {
                svg {
                    "aria-hidden": "true",
                    "focusable": "false",
                    role: "presentation",
                    width: "{size}",
                    height: "{size}",
                    view_box: "0 0 {size} {size}",
                    class,
                    circle {
                        cx: "{center}",
                        cy: "{center}",
                        r: "{radius}",
                        fill: "none",
                        stroke_width: "{thickness}",
                        class: "stroke-current opacity-15",
                    }
                    circle {
                        cx: "{center}",
                        cy: "{center}",
                        r: "{radius}",
                        fill: "none",
                        stroke_width: "{thickness}",
                        stroke_linecap: "round",
                        class: "stroke-current",
                        stroke_dasharray: "{circumference}",
                        transform: "rotate(-90 {center} {center})",
                        style: "stroke-dashoffset:{offset}",
                    }
                }
            }
        }
    }
}

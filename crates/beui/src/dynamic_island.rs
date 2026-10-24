//! `motion/dynamic-island.tsx` — a shell that springs its real width and height
//! to the active view's natural size.
//!
//! The shell animates `width`/`height` (not transforms) so slots are never
//! scale-distorted, and the radius stays a constant 32px — the browser clamps it
//! to half the shell height, so the pill-to-rect morph falls out of the resize.
//! `SHELL_SPRING` (`{duration: 0.8, bounce: 0.2}`) and `CONTENT_SPRING`
//! (`{0.8, 0.35}`) are time-defined springs, converted by [`crate::spring`].
//!
//! The source keeps an exiting view mounted with `AnimatePresence`; here a view
//! unmounts as soon as it is inactive, so only the enter half of the content
//! transition plays.

use crate::prelude::*;
use crate::rect::{use_element_rect, ElementRect};
use crate::spring::spring_from_duration;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{
    use_motion, AnimationConfig, AnimationManager, MotionHandle, Time, TimeProvider,
};

/// `RADIUS` — never animated.
const RADIUS: f64 = 32.0;
const PILL_WIDTH: f64 = 126.0;
const PILL_HEIGHT: f64 = 37.0;

fn shell_spring() -> dioxus_motion::prelude::Spring {
    spring_from_duration(0.8, 0.2)
}

fn content_spring() -> dioxus_motion::prelude::Spring {
    spring_from_duration(0.8, 0.35)
}

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The active view, read by [`DynamicIslandView`].
///
/// The view is a `Signal`, not a plain value: a context consumer only re-renders
/// when it reads something reactive, and `use_context` on its own subscribes to
/// nothing.
#[derive(Clone, Copy)]
struct IslandContext {
    view: Signal<Option<String>>,
}

/// Props for [`DynamicIsland`].
#[derive(Props, Clone, PartialEq)]
pub struct DynamicIslandProps {
    /// Active view id; `None` shows the compact pill.
    pub view: Option<String>,
    /// Compact pill content, shown when no view is active.
    #[props(default)]
    pub compact: Option<Element>,
    #[props(default)]
    pub children: Option<Element>,
    #[props(default)]
    pub class: Option<String>,
}

/// The island shell.
#[component]
pub fn DynamicIsland(props: DynamicIslandProps) -> Element {
    let DynamicIslandProps {
        view,
        compact,
        children,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let expanded = view.is_some();
    let rect: ElementRect = use_element_rect();
    let reduce_value = reduce();

    let mut view_signal = use_signal(|| view.clone());
    use_effect(use_reactive((&view,), move |(view,)| view_signal.set(view)));
    use_context_provider(|| IslandContext { view: view_signal });

    // Measure on mount (the shell must own explicit dimensions before the first
    // interaction) and again whenever the active view changes. The re-measure is
    // deferred a frame: the view is selected through a context signal, so the
    // active content mounts in a *following* commit and an immediate measure
    // would read an empty sizer.
    use_effect(move || rect.refresh());
    use_effect(use_reactive((&view,), move |_| {
        spawn(async move {
            let _ = Time::delay(dioxus_motion::Duration::from_millis(16)).await;
            rect.refresh();
        });
    }));

    let (target_width, target_height) = rect
        .get()
        .map(|rect| (rect.width, rect.height))
        .unwrap_or((PILL_WIDTH, PILL_HEIGHT));
    let mut width = use_motion(PILL_WIDTH as f32).expect("a finite width");
    let mut height = use_motion(PILL_HEIGHT as f32).expect("a finite height");
    use_effect(use_reactive(
        (&target_width, &target_height),
        move |(target_width, target_height)| {
            let config = if reduce_value {
                beui_core::ease::tween_ease_out(0)
            } else {
                AnimationConfig::spring(shell_spring())
            };
            let _ = width.animate_to(target_width as f32, config.clone());
            let _ = height.animate_to(target_height as f32, config);
        },
    ));

    let shell_style = format!(
        "width:{}px;height:{}px;border-radius:{RADIUS}px",
        f64::from(current(width)),
        f64::from(current(height))
    );
    let class = cn!(
        "relative inline-flex items-start justify-center overflow-hidden",
        "bg-foreground text-background shadow-2xl",
        class,
    );
    let reduce_now = reduce_value;

    rsx! {
        div {
            role: "status",
            aria_live: "polite",
            class,
            style: shell_style,
            div {
                class: "w-max shrink-0",
                "data-beui-rect": rect.attr(),
                onmounted: move |data| rect.measure(data.data()),
                if !expanded {
                    if let Some(compact) = compact {
                        Slot {
                            class: "min-h-[37px] min-w-[126px] gap-2 px-4 py-1.5 text-xs font-medium",
                            reduce: reduce_now,
                            {compact}
                        }
                    }
                }
                {children}
            }
        }
    }
}

/// Props for [`DynamicIslandView`].
#[derive(Props, Clone, PartialEq)]
pub struct DynamicIslandViewProps {
    /// Matches the parent `view` prop when active.
    pub id: String,
    pub children: Element,
    #[props(default)]
    pub class: Option<String>,
}

/// One view of a [`DynamicIsland`].
#[component]
pub fn DynamicIslandView(props: DynamicIslandViewProps) -> Element {
    let DynamicIslandViewProps {
        id,
        children,
        class,
    } = props;
    let context = use_context::<IslandContext>();
    let active = (context.view)().as_deref() == Some(id.as_str());
    let reduce = use_reduced_motion();

    rsx! {
        if active {
            Slot { class: cn!("px-6 py-4", class), reduce: reduce(), {children} }
        }
    }
}

/// One slot of content, unfurling out of the pill line.
#[component]
fn Slot(
    children: Element,
    #[props(default)] class: Option<String>,
    #[props(default)] reduce: bool,
) -> Element {
    let mut progress = use_motion(0.0f32).expect("0.0 is finite");
    use_effect(move || {
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(content_spring())
        };
        let _ = progress.animate_to(1.0, config);
    });
    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if reduce {
        format!("opacity:{p};transform:none;filter:none")
    } else {
        format!(
            "opacity:{p};transform-origin:top center;transform:translateY({}px) scale({});filter:blur({}px)",
            -8.0 * (1.0 - p),
            0.9 + 0.1 * p,
            5.0 * (1.0 - p)
        )
    };
    let class = cn!("flex items-center justify-center", class);

    rsx! {
        div { style, class, {children} }
    }
}

//! `motion/bloom-menu.tsx` — a trigger that blooms into a panel.
//!
//! The trigger and the panel share one `layoutId`, which in this port is
//! `beui_core::flip`'s registry: the panel mounts pinned to the trigger's last
//! box and springs to its own, so the morph grows from the middle outward. The
//! panel's content then plays its iris reveal and the items stagger radially by
//! their distance from the grid centre.
//!
//! Motion's nested `layout` projection (which counter-scales the content while
//! the box morphs) is a known gap, so the content can stretch briefly during the
//! morph.

use crate::icons::{Bell, FileText, FolderClosed, LayoutGrid, LinkIcon, Plus, Table, X};
use crate::prelude::*;
use beui_core::flip::{use_flip, FlipMode, FlipOptions};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform::next_scope_id;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

/// `SPRING_FOLDER`, verbatim.
const SPRING_FOLDER: Spring = Spring {
    stiffness: 300.0,
    damping: 32.0,
    mass: 0.9,
};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

fn folder_config(reduce: bool) -> AnimationConfig {
    if reduce {
        beui_core::ease::tween_ease_out(150)
    } else {
        AnimationConfig::spring(SPRING_FOLDER)
    }
}

/// One item in the bloom grid.
#[derive(Clone, PartialEq)]
pub struct BloomMenuItem {
    pub label: String,
    pub icon: Element,
}

impl BloomMenuItem {
    /// An item with a label and icon.
    pub fn new(label: impl Into<String>, icon: Element) -> Self {
        Self {
            label: label.into(),
            icon,
        }
    }
}

/// Props for [`BloomMenu`].
#[derive(Props, Clone, PartialEq)]
pub struct BloomMenuProps {
    #[props(default)]
    pub items: Option<Vec<BloomMenuItem>>,
    #[props(default)]
    pub on_select: Option<EventHandler<String>>,
    #[props(default)]
    pub class: Option<String>,
}

/// A trigger that blooms into a panel of create actions.
#[component]
pub fn BloomMenu(props: BloomMenuProps) -> Element {
    let BloomMenuProps {
        items,
        on_select,
        class,
    } = props;
    let items = items.unwrap_or_else(default_items);
    let mut open = use_signal(|| false);
    let layout_id = use_hook(|| format!("bloom-menu-{}", next_scope_id()));
    let scope = use_dismiss(
        open,
        EventHandler::new(move |_| open.set(false)),
        DismissOptions::default(),
    );
    let class = cn!("relative inline-flex", class);

    rsx! {
        div { class, "data-beui-scope": scope.attr(),
            div { class: "h-11 w-36", aria_hidden: "true" }
            div { class: "pointer-events-none absolute left-1/2 top-1/2 z-30 grid h-[300px] w-[min(86vw,420px)] -translate-x-1/2 -translate-y-1/2 place-items-center [&>*]:pointer-events-auto",
                if open() {
                    BloomPanel {
                        layout_id,
                        items,
                        on_close: EventHandler::new(move |_| open.set(false)),
                        on_select: EventHandler::new(move |label: String| {
                            if let Some(handler) = on_select {
                                handler.call(label);
                            }
                            open.set(false);
                        }),
                    }
                } else {
                    BloomTrigger {
                        layout_id,
                        on_open: EventHandler::new(move |_| open.set(true)),
                    }
                }
            }
        }
    }
}

fn default_items() -> Vec<BloomMenuItem> {
    vec![
        BloomMenuItem::new(
            "Doc",
            rsx! { FileText { class: Some("h-5 w-5".to_string()) } },
        ),
        BloomMenuItem::new(
            "Board",
            rsx! { LayoutGrid { class: Some("h-5 w-5".to_string()) } },
        ),
        BloomMenuItem::new(
            "Table",
            rsx! { Table { class: Some("h-5 w-5".to_string()) } },
        ),
        BloomMenuItem::new(
            "Folder",
            rsx! { FolderClosed { class: Some("h-5 w-5".to_string()) } },
        ),
        BloomMenuItem::new(
            "Reminder",
            rsx! { Bell { class: Some("h-5 w-5".to_string()) } },
        ),
        BloomMenuItem::new(
            "Link",
            rsx! { LinkIcon { class: Some("h-5 w-5".to_string()) } },
        ),
    ]
}

/// The collapsed trigger.
#[component]
fn BloomTrigger(layout_id: String, on_open: EventHandler<()>) -> Element {
    let reduce = use_reduced_motion();
    let flip = use_flip(
        true,
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(folder_config(reduce()))
            .mode(FlipMode::Both)
            .border_radius(16.0),
    );
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    let scale_value = f64::from(current(scale));
    let press = || AnimationConfig::spring(SPRING_PRESS);

    rsx! {
        button {
            r#type: "button",
            "data-beui-flip": flip.scope(),
            aria_haspopup: "menu",
            aria_expanded: "false",
            class: "inline-flex h-11 w-36 items-center justify-center border border-border bg-card text-sm font-medium text-foreground",
            style: flip.style_with("border-radius:16px"),
            onclick: move |_| on_open.call(()),
            onpointerdown: move |_| {
                if !reduce() {
                    let _ = scale.animate_to(0.97, press());
                }
            },
            onpointerup: move |_| {
                let _ = scale.animate_to(1.0, press());
            },
            onpointerleave: move |_| {
                let _ = scale.animate_to(1.0, press());
            },
            span {
                class: "inline-flex items-center gap-2 whitespace-nowrap",
                style: "transform:scale({scale_value})",
                "Create"
                Plus { class: Some("h-4 w-4".to_string()) }
            }
        }
    }
}

/// The expanded panel.
#[component]
fn BloomPanel(
    layout_id: String,
    items: Vec<BloomMenuItem>,
    on_close: EventHandler<()>,
    on_select: EventHandler<String>,
) -> Element {
    let reduce = use_reduced_motion();
    let flip = use_flip(
        true,
        FlipOptions::default()
            .layout_id(layout_id)
            .transition(folder_config(reduce()))
            .mode(FlipMode::Both)
            .border_radius(16.0),
    );
    let mut iris = use_motion(0.0f32).expect("0.0 is finite");
    let mut header = use_motion(0.0f32).expect("0.0 is finite");
    let reduce_value = reduce();
    use_effect(move || {
        let delay = if reduce_value { 0 } else { 80 };
        let _ = iris.animate_to(
            1.0,
            beui_core::ease::tween_ease_out(450)
                .with_delay(dioxus_motion::Duration::from_millis(delay)),
        );
        let _ = header.animate_to(
            1.0,
            beui_core::ease::tween_ease_out(200).with_delay(dioxus_motion::Duration::from_millis(
                if reduce_value { 0 } else { 120 },
            )),
        );
    });

    let p = f64::from(current(iris).clamp(0.0, 1.0));
    let inset_x = 34.0 * (1.0 - p);
    let inset_y = 45.0 * (1.0 - p);
    let iris_style = format!("clip-path:inset({inset_y}% {inset_x}% {inset_y}% {inset_x}%)");
    let header_value = f64::from(current(header).clamp(0.0, 1.0));
    let count = items.len();
    let rows = count.div_ceil(3);
    let reduce_now = reduce_value;

    rsx! {
        div {
            "data-beui-flip": flip.scope(),
            class: "w-[min(86vw,420px)] overflow-hidden border border-border bg-card",
            style: flip.style_with("border-radius:16px"),
            div {
                div { style: "opacity:{header_value}",
                    div { class: "flex items-center justify-between border-b border-border px-4 py-3",
                        span { class: "text-sm font-medium text-muted-foreground", "Create" }
                        button {
                            r#type: "button",
                            aria_label: "Close menu",
                            class: "text-muted-foreground transition-colors hover:text-foreground",
                            onclick: move |_| on_close.call(()),
                            X { class: Some("h-4 w-4".to_string()) }
                        }
                    }
                }
                div { class: "grid grid-cols-3", style: iris_style,
                    for (index, item) in items.into_iter().enumerate() {
                        {
                            let col = index % 3;
                            let row = index / 3;
                            let dist = ((col as f64 - 1.0).powi(2)
                                + (row as f64 - (rows as f64 - 1.0) / 2.0).powi(2))
                            .sqrt();
                            let cell_class = cn!(
                                "flex items-center justify-center px-3 py-6 text-muted-foreground transition-colors hover:text-foreground",
                                if index % 3 != 2 { "border-r border-border" } else { "" },
                                if index < 3 { "border-b border-border" } else { "" },
                            );
                            let label = item.label.clone();
                            let select_label = item.label.clone();
                            rsx! {
                                button {
                                    key: "{label}",
                                    r#type: "button",
                                    class: cell_class,
                                    onclick: move |_| on_select.call(select_label.clone()),
                                    BloomItem { label: item.label.clone(), icon: item.icon.clone(), dist, reduce: reduce_now }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One grid item, staggering in by its distance from the centre.
#[component]
fn BloomItem(label: String, icon: Element, dist: f64, reduce: bool) -> Element {
    let mut progress = use_motion(0.0f32).expect("0.0 is finite");
    use_effect(move || {
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(Spring {
                stiffness: 440.0,
                damping: 34.0,
                mass: 1.0,
            })
            .with_delay(dioxus_motion::Duration::from_millis(
                ((0.1 + dist * 0.07) * 1000.0).round() as u64,
            ))
        };
        let _ = progress.animate_to(1.0, config);
    });
    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = format!(
        "opacity:{p};transform:scale({});filter:blur({}px)",
        0.85 + 0.15 * p,
        6.0 * (1.0 - p)
    );

    rsx! {
        span {
            style,
            class: "flex flex-col items-center gap-2 will-change-[opacity,filter,transform]",
            {icon}
            span { class: "text-sm font-medium", "{label}" }
        }
    }
}

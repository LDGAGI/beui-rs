//! `motion/project-folder.tsx` — a folder that fans its previews out on hover
//! and opens them into a full-screen overlay.
//!
//! The fan is one progress: each card's rest placement comes from
//! `get_preview_transform` (offset from the centre, lift, rotate, scale,
//! opacity, z-index) and the opened placement multiplies it, as the source's
//! two `animate` states do. The overlay uses `use_modal_scope` for the inert
//! backdrop, scroll lock, focus trap, Escape and focus return.
//!
//! Two deviations: the overlay renders in place rather than through
//! `beui-core::portal` (Dioxus delegates bubbling clicks to the mount root, so a
//! portaled subtree becomes dead), and the overlay cards
//! enter with a scale/fade instead of the source's `layoutId` morph out of the
//! fan (the fan cards already own their transform, so a flip pin cannot share
//! it).

use crate::icons::X;
use crate::prelude::*;
use beui_core::hooks::{use_dismiss, use_modal_scope, DismissOptions};
use beui_core::platform::next_scope_id;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, Spring};

const MAX_PREVIEWS: usize = 5;

fn layout_spring() -> Spring {
    // `SPRING_LAYOUT` from `lib/ease.ts`.
    Spring {
        stiffness: 360.0,
        damping: 32.0,
        mass: 0.6,
    }
}

fn press_spring() -> Spring {
    Spring {
        stiffness: 500.0,
        damping: 30.0,
        mass: 0.6,
    }
}

fn current(handle: dioxus_motion::prelude::MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// One preview card.
#[derive(Clone, PartialEq)]
pub struct ProjectFolderPreview {
    pub id: String,
    pub content: Element,
}

impl ProjectFolderPreview {
    /// A preview with content.
    pub fn new(id: impl Into<String>, content: Element) -> Self {
        Self {
            id: id.into(),
            content,
        }
    }
}

/// A card's rest placement.
struct PreviewTransform {
    x: f64,
    y: f64,
    rotate: f64,
    scale: f64,
    opacity: f64,
    z_index: i32,
}

/// `getPreviewTransform`.
fn preview_transform(index: usize, count: usize) -> PreviewTransform {
    let offset = index as f64 - (count as f64 - 1.0) / 2.0;
    let distance = offset.abs();
    let center_lift = (2.0 - distance).max(0.0) * 8.0;
    PreviewTransform {
        x: offset * 44.0,
        y: 8.0 - center_lift,
        rotate: offset * 6.0,
        scale: if distance == 0.0 {
            1.04
        } else if distance == 1.0 {
            0.95
        } else {
            0.88
        },
        opacity: if distance == 0.0 {
            1.0
        } else if distance == 1.0 {
            0.78
        } else {
            0.58
        },
        z_index: 10 - distance as i32,
    }
}

/// Props for [`ProjectFolder`].
#[derive(Props, Clone, PartialEq)]
pub struct ProjectFolderProps {
    pub title: String,
    #[props(default = "Updated recently".to_string())]
    pub description: String,
    #[props(default)]
    pub previews: Vec<ProjectFolderPreview>,
    #[props(default)]
    pub count: Option<usize>,
    #[props(default = "file".to_string())]
    pub item_label: String,
    #[props(default)]
    pub open: Option<bool>,
    #[props(default)]
    pub default_open: bool,
    #[props(default)]
    pub on_open_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub expanded: Option<bool>,
    #[props(default)]
    pub default_expanded: bool,
    #[props(default)]
    pub on_expanded_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub on_click: Option<EventHandler<()>>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default)]
    pub class: Option<String>,
}

/// A folder of preview files.
#[component]
pub fn ProjectFolder(props: ProjectFolderProps) -> Element {
    let ProjectFolderProps {
        title,
        description,
        previews,
        count,
        item_label,
        open,
        default_open,
        on_open_change,
        expanded,
        default_expanded,
        on_expanded_change,
        on_click,
        disabled,
        aria_label,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let dialog_id = use_hook(next_scope_id);
    let mut internal_open = use_signal(|| default_open);
    let mut internal_expanded = use_signal(|| default_expanded);
    let open_controlled = open.is_some();
    let expanded_controlled = expanded.is_some();
    let preview_items: Vec<ProjectFolderPreview> =
        previews.into_iter().take(MAX_PREVIEWS).collect();
    let is_expanded = match expanded {
        Some(expanded) => expanded,
        None => internal_expanded(),
    };
    let is_open = match open {
        Some(open) => open,
        None => internal_open(),
    } || is_expanded;
    let reduce_value = reduce();

    let has_previews = !preview_items.is_empty();
    let set_expanded = EventHandler::new(move |next: bool| {
        if disabled || !has_previews {
            return;
        }
        if !expanded_controlled {
            internal_expanded.set(next);
        }
        if let Some(handler) = on_expanded_change {
            handler.call(next);
        }
    });
    let set_open = EventHandler::new(move |next: bool| {
        if disabled {
            return;
        }
        if !open_controlled {
            internal_open.set(next);
        }
        if let Some(handler) = on_open_change {
            handler.call(next);
        }
    });
    let close_overlay = EventHandler::new(move |_| {
        set_open.call(false);
        set_expanded.call(false);
    });

    let expanded_signal = use_signal(|| is_expanded);
    let mut expanded_flag = expanded_signal;
    use_effect(use_reactive((&is_expanded,), move |(is_expanded,)| {
        expanded_flag.set(is_expanded);
    }));
    let scope = use_dismiss(
        expanded_flag,
        close_overlay,
        DismissOptions::default().without_escape(),
    );
    use_modal_scope(expanded_flag, close_overlay, scope);

    let mut open_progress = use_motion(if is_open { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    use_effect(move || {
        let config = || {
            if reduce_value {
                beui_core::ease::tween_ease_out(0)
            } else {
                AnimationConfig::spring(layout_spring())
            }
        };
        let _ = open_progress.animate_to(if is_open { 1.0 } else { 0.0 }, config());
    });

    let open_p = f64::from(current(open_progress).clamp(0.0, 1.0));
    let scale_value = f64::from(current(scale));
    let count_value = count.unwrap_or(preview_items.len());
    let count_text = format!(
        "{count_value} {item_label}{}",
        if count_value == 1 { "" } else { "s" }
    );
    let title_for_label = title.clone();
    let default_aria = format!("Open {title}");

    let back_style = if reduce_value {
        "transform:none".to_string()
    } else {
        format!("transform:rotateX({}deg)", 15.0 * open_p)
    };
    let front_style = if reduce_value {
        "transform:none".to_string()
    } else {
        format!("transform:rotateX({}deg)", -25.0 * open_p)
    };
    let button_class = cn!(
        "relative block h-56 w-72 select-none rounded-2xl text-left outline-none [perspective:1200px] focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-4 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50",
        class,
    );

    rsx! {
        div { "data-beui-scope": scope.attr(), class: "inline-block",
            button {
                r#type: "button",
                disabled,
                aria_label: aria_label.clone().unwrap_or(default_aria),
                aria_haspopup: "dialog",
                aria_expanded: if is_expanded { "true" } else { "false" },
                "data-open": if is_open { "true" } else { "false" },
                "data-expanded": if is_expanded { "true" } else { "false" },
                tabindex: if is_expanded { Some("-1") } else { None },
                class: button_class,
                style: "transform:scale({scale_value})",
                onpointerdown: move |_| {
                    if !reduce_value && !disabled {
                        let _ = scale.animate_to(0.98, AnimationConfig::spring(press_spring()));
                    }
                },
                onpointerup: move |_| {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(press_spring()));
                },
                onpointercancel: move |_| {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(press_spring()));
                },
                onpointerenter: move |_| {
                    if can_hover() {
                        set_open.call(true);
                    }
                },
                onpointerleave: move |_| {
                    if can_hover() && !is_expanded {
                        set_open.call(false);
                    }
                },
                onfocus: move |_| set_open.call(true),
                onblur: move |_| {
                    if !is_expanded {
                        set_open.call(false);
                    }
                },
                onclick: move |_| {
                    set_expanded.call(true);
                    set_open.call(true);
                    if let Some(handler) = on_click {
                        handler.call(());
                    }
                },
                // Back plate, tilting back as the folder opens.
                span {
                    aria_hidden: "true",
                    class: "absolute inset-0 rounded-2xl border border-foreground/10 bg-background/25 backdrop-blur-xl [transform-origin:center_bottom]",
                    style: back_style,
                }
                // The fan.
                span { aria_hidden: "true", class: "pointer-events-none absolute inset-0",
                    span { class: "absolute left-1/2 top-0 block h-0 w-0",
                        for (index, preview) in preview_items.iter().enumerate() {
                            FanCard {
                                key: "{preview.id}",
                                index,
                                count: preview_items.len(),
                                open_p,
                                reduce: reduce_value,
                                {preview.content.clone()}
                            }
                        }
                    }
                }
                // Front sleeve with the title and count.
                span {
                    class: "absolute inset-x-0 bottom-0 z-20 overflow-hidden rounded-2xl border border-foreground/10 bg-background/60 backdrop-blur-2xl [backface-visibility:hidden] [transform-origin:center_bottom]",
                    style: front_style,
                    span { class: "flex h-16 items-center px-4",
                        span { class: "line-clamp-2 text-xl font-medium leading-tight text-foreground", "{title_for_label}" }
                    }
                    span { class: "flex h-12 items-center justify-between gap-3 border-t border-foreground/10 px-4",
                        span { class: "shrink-0 text-sm font-medium text-foreground/70", "{count_text}" }
                        span { class: "truncate text-sm text-muted-foreground", "{description}" }
                    }
                }
            }

            if is_expanded {
                button {
                    r#type: "button",
                    tabindex: "-1",
                    aria_label: "Close file overlay",
                    class: "fixed inset-0 z-50 cursor-default bg-background/80 backdrop-blur-xl",
                    onclick: move |_| close_overlay.call(()),
                }
                div {
                    role: "dialog",
                    "aria-modal": "true",
                    aria_label: "{title_for_label}",
                    class: "pointer-events-none fixed inset-x-6 inset-y-8 z-50 flex items-start justify-center overflow-y-auto sm:items-center",
                    div { class: "pointer-events-auto relative z-10 w-full max-w-[61rem]",
                        div { class: "mb-6 flex items-center justify-between gap-4",
                            div {
                                h2 { id: "{dialog_id}", class: "text-xl font-medium text-foreground", "{title_for_label}" }
                                p { class: "mt-1 text-sm text-muted-foreground", "{count_text}" }
                            }
                            button {
                                r#type: "button",
                                aria_label: "Close {title_for_label}",
                                class: "flex size-10 items-center justify-center rounded-full border border-foreground/10 bg-background/50 text-muted-foreground backdrop-blur-xl transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                                onclick: move |_| close_overlay.call(()),
                                X { class: Some("size-4".to_string()) }
                            }
                        }
                        div { class: "grid grid-cols-2 place-items-center gap-3 sm:grid-cols-3 lg:grid-cols-5",
                            for preview in preview_items.iter() {
                                OverlayCard { key: "{preview.id}", reduce: reduce_value, {preview.content.clone()} }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One card in the fan.
#[component]
fn FanCard(index: usize, count: usize, open_p: f64, reduce: bool, children: Element) -> Element {
    let transform = preview_transform(index, count);
    let x = transform.x * (1.0 + 0.4 * open_p);
    let y = transform.y - 8.0 * open_p;
    let rotate = transform.rotate * (1.0 + 0.3 * open_p);
    let scale = transform.scale * (1.0 + 0.02 * open_p);
    let opacity = (transform.opacity + 0.18 * open_p).min(1.0);
    let style = if reduce {
        format!(
            "position:absolute;left:0;top:0;margin-left:-48px;width:96px;height:160px;z-index:{};opacity:{opacity}",
            transform.z_index
        )
    } else {
        format!(
            "position:absolute;left:0;top:0;margin-left:-48px;width:96px;height:160px;z-index:{};opacity:{opacity};transform:translate({x}px,{y}px) rotate({rotate}deg) scale({scale})",
            transform.z_index
        )
    };

    rsx! {
        span {
            class: "block overflow-hidden rounded-lg border border-foreground/10 bg-background/45 backdrop-blur-lg will-change-transform",
            style,
            {children}
        }
    }
}

/// One card in the overlay grid.
#[component]
fn OverlayCard(reduce: bool, children: Element) -> Element {
    let mut progress = use_motion(0.0f32).expect("0.0 is finite");
    use_effect(move || {
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(layout_spring())
        };
        let _ = progress.animate_to(1.0, config);
    });
    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if reduce {
        format!("opacity:{p}")
    } else {
        format!("opacity:{p};transform:scale({})", 0.92 + 0.08 * p)
    };

    rsx! {
        div {
            class: "aspect-[3/4] w-full max-w-40 overflow-hidden rounded-xl border border-foreground/10 bg-background/50 backdrop-blur-xl",
            style,
            {children}
        }
    }
}

//! `motion/bounce-sidebar.tsx` — a nav rail whose selection dot lands with a
//! sideways bounce.
//!
//! The dot's `y` is a lightly underdamped spring whose stiffness drops, damping
//! rises and mass grows as the jump gets longer, so a long jump settles rather
//! than flinging. The source carries the bounce on motion's `arc()` path
//! modifier; `offset-path` is not portable across the renderers, so the same
//! sideways bulge is reproduced in the transform: the dot's `x` is
//! `-bulge·sin(π·p)` where `p` is the spring's progress, and
//! `bulge = min(0.26, 40/travel)·travel/2` (adjacent rows ≈ 5px, a long jump
//! peaks near 20px). Zero at both ends, maximal in the middle — the shape of the
//! arc, in the domain the port can write.

use std::cell::Cell;
use std::rc::Rc;

use crate::prelude::*;
use beui_core::platform::{js_string, json, next_scope_id, Rect};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

const DOT_SIZE: f64 = 6.0;

/// `BOUNCE_SPRING`, verbatim.
const BOUNCE_SPRING: Spring = Spring {
    stiffness: 280.0,
    damping: 18.0,
    mass: 0.3,
};

const ARC_MAX_STRENGTH: f64 = 0.26;
const ARC_MAX_BULGE: f64 = 40.0;

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// One nav row.
#[derive(Clone, PartialEq)]
pub struct BounceSidebarItem {
    pub id: String,
    pub label: String,
    pub href: Option<String>,
    pub icon: Option<Element>,
    pub disabled: bool,
    pub target: Option<String>,
    pub rel: Option<String>,
}

impl BounceSidebarItem {
    /// A row with a label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            href: None,
            icon: None,
            disabled: false,
            target: None,
            rel: None,
        }
    }

    /// Attach an icon.
    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Turn the row into a link.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
}

/// Props for [`BounceSidebar`].
#[derive(Props, Clone, PartialEq)]
pub struct BounceSidebarProps {
    pub items: Vec<BounceSidebarItem>,
    #[props(default)]
    pub value: Option<String>,
    #[props(default)]
    pub default_value: Option<String>,
    #[props(default)]
    pub on_value_change: Option<EventHandler<String>>,
    #[props(default = "Sidebar navigation".to_string())]
    pub aria_label: String,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub list_class: Option<String>,
    #[props(default)]
    pub item_class: Option<String>,
    #[props(default)]
    pub indicator_class: Option<String>,
}

/// A sidebar whose dot bounces sideways between rows.
#[component]
pub fn BounceSidebar(props: BounceSidebarProps) -> Element {
    let BounceSidebarProps {
        items,
        value,
        default_value,
        on_value_change,
        aria_label,
        class,
        list_class,
        item_class,
        indicator_class,
    } = props;

    let reduce = use_reduced_motion();
    let list_scope = use_hook(next_scope_id);
    let mut internal = use_signal(|| default_value.clone());
    let is_controlled = value.is_some();
    let active = match value.clone() {
        Some(active) => active,
        None => internal().unwrap_or_default(),
    };
    let selected_index = items.iter().position(|item| item.id == active).unwrap_or(0);
    let selected_value = items
        .get(selected_index)
        .map(|item| item.id.clone())
        .unwrap_or_default();

    let metrics = use_signal(Vec::<Rect>::new);
    let dot = use_motion(0.0f32).expect("0.0 is finite");
    let mut arc_from = use_signal(|| 0.0f64);
    let mut arc_to = use_signal(|| 0.0f64);
    let mut arc_bulge = use_signal(|| 0.0f64);
    let has_position: Rc<Cell<bool>> = use_hook(|| Rc::new(Cell::new(false)));

    // Measure every row's top and height relative to the list.
    let measure = move || {
        let mut metrics = metrics;
        let selector = format!("[data-beui-bounce-list=\"{list_scope}\"]");
        spawn(async move {
            let script = format!(
                "const list = document.querySelector({});\n\
                 if (!list) return null;\n\
                 const base = list.getBoundingClientRect();\n\
                 return Array.from(list.querySelectorAll('[data-beui-bounce-item]')).map(function (el) {{ const r = el.getBoundingClientRect(); return {{ left: 0, top: r.top - base.top, width: r.width, height: r.height }}; }});",
                js_string(&selector)
            );
            if let Some(next) = json::<Vec<Rect>>(&script).await {
                metrics.set(next);
            }
        });
    };
    use_effect(measure);

    let mut select_value = move |id: String| {
        if !is_controlled {
            internal.set(Some(id.clone()));
        }
        if let Some(handler) = on_value_change {
            handler.call(id);
        }
    };
    let has_position_effect = has_position.clone();
    use_effect(use_reactive((&selected_index,), move |(selected_index,)| {
        let Some(metric) = metrics().get(selected_index).copied() else {
            measure();
            return;
        };
        let destination = metric.top + (metric.height - DOT_SIZE) / 2.0;
        if !has_position_effect.get() || reduce() {
            let mut handle = dot;
            let _ = handle.animate_to(destination as f32, beui_core::ease::tween_ease_out(0));
            arc_from.set(destination);
            arc_to.set(destination);
            arc_bulge.set(0.0);
            has_position_effect.set(true);
            return;
        }
        let from = f64::from(current(dot));
        let distance = destination - from;
        let travel = distance.abs();
        let long_jump = ((travel - 48.0) / 120.0).clamp(0.0, 1.0);
        let bulge = (ARC_MAX_STRENGTH.min(ARC_MAX_BULGE / travel.max(1.0))) * travel / 2.0;
        arc_from.set(from);
        arc_to.set(destination);
        arc_bulge.set(if distance == 0.0 { 0.0 } else { bulge });
        let adjusted = AnimationConfig::spring(Spring {
            stiffness: BOUNCE_SPRING.stiffness - 60.0 * long_jump as f32,
            damping: BOUNCE_SPRING.damping + long_jump as f32,
            mass: BOUNCE_SPRING.mass + 0.15 * long_jump as f32,
        });
        let mut handle = dot;
        let _ = handle.animate_to(destination as f32, adjusted);
    }));

    let y = f64::from(current(dot));
    let from = arc_from();
    let to = arc_to();
    let bulge = arc_bulge();
    let span = to - from;
    let progress = if span.abs() < 0.001 {
        1.0
    } else {
        ((y - from) / span).clamp(0.0, 1.0)
    };
    let x = -bulge * (std::f64::consts::PI * progress).sin();
    let dot_style = format!("transform:translateX({x}px) translateY({y}px)");
    let dot_class = cn!(
        "absolute top-0 left-3 h-1.5 w-1.5 rounded-full bg-primary",
        indicator_class,
    );
    let list_class = cn!("relative flex list-none flex-col gap-1.5 pl-6", list_class);
    let class = cn!("relative", class);
    let reduce_value = reduce();

    rsx! {
        nav { aria_label, class,
            ul { "data-beui-bounce-list": "{list_scope}", class: list_class,
                li { aria_hidden: "true", role: "presentation", class: "pointer-events-none absolute inset-0",
                    span { class: dot_class, style: dot_style }
                }
                for item in items.iter() {
                    {
                        let is_active = item.id == selected_value;
                        let id = item.id.clone();
                        let interactive_class = cn!(
                            "flex min-h-9 w-full items-center gap-2 rounded-lg px-2 text-left text-sm font-medium outline-none transition-colors",
                            "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                            if is_active { "text-foreground" } else { "text-muted-foreground hover:text-foreground" },
                            if item.disabled { "cursor-not-allowed opacity-40" } else { "" },
                            item_class.clone(),
                        );
                        let icon = item.icon.clone();
                        let label = item.label.clone();
                        let disabled = item.disabled;
                        let select_id = id.clone();
                        let anchor_target = item.target.clone();
                        let anchor_rel = item.rel.clone();
                        let href = item.href.clone();
                        let _ = reduce_value;
                        rsx! {
                            li { key: "{id}", "data-beui-bounce-item": "", class: "relative",
                                if let Some(href) = href {
                                    a {
                                        href,
                                        target: anchor_target,
                                        rel: anchor_rel,
                                        aria_current: if is_active { Some("page") } else { None },
                                        "aria-disabled": if disabled { Some("true") } else { None },
                                        "data-active": if is_active { Some("true") } else { None },
                                        tabindex: if disabled { Some("-1") } else { None },
                                        class: interactive_class,
                                        onclick: move |event: Event<MouseData>| {
                                            if disabled {
                                                event.prevent_default();
                                                return;
                                            }
                                            select_value(select_id.clone());
                                        },
                                        if let Some(icon) = icon {
                                            span { aria_hidden: "true", class: "shrink-0", {icon} }
                                        }
                                        span { "{label}" }
                                    }
                                } else {
                                    button {
                                        r#type: "button",
                                        disabled,
                                        aria_current: if is_active { Some("page") } else { None },
                                        "data-active": if is_active { Some("true") } else { None },
                                        class: interactive_class,
                                        onclick: move |_| select_value(select_id.clone()),
                                        if let Some(icon) = icon {
                                            span { aria_hidden: "true", class: "shrink-0", {icon} }
                                        }
                                        span { "{label}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

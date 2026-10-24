//! `components/motion/expandable-tabs.tsx` — a Dynamic-Island tab bar.
//!
//! The shell is sized by two springs: closed it is the bar alone, open it grows
//! to the measured content size. The active tab widens to fit its label while
//! the label unclips (width + opacity + blur), and the panel enters from just
//! above the dock.
//!
//! Deviations from the source:
//!
//! - `items` carries `label`/`content` as `String`s and has no `icon` slot,
//!   because `Vec<_>` props need `Clone + PartialEq` and an `Element` has
//!   neither;
//! - `duration`+`bounce` springs go through [`spring_from_visual_duration`] as
//!   if they were `visualDuration`+`bounce` (the lane-wide approximation).

use dioxus::prelude::*;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

use crate::motion::spring_from_visual_duration;
use beui_core::ease::tween_ease_out;
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform;
use beui_core::presence::{
    motion_style, presence_css, use_presence_style, AnimatePresence, PresenceConfig, PresenceMode,
};
use beui_core::{cn, use_reduced_motion};

const BAR_H: f64 = 52.0;
const TAB_W: f64 = 32.0;
const BAR_X: f64 = 16.0;
const BAR_GAP: f64 = 4.0;
const ROOT_BORDER: f64 = 2.0;
const ICON_W: f64 = 16.0;
const ACTIVE_LEFT_PAD: f64 = 10.0;
const ACTIVE_RIGHT_PAD: f64 = 16.0;
const LABEL_GAP: f64 = 7.0;
const PANEL_DOCK_GAP: f64 = 4.0;

/// The icon slot. An `Element` cannot live in a `Vec<_>` prop, so the port
/// names a small set of Lucide glyphs instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExpandableTabsIcon {
    #[default]
    None,
    Home,
    Mail,
    Settings,
    Search,
    Bell,
}

fn icon_element(icon: ExpandableTabsIcon) -> Element {
    match icon {
        ExpandableTabsIcon::None => rsx! {},
        ExpandableTabsIcon::Home => rsx! { crate::icons::HomeIcon { class: "h-4 w-4" } },
        ExpandableTabsIcon::Mail => rsx! { crate::icons::MailIcon { class: "h-4 w-4" } },
        ExpandableTabsIcon::Settings => rsx! { crate::icons::SettingsIcon { class: "h-4 w-4" } },
        ExpandableTabsIcon::Search => rsx! { crate::icons::SearchIcon { class: "h-4 w-4" } },
        ExpandableTabsIcon::Bell => rsx! { crate::icons::BellIcon { class: "h-4 w-4" } },
    }
}

/// One tab.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpandableTabsItem {
    pub id: String,
    pub label: String,
    pub content: String,
    pub icon: ExpandableTabsIcon,
}

impl ExpandableTabsItem {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            content: content.into(),
            icon: ExpandableTabsIcon::None,
        }
    }

    pub fn icon(mut self, icon: ExpandableTabsIcon) -> Self {
        self.icon = icon;
        self
    }
}

fn closed_width(count: usize) -> f64 {
    count as f64 * TAB_W + count.saturating_sub(1) as f64 * BAR_GAP + BAR_X + ROOT_BORDER
}

fn closed_height() -> f64 {
    BAR_H + ROOT_BORDER
}

/// `ExpandableTabs`.
#[component]
pub fn ExpandableTabs(
    items: Vec<ExpandableTabsItem>,
    #[props(default = None)] value: Option<String>,
    #[props(default = None)] default_value: Option<String>,
    #[props(default = None)] on_value_change: Option<EventHandler<Option<String>>>,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut internal = use_signal(|| default_value.clone());
    let controlled = value.is_some();
    let fallback = internal();
    let active_id = value.clone().or(fallback);
    let active = items
        .iter()
        .find(|item| Some(&item.id) == active_id.as_ref())
        .cloned();

    let active_signal = use_signal(|| active_id.clone());
    let mut active_sync = active_signal;
    if *active_sync.peek() != active_id {
        active_sync.set(active_id.clone());
    }
    let open_signal = use_signal(|| active_id.is_some());
    let mut open_sync = open_signal;
    if *open_sync.peek() != active_id.is_some() {
        open_sync.set(active_id.is_some());
    }

    let set_active = EventHandler::new(move |next: Option<String>| {
        if !controlled {
            internal.set(next.clone());
        }
        if let Some(callback) = on_value_change {
            callback.call(next);
        }
    });

    // Outside click / Escape closes, like an open menu.
    let close = EventHandler::new(move |_| set_active.call(None));
    let scope = use_dismiss(open_signal, close, DismissOptions::default());

    let content_size = use_signal(|| (0.0_f64, 0.0_f64));
    let label_widths = use_signal(Vec::<(String, f64)>::new);

    let count = items.len();
    let shell_width = use_motion(closed_width(count) as f32).expect("finite");
    let shell_height = use_motion(closed_height() as f32).expect("finite");
    let width_signal = shell_width;
    let height_signal = shell_height;
    let reduce_for_shell = reduce;
    use_effect(move || {
        let open = (active_signal)().is_some();
        let (content_w, content_h) = (content_size)();
        let closed_w = closed_width(count);
        let closed_h = closed_height();
        let (target_w, target_h) = if open {
            (
                (content_w + ROOT_BORDER).max(closed_w),
                (content_h + ROOT_BORDER).max(closed_h),
            )
        } else {
            (closed_w, closed_h)
        };
        let config = if reduce_for_shell {
            tween_ease_out(1)
        } else {
            AnimationConfig::spring(spring_from_visual_duration(0.58, 0.06))
        };
        let mut w = width_signal;
        let mut h = height_signal;
        let _ = w.animate_to(target_w as f32, config.clone());
        let _ = h.animate_to(target_h as f32, config);
    });

    let sizer_items = items.clone();
    let bar_items = items.clone();
    let measure_items = items.clone();
    let layers: Vec<String> = active.iter().map(|item| item.id.clone()).collect();
    let content = active
        .as_ref()
        .map(|item| item.content.clone())
        .unwrap_or_default();
    let active_now = active.as_ref().map(|item| item.id.clone());

    rsx! {
        div {
            "data-beui-scope": scope.attr(),
            style: format!(
                "width:{}px;height:{}px;transform-origin:bottom center",
                shell_width.get_value(),
                shell_height.get_value(),
            ),
            class: cn!(
                "relative overflow-hidden rounded-[26px] border border-border bg-card",
                class.as_deref().unwrap_or(""),
            ),
            div {
                "aria-hidden": "true",
                class: "pointer-events-none invisible absolute left-0 top-0 grid w-max px-2 pt-2",
                style: format!("padding-bottom:{}px", BAR_H + PANEL_DOCK_GAP),
                onmounted: move |event| {
                    let mut size = content_size;
                    spawn(async move {
                        if let Some(rect) = platform::client_rect(&event.data()).await {
                            let next = (rect.width, rect.height);
                            if *size.peek() != next {
                                size.set(next);
                            }
                        }
                    });
                },
                for item in sizer_items {
                    div { key: "{item.id}", class: "col-start-1 row-start-1 w-max", "{item.content}" }
                }
            }
            div {
                class: "absolute left-0 right-0 top-0 z-10 overflow-hidden px-2 pt-2",
                style: format!("bottom:{}px", BAR_H + PANEL_DOCK_GAP),
                AnimatePresence {
                    initial: false,
                    mode: PresenceMode::PopLayout,
                    for id in layers {
                        ContentPanel { key: "{id}", content: content.clone(), reduce }
                    }
                }
            }
            div {
                role: "tablist",
                "aria-label": "Navigation tabs",
                "aria-orientation": "horizontal",
                style: format!("height:{}px", BAR_H),
                class: "absolute bottom-0 left-0 z-20 flex w-full items-center justify-between gap-1 p-2",
                for item in bar_items {
                    ExpandableTab {
                        key: "{item.id}",
                        label_width: (label_widths)()
                            .into_iter()
                            .find(|(id, _)| *id == item.id)
                            .map(|(_, width)| width)
                            .unwrap_or(0.0),
                        is_active: active_now.as_deref() == Some(item.id.as_str()),
                        id: item.id.clone(),
                        label: item.label.clone(),
                        icon: item.icon,
                        reduce,
                        on_toggle: move |id: String| {
                            let next = if active_sync.peek().as_deref() == Some(id.as_str()) {
                                None
                            } else {
                                Some(id)
                            };
                            set_active.call(next);
                        },
                    }
                }
            }
        }
        div {
            "aria-hidden": "true",
            class: "pointer-events-none fixed left-0 top-0 -z-10 flex opacity-0",
            for item in measure_items {
                span {
                    key: "{item.id}",
                    class: "whitespace-nowrap text-sm font-medium leading-none",
                    onmounted: {
                        let id = item.id.clone();
                        move |event| {
                            let mut widths = label_widths;
                            let id = id.clone();
                            spawn(async move {
                                if let Some(rect) = platform::client_rect(&event.data()).await {
                                    let width = rect.width.ceil();
                                    let mut list = widths.write();
                                    if let Some(entry) = list.iter_mut().find(|(key, _)| *key == id) {
                                        entry.1 = width;
                                    } else {
                                        list.push((id, width));
                                    }
                                }
                            });
                        }
                    },
                    "{item.label}"
                }
            }
        }
    }
}

#[component]
fn ExpandableTab(
    id: String,
    label: String,
    icon: ExpandableTabsIcon,
    is_active: bool,
    label_width: f64,
    reduce: bool,
    on_toggle: EventHandler<String>,
) -> Element {
    let target_width = if is_active {
        (ACTIVE_LEFT_PAD + ICON_W + LABEL_GAP + label_width + ACTIVE_RIGHT_PAD).max(TAB_W)
    } else {
        TAB_W
    };
    let width = use_motion(target_width as f32).expect("finite");
    let label_progress = use_motion(if is_active { 1.0f32 } else { 0.0 }).expect("finite");

    let mut active_signal = use_signal(|| is_active);
    if *active_signal.peek() != is_active {
        active_signal.set(is_active);
    }
    let mut label_width_signal = use_signal(|| label_width);
    if *label_width_signal.peek() != label_width {
        label_width_signal.set(label_width);
    }
    let mut width_signal = width;
    let mut label_signal = label_progress;
    use_effect(move || {
        let active = active_signal();
        let width_target = if active {
            (ACTIVE_LEFT_PAD + ICON_W + LABEL_GAP + label_width_signal() + ACTIVE_RIGHT_PAD)
                .max(TAB_W)
        } else {
            TAB_W
        };
        let width_config = if reduce {
            tween_ease_out(1)
        } else {
            AnimationConfig::spring(spring_from_visual_duration(0.46, 0.04))
        };
        let label_config = if reduce {
            tween_ease_out(1)
        } else if active {
            AnimationConfig::spring(spring_from_visual_duration(0.38, 0.03))
        } else {
            tween_ease_out(160)
        };
        let _ = width_signal.animate_to(width_target as f32, width_config);
        let _ = label_signal.animate_to(if active { 1.0 } else { 0.0 }, label_config);
    });

    let progress = f64::from(label_progress.get_value());
    let blur = if reduce { 0.0 } else { 3.0 * (1.0 - progress) };

    rsx! {
        button {
            r#type: "button",
            role: "tab",
            "aria-selected": is_active,
            "aria-label": "{label}",
            style: format!("width:{}px", width.get_value()),
            class: cn!(
                "relative isolate flex h-9 min-w-8 shrink-0 items-center justify-center overflow-hidden rounded-[18px] px-2 text-sm font-medium outline-none",
                "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                if is_active { "min-w-0 justify-start pl-2.5 pr-4 text-foreground" } else { "text-muted-foreground hover:text-foreground" },
            ),
            onclick: move |_| on_toggle.call(id.clone()),
            if is_active {
                span { "aria-hidden": "true", class: "absolute inset-0 -z-10 rounded-[18px] bg-foreground/10" }
            }
            span {
                class: "grid shrink-0 place-items-center",
                style: format!("width:{}px;height:{}px", ICON_W, ICON_W),
                {icon_element(icon)}
            }
            span {
                "aria-hidden": "true",
                style: format!(
                    "display:inline-block;overflow:hidden;white-space:nowrap;width:{}px;opacity:{progress};margin-left:{}px;filter:blur({blur}px)",
                    (progress * label_width).max(0.0),
                    progress * LABEL_GAP,
                ),
                "{label}"
            }
        }
    }
}

/// The panel body. Its own presence boundary so the exit can play out.
#[component]
fn ContentPanel(content: String, reduce: bool) -> Element {
    let handle = use_presence_style(panel_config(reduce)).expect("static presence config");
    let css = presence_css(&handle);
    rsx! {
        div {
            style: format!("{css};transform-origin:top center;will-change:transform,opacity,filter"),
            class: "w-max",
            "{content}"
        }
    }
}

fn panel_config(reduce: bool) -> PresenceConfig {
    if reduce {
        return PresenceConfig::with_transitions(
            motion_style! { opacity: 0.0 },
            motion_style! { opacity: 1.0 },
            motion_style! { opacity: 0.0 },
            tween_ease_out(150),
            tween_ease_out(80),
        );
    }
    let mut enter = motion_style! { y: -8.0, scale: 0.98, opacity: 0.0 };
    enter.add_css_property("filter", "blur(4px)");
    let mut center = motion_style! { y: 0.0, scale: 1.0, opacity: 1.0 };
    center.add_css_property("filter", "blur(0px)");
    let mut exit = motion_style! { y: -6.0, scale: 0.98, opacity: 0.0 };
    exit.add_css_property("filter", "blur(4px)");
    PresenceConfig::with_transitions(
        enter,
        center,
        exit,
        AnimationConfig::spring(spring_from_visual_duration(0.46, 0.08)),
        tween_ease_out(80),
    )
}

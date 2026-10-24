//! `ExpandableActionBar` — the beUI preview, same copy and structure.

use beui::icons::{Archive, Bell, Copy, Download, Send, Settings};
use beui::prelude::*;
use beui::{ExpandableActionBar, ExpandableActionBarClassNames, ExpandableActionBarItem};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, Spring};

const SHORTCUTS: [(&str, &str, &str, &str); 6] = [
    (
        "send",
        "Send",
        "S",
        "motion-safe:group-hover:animate-action-send",
    ),
    (
        "copy",
        "Copy",
        "C",
        "motion-safe:group-hover:animate-action-copy",
    ),
    (
        "download",
        "Export",
        "E",
        "motion-safe:group-hover:animate-action-download",
    ),
    (
        "archive",
        "Archive",
        "",
        "motion-safe:group-hover:animate-action-archive",
    ),
    (
        "alerts",
        "Alerts",
        "",
        "motion-safe:group-hover:animate-action-bell",
    ),
    (
        "settings",
        "Settings",
        "",
        "motion-safe:group-hover:animate-action-settings",
    ),
];

fn icon(id: &str, class: &str) -> Element {
    match id {
        "send" => rsx! { Send { class: Some(class.to_string()) } },
        "copy" => rsx! { Copy { class: Some(class.to_string()) } },
        "download" => rsx! { Download { class: Some(class.to_string()) } },
        "archive" => rsx! { Archive { class: Some(class.to_string()) } },
        "alerts" => rsx! { Bell { class: Some(class.to_string()) } },
        _ => rsx! { Settings { class: Some(class.to_string()) } },
    }
}

fn items(active_id: &str) -> Vec<ExpandableActionBarItem> {
    SHORTCUTS
        .iter()
        .map(|(id, label, shortcut, anim)| {
            let mut item =
                ExpandableActionBarItem::new(*id, *label, icon(id, &format!("h-4 w-4 {anim}")));
            if !shortcut.is_empty() {
                item = item.shortcut(*shortcut);
            }
            if *id == "alerts" {
                item = item.badge("3");
            }
            ExpandableActionBarItem {
                active: *id == active_id,
                ..item
            }
        })
        .collect()
}

/// The preview's expand/collapse button arrow, interpolated between the two paths.
fn arrow_path(progress: f64, second: bool) -> String {
    let (from, to) = if second {
        (
            [15.0, 3.0, 21.0, 3.0, 21.0, 9.0],
            [14.0, 4.0, 14.0, 10.0, 20.0, 10.0],
        )
    } else {
        (
            [9.0, 21.0, 3.0, 21.0, 3.0, 15.0],
            [10.0, 20.0, 10.0, 14.0, 4.0, 14.0],
        )
    };
    let value = |index: usize| from[index] + (to[index] - from[index]) * progress;
    format!(
        "M {} {} L {} {} L {} {}",
        value(0),
        value(1),
        value(2),
        value(3),
        value(4),
        value(5)
    )
}

#[component]
pub fn ExpandableActionBarPage() -> Element {
    let mut expanded = use_signal(|| false);
    let active = use_signal(|| "send".to_string());
    let mut progress = use_motion(0.0f32).expect("0.0 is finite");
    use_effect(move || {
        let target = if expanded() { 1.0 } else { 0.0 };
        let _ = progress.animate_to(
            target,
            AnimationConfig::spring(Spring {
                stiffness: 300.0,
                damping: 30.0,
                mass: 1.0,
            }),
        );
    });
    let p = f64::from(*progress.current().read());
    let path_first = arrow_path(p, false);
    let path_second = arrow_path(p, true);
    let expand_style_a = format!(
        "opacity:{};transform:translateY({}px);filter:blur({}px)",
        if expanded() { 0.0 } else { 1.0 },
        if expanded() { -25.0 } else { 0.0 },
        if expanded() { 4.0 } else { 0.0 }
    );
    let expand_style_b = format!(
        "opacity:{};transform:translateY({}px);filter:blur({}px)",
        if expanded() { 1.0 } else { 0.0 },
        if expanded() { 0.0 } else { 25.0 },
        if expanded() { 0.0 } else { 4.0 }
    );

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "ExpandableActionBar" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A rail of icon actions whose labels unfold on hover, focus or a first tap. The highlight is a layoutId pill handed between items through beui-core's flip registry."
                }
            }
            div { class: "flex min-h-72 w-full flex-col items-center justify-center gap-6",
                div { class: "flex min-h-24 w-full items-center justify-center",
                    ExpandableActionBar {
                        items: items(&active()),
                        expanded: Some(expanded()),
                        on_expanded_change: Some(EventHandler::new({
                            let mut expanded = expanded;
                            move |next: bool| expanded.set(next)
                        })),
                        active_id: Some(active()),
                        on_action: Some(EventHandler::new({
                            let mut active = active;
                            move |item: ExpandableActionBarItem| active.set(item.id)
                        })),
                        class_names: ExpandableActionBarClassNames {
                            item: Some("group".to_string()),
                            ..Default::default()
                        },
                    }
                }
                div { class: "flex flex-wrap items-center justify-center gap-2",
                    button {
                        r#type: "button",
                        class: "relative flex h-9 w-[110px] items-center justify-center overflow-hidden rounded-full border border-border bg-card text-xs font-medium text-foreground transition-colors hover:border-(--color-border-strong)",
                        onclick: move |_| expanded.toggle(),
                        div { class: "flex items-center gap-1.5",
                            svg {
                                width: "24",
                                height: "24",
                                view_box: "0 0 24 24",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                class: "h-3.5 w-3.5 shrink-0",
                                path { d: "{path_first}" }
                                path { d: "{path_second}" }
                                line { x1: "14", x2: "21", y1: "10", y2: "3" }
                                line { x1: "3", x2: "10", y1: "21", y2: "14" }
                            }
                            span { class: "relative inline-grid place-items-center",
                                span {
                                    class: "col-start-1 row-start-1 whitespace-nowrap",
                                    style: "{expand_style_a};transition:opacity 300ms,transform 300ms,filter 300ms",
                                    "Expand"
                                }
                                span {
                                    class: "col-start-1 row-start-1 whitespace-nowrap",
                                    style: "{expand_style_b};transition:opacity 300ms,transform 300ms,filter 300ms",
                                    "Collapse"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

//! `motion/infinite-masonry.tsx` — a masonry feed that loads more as it is
//! scrolled.
//!
//! The source delegates placement to `@tanstack/react-virtual` with lanes,
//! which assigns each item to the shortest lane and windows the render. This
//! port keeps the lane assignment (each item goes to the shortest lane, using
//! its measured height, falling back to the estimate until it has been
//! measured), the loading/error tail, the `prefetch` trigger and the reveal
//! animation — but renders every item rather than windowing them, and replaces
//! the virtualizer's threshold check with an `IntersectionObserver` on a marker
//! placed at the `items.len() - prefetch` item's offset. Windowing is the one
//! real gap; the layout, the tail and the load trigger are the source's.

use crate::icons::AlertCircle;
use crate::prelude::*;
use crate::rect::{use_element_rect, use_element_width};
use crate::scroll::{use_in_view, InViewAmount, InViewOptions};
use beui_core::platform::next_scope_id;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};
use std::collections::HashMap;

const INBOX_FALLBACK: &str = "No items yet";

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// One feed item.
#[derive(Clone, PartialEq)]
pub struct MasonryItem {
    pub key: String,
    /// Estimated height in px, used until the item has been measured.
    pub height: f64,
    pub content: Element,
}

impl MasonryItem {
    /// An item with the default 240px estimate.
    pub fn new(key: impl Into<String>, content: Element) -> Self {
        Self {
            key: key.into(),
            height: 240.0,
            content,
        }
    }

    /// Set the height estimate.
    pub fn height(mut self, height: f64) -> Self {
        self.height = height;
        self
    }
}

/// Props for [`InfiniteMasonry`].
#[derive(Props, Clone, PartialEq)]
pub struct InfiniteMasonryProps {
    pub items: Vec<MasonryItem>,
    pub on_load_more: EventHandler<()>,
    pub has_more: bool,
    #[props(default)]
    pub loading: bool,
    #[props(default)]
    pub error: Option<Element>,
    #[props(default)]
    pub on_retry: Option<EventHandler<()>>,
    /// Replaces the default pulsing placeholder for a tail slot.
    #[props(default)]
    pub loading_item: Option<Callback<usize, Element>>,
    #[props(default)]
    pub empty_state: Option<Element>,
    #[props(default)]
    pub end_state: Option<Element>,
    #[props(default = 208.0)]
    pub min_column_width: f64,
    #[props(default = 4)]
    pub max_columns: usize,
    #[props(default = 12.0)]
    pub gap: f64,
    /// How many items from the end may still be off screen before loading.
    #[props(default = 3)]
    pub prefetch: usize,
    #[props(default = true)]
    pub animate_items: bool,
    #[props(default = "Infinite masonry feed".to_string())]
    pub aria_label: String,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub content_class: Option<String>,
    #[props(default)]
    pub item_class: Option<String>,
}

/// The default loading placeholder.
#[component]
fn DefaultLoadingItem(index: usize) -> Element {
    let height = 144 + (index % 3) * 36;
    rsx! {
        div {
            aria_hidden: "true",
            class: "animate-pulse rounded-2xl border border-border bg-card p-3",
            style: "min-height:{height}px",
            div { class: "h-3 w-2/3 rounded-full bg-muted" }
            div { class: "mt-3 h-2 w-full rounded-full bg-muted" }
            div { class: "mt-2 h-2 w-4/5 rounded-full bg-muted" }
        }
    }
}

/// The default empty state.
#[component]
fn DefaultEmptyState() -> Element {
    rsx! {
        div { class: "flex h-full min-h-64 flex-col items-center justify-center px-6 text-center",
            svg {
                "aria-hidden": "true",
                xmlns: "http://www.w3.org/2000/svg",
                width: "24",
                height: "24",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                class: "size-8 text-muted-foreground",
                path { d: "M22 12h-6l-2 3h-4l-2-3H2" }
                path { d: "M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" }
            }
            p { class: "mt-3 text-sm font-medium text-foreground", "{INBOX_FALLBACK}" }
            p { class: "mt-1 max-w-xs text-xs leading-5 text-muted-foreground",
                "New items will appear here when they become available."
            }
        }
    }
}

/// A masonry feed.
#[component]
pub fn InfiniteMasonry(props: InfiniteMasonryProps) -> Element {
    let InfiniteMasonryProps {
        items,
        on_load_more,
        has_more,
        loading,
        error,
        on_retry,
        loading_item,
        empty_state,
        end_state,
        min_column_width,
        max_columns,
        gap,
        prefetch,
        animate_items,
        aria_label,
        class,
        content_class,
        item_class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let scope = use_hook(next_scope_id);
    let scroll_selector = format!("[data-beui-masonry-scroll=\"{scope}\"]");
    let width_stream = use_element_width();
    let width = width_stream.width();
    let measured: Signal<HashMap<String, f64>> = use_signal(HashMap::new);

    let has_error = error.is_some();
    let empty = items.is_empty() && !has_more && !loading && !has_error;
    let columns = if width > 0.0 {
        (((width + gap) / (min_column_width + gap)).floor() as usize).clamp(1, max_columns.max(1))
    } else {
        1
    };
    let column_width = if columns > 0 {
        ((width - gap * (columns.saturating_sub(1)) as f64) / columns as f64).max(0.0)
    } else {
        0.0
    };

    // Lane assignment: each item joins the shortest lane, as tanstack's lanes
    // do, using the measured height once the item has reported one.
    let mut lane_heights = vec![0.0f64; columns];
    let mut tops: Vec<f64> = Vec::with_capacity(items.len());
    let mut lanes: Vec<usize> = Vec::with_capacity(items.len());
    let measured_map = measured();
    for item in items.iter() {
        let height = measured_map.get(&item.key).copied().unwrap_or(item.height);
        let lane = lane_heights
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, _)| index)
            .unwrap_or(0);
        let top = lane_heights[lane];
        lane_heights[lane] = top + height + gap;
        tops.push(top);
        lanes.push(lane);
    }
    let total_height = lane_heights.iter().cloned().fold(0.0f64, f64::max);

    let initial_count = use_hook(|| items.len());
    let mut revealed: Signal<std::collections::HashSet<String>> = use_signal(Default::default);
    let tail_count = if has_error {
        1
    } else if loading {
        columns
    } else {
        0
    };
    let tail_tops = lane_heights.clone();
    let tallest = tail_tops.iter().cloned().fold(0.0f64, f64::max);

    // The load trigger sits at the offset of the `prefetch`-th item from the
    // end: once that item is on screen, the tail is close enough to fetch.
    let trigger_index = items.len().saturating_sub(prefetch.max(1));
    let trigger_top = tops.get(trigger_index).copied().unwrap_or(0.0);
    let in_view = use_in_view(InViewOptions {
        once: false,
        amount: InViewAmount::default(),
        root: Some(scroll_selector.clone()),
    });
    let mut pending = use_signal(|| false);
    use_effect(move || {
        if in_view.value() && has_more && !loading && !has_error && !pending() {
            pending.set(true);
            on_load_more.call(());
        }
    });
    use_effect(use_reactive((&loading,), move |(loading,)| {
        if !loading {
            pending.set(false);
        }
    }));

    if empty {
        return rsx! {
            section {
                aria_label,
                class: cn!(
                    "w-full overflow-hidden rounded-3xl border border-border bg-background",
                    class,
                ),
                if let Some(empty) = empty_state {
                    {empty}
                } else {
                    DefaultEmptyState {}
                }
            }
        };
    }

    // Tail slots continue each lane where its last item stopped.
    let tail_styles: Vec<String> = (0..tail_count)
        .map(|lane| {
            let top = tail_tops.get(lane).copied().unwrap_or(tallest);
            format!(
                "width:{column_width}px;transform:translate3d({}px, {top}px, 0)",
                lane as f64 * (column_width + gap)
            )
        })
        .collect();

    let scroll_class = cn!(
        "w-full contain-[layout_paint] overflow-y-auto overscroll-none rounded-3xl border border-border bg-background p-3 [overflow-anchor:none] [scrollbar-gutter:stable]",
        class,
    );
    let content_class = cn!("relative w-full", content_class);

    rsx! {
        section {
            "data-beui-masonry-scroll": "{scope}",
            aria_label,
            aria_busy: if loading { "true" } else { "false" },
            class: scroll_class,
            div {
                "data-beui-width": width_stream.attr(),
                class: content_class,
                style: "height:{total_height}px",
                for (index, item) in items.iter().enumerate() {
                    MasonryCell {
                        key: "{item.key}",
                        item: item.clone(),
                        lane: lanes.get(index).copied().unwrap_or(0),
                        top: tops.get(index).copied().unwrap_or(0.0),
                        column_width,
                        gap,
                        animate: animate_items && !reduce_value && index >= initial_count && !revealed().contains(&item.key),
                        reduce: reduce_value,
                        class: item_class.clone(),
                        measured,
                        on_revealed: EventHandler::new(move |key: String| {
                            revealed.write().insert(key);
                        }),
                    }
                }
                for lane in 0..tail_count {
                    div {
                        key: "tail-{lane}",
                        class: "absolute left-0 top-0 will-change-transform",
                        style: tail_styles[lane].clone(),
                        if has_error {
                            if lane == 0 {
                                div { class: "flex min-h-36 flex-col items-start justify-center rounded-2xl border border-destructive/20 bg-destructive/5 p-4",
                                    div { class: "flex items-center gap-2 text-destructive",
                                        AlertCircle { class: Some("size-4".to_string()) }
                                        p { class: "text-sm font-medium", "Couldn’t load more" }
                                    }
                                    div { class: "mt-2 text-xs leading-5 text-muted-foreground",
                                        if let Some(error) = error.clone() {
                                            {error}
                                        }
                                    }
                                    if let Some(retry) = on_retry {
                                        button {
                                            r#type: "button",
                                            class: "mt-3 min-h-10 rounded-full border border-border bg-background px-4 text-xs font-medium text-foreground outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                                            onclick: move |_| retry.call(()),
                                            "Try again"
                                        }
                                    }
                                }
                            }
                        } else if let Some(loading_item) = loading_item {
                            {loading_item.call(lane)}
                        } else {
                            DefaultLoadingItem { index: lane }
                        }
                    }
                }
                // The virtualizer's threshold check, as a marker.
                div {
                    "data-beui-inview": in_view.attr(),
                    aria_hidden: "true",
                    class: "pointer-events-none absolute h-px w-px",
                    style: "left:0;top:{trigger_top}px",
                }
            }
            if !has_more && !items.is_empty() {
                if let Some(end_state) = end_state {
                    div { class: "py-4 text-center text-xs text-muted-foreground", {end_state} }
                }
            }
            span { class: "sr-only", aria_live: "polite",
                if loading {
                    "Loading more items"
                }
            }
        }
    }
}

/// One absolutely placed item.
#[component]
fn MasonryCell(
    item: MasonryItem,
    lane: usize,
    top: f64,
    column_width: f64,
    gap: f64,
    animate: bool,
    reduce: bool,
    #[props(default)] class: Option<String>,
    measured: Signal<HashMap<String, f64>>,
    on_revealed: EventHandler<String>,
) -> Element {
    let rect = use_element_rect();
    let mut progress = use_motion(if animate { 0.0f32 } else { 1.0 }).expect("0/1 are finite");

    // Reveal: the source delays by `min(lane, 3) * 40ms` and uses SPRING_PANEL
    // for the lift with a 200ms EASE_OUT fade.
    let key = item.key.clone();
    let rect_signal = rect;
    use_effect(move || {
        if !animate {
            return;
        }
        let delay = (lane.min(3) as u64) * 40;
        spawn(async move {
            let _ = beui_core::platform::json::<bool>(&format!(
                "await new Promise((resolve) => setTimeout(resolve, {delay}));\nreturn true;"
            ))
            .await;
            let _ = progress.animate_to(1.0, AnimationConfig::spring(SPRING_PANEL));
        });
        on_revealed.call(key.clone());
        rect_signal.refresh();
    });

    // Report the measured height back so the lane assignment can use it.
    use_effect(use_reactive((&column_width,), move |(column_width,)| {
        let _ = column_width;
        rect.refresh();
    }));
    use_effect(move || {
        if let Some(box_rect) = rect.get() {
            let height = box_rect.height;
            let key = item.key.clone();
            let mut measured = measured;
            measured.write().insert(key, height);
        }
    });

    let p = f64::from(current(progress).clamp(0.0, 1.0));
    let style = if animate && !reduce {
        format!(
            "transform:translate3d({}px, {top}px, 0);opacity:{p};translate:0 {}px",
            lane as f64 * (column_width + gap),
            12.0 * (1.0 - p),
        )
    } else {
        format!(
            "transform:translate3d({}px, {top}px, 0)",
            lane as f64 * (column_width + gap),
        )
    };

    rsx! {
        div {
            "data-beui-rect": rect.attr(),
            class: cn!("absolute left-0 top-0 will-change-transform", class),
            style: "width:{column_width}px;{style}",
            onmounted: move |event| rect.measure(event.data()),
            {item.content.clone()}
        }
    }
}

//! `charts/treemap/` — a valued rectangle map with an inspecting cursor.
//!
//! `buildTreemap` and `layoutTreemap` (order-preserving balanced binary tiling)
//! are ported as pure functions; the model — nodes, leaves, tiles, the active
//! item and the tooltip id — is a context shared by plot, tooltip and legend.
//!
//! Differences from the source: the source throws on duplicate ids and
//! invalid values, the port builds silently; `NumberTicker` is not ported (the
//! tooltip prints formatted values directly); the plot measures itself once and
//! on resize through `document::eval`; the tooltip is anchored at the active
//! tile's centre rather than following the cursor; keyboard navigation drives
//! focus from each tile's `keydown`.

use beui::prelude::*;

use crate::common::use_unique_id;
use crate::donut_chart::default_format;

/// `TREEMAP_COLORS`.
pub const TREEMAP_COLORS: [&str; 6] = [
    "#1d4ed8", "#7c3aed", "#0f766e", "#b45309", "#be123c", "#475569",
];

/// `TreemapLeaf`.
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapLeaf {
    pub id: String,
    pub label: String,
    pub color: Option<String>,
    pub text_color: Option<String>,
    pub description: Option<String>,
    pub value: f64,
}

impl TreemapLeaf {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color: None,
            text_color: None,
            description: None,
            value,
        }
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// `TreemapGroup`.
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapGroup {
    pub id: String,
    pub label: String,
    pub color: Option<String>,
    pub text_color: Option<String>,
    pub description: Option<String>,
    pub children: Vec<TreemapNode>,
}

impl TreemapGroup {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        children: Vec<TreemapNode>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color: None,
            text_color: None,
            description: None,
            children,
        }
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// `TreemapNode`.
#[derive(Clone, Debug, PartialEq)]
pub enum TreemapNode {
    Leaf(TreemapLeaf),
    Group(TreemapGroup),
}

/// `TreemapItem`.
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapItem {
    pub id: String,
    pub label: String,
    pub color: String,
    pub text_color: String,
    pub description: Option<String>,
    pub value: f64,
    pub path: Vec<String>,
    pub share: f64,
}

/// `TreemapTile`.
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapTile {
    pub id: String,
    pub label: String,
    pub color: String,
    pub text_color: String,
    pub description: Option<String>,
    pub value: f64,
    pub path: Vec<String>,
    pub share: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A resolved node (group or leaf) used by the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedNode {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub color: String,
    pub children: Vec<ResolvedNode>,
    pub item: Option<TreemapItem>,
}

/// `buildTreemap` result.
#[derive(Clone, Debug, PartialEq)]
pub struct TreemapBuild {
    pub nodes: Vec<ResolvedNode>,
    pub items: Vec<TreemapItem>,
    pub total: f64,
}

fn resolve(
    node: &TreemapNode,
    path: &[String],
    inherited_color: &str,
    inherited_text: &str,
    items: &mut Vec<TreemapItem>,
) -> ResolvedNode {
    match node {
        TreemapNode::Group(group) => {
            let color = group
                .color
                .clone()
                .unwrap_or_else(|| inherited_color.to_string());
            let foreground = group
                .text_color
                .clone()
                .unwrap_or_else(|| inherited_text.to_string());
            let mut next_path = path.to_vec();
            next_path.push(group.label.clone());
            let children: Vec<ResolvedNode> = group
                .children
                .iter()
                .map(|child| resolve(child, &next_path, &color, &foreground, items))
                .collect();
            let value: f64 = children.iter().map(|child| child.value).sum();
            ResolvedNode {
                id: group.id.clone(),
                label: group.label.clone(),
                value,
                color,
                children,
                item: None,
            }
        }
        TreemapNode::Leaf(leaf) => {
            let color = leaf
                .color
                .clone()
                .unwrap_or_else(|| inherited_color.to_string());
            let foreground = leaf
                .text_color
                .clone()
                .unwrap_or_else(|| inherited_text.to_string());
            let mut next_path = path.to_vec();
            next_path.push(leaf.label.clone());
            let item = TreemapItem {
                id: leaf.id.clone(),
                label: leaf.label.clone(),
                color: color.clone(),
                text_color: foreground.clone(),
                description: leaf.description.clone(),
                value: leaf.value,
                path: next_path,
                share: 0.0,
            };
            items.push(item.clone());
            ResolvedNode {
                id: leaf.id.clone(),
                label: leaf.label.clone(),
                value: leaf.value,
                color,
                children: Vec::new(),
                item: Some(item),
            }
        }
    }
}

/// `buildTreemap`.
pub fn build_treemap(data: &[TreemapNode], colors: &[&str], text_color: &str) -> TreemapBuild {
    let fallback = TREEMAP_COLORS[0];
    let mut items = Vec::new();
    let nodes: Vec<ResolvedNode> = data
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let color = colors
                .get(index % colors.len().max(1))
                .copied()
                .unwrap_or(fallback);
            resolve(node, &[], color, text_color, &mut items)
        })
        .collect();
    let total: f64 = nodes.iter().map(|node| node.value).sum();
    for item in &mut items {
        item.share = if total != 0.0 {
            item.value / total
        } else {
            0.0
        };
    }
    TreemapBuild {
        nodes,
        items,
        total,
    }
}

/// `layoutTreemap`.
pub fn layout_treemap(nodes: &[ResolvedNode], width: f64, height: f64) -> Vec<TreemapTile> {
    fn partition(
        siblings: &[&ResolvedNode],
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        tiles: &mut Vec<TreemapTile>,
    ) {
        let positive: Vec<&ResolvedNode> = siblings
            .iter()
            .copied()
            .filter(|node| node.value > 0.0)
            .collect();
        if positive.is_empty() {
            return;
        }
        if positive.len() == 1 {
            let node = positive[0];
            if let Some(item) = &node.item {
                tiles.push(TreemapTile {
                    id: item.id.clone(),
                    label: item.label.clone(),
                    color: item.color.clone(),
                    text_color: item.text_color.clone(),
                    description: item.description.clone(),
                    value: item.value,
                    path: item.path.clone(),
                    share: item.share,
                    x,
                    y,
                    width: w,
                    height: h,
                });
            } else {
                let child_refs: Vec<&ResolvedNode> = node.children.iter().collect();
                partition(&child_refs, x, y, w, h, tiles);
            }
            return;
        }
        let total: f64 = positive.iter().map(|node| node.value).sum();
        let mut cut = 1usize;
        let mut subtotal = positive[0].value;
        for (offset, node) in positive
            .iter()
            .enumerate()
            .take(positive.len().saturating_sub(1))
            .skip(1)
        {
            let next = subtotal + node.value;
            if (next - total / 2.0).abs() > (subtotal - total / 2.0).abs() {
                break;
            }
            cut = offset + 1;
            subtotal = next;
        }
        let fraction = if total != 0.0 { subtotal / total } else { 0.0 };
        if w >= h {
            let split = w * fraction;
            partition(&positive[..cut], x, y, split, h, tiles);
            partition(&positive[cut..], x + split, y, w - split, h, tiles);
        } else {
            let split = h * fraction;
            partition(&positive[..cut], x, y, w, split, tiles);
            partition(&positive[cut..], x, y + split, w, h - split, tiles);
        }
    }
    let mut tiles = Vec::new();
    let refs: Vec<&ResolvedNode> = nodes.iter().collect();
    partition(&refs, 0.0, 0.0, width, height, &mut tiles);
    tiles.retain(|tile| tile.width > 0.0 && tile.height > 0.0);
    tiles
}

#[derive(Clone, Copy)]
struct TreemapContext {
    build: Signal<TreemapBuild>,
    tiles: Signal<Vec<TreemapTile>>,
    size: Signal<(f64, f64)>,
    format_value: Option<Callback<f64, String>>,
    active: Signal<Option<String>>,
    on_active_change: Option<EventHandler<Option<String>>>,
    controlled: bool,
    focused: Signal<Option<String>>,
    tooltip_open: Signal<bool>,
    tooltip_id: Signal<String>,
}

impl TreemapContext {
    fn format(&self, value: f64) -> String {
        match self.format_value {
            Some(callback) => callback.call(value),
            None => default_format(value),
        }
    }

    fn set_active(&self, id: Option<String>) {
        if !self.controlled {
            let mut active = self.active;
            active.set(id.clone());
        }
        if let Some(handler) = self.on_active_change {
            handler.call(id);
        }
    }
}

/// `Treemap`.
#[component]
pub fn Treemap(
    data: Vec<TreemapNode>,
    #[props(default = "Treemap".to_string())] label: String,
    #[props(default)] format_value: Option<Callback<f64, String>>,
    #[props(default = vec![
        TREEMAP_COLORS[0].to_string(),
        TREEMAP_COLORS[1].to_string(),
        TREEMAP_COLORS[2].to_string(),
        TREEMAP_COLORS[3].to_string(),
        TREEMAP_COLORS[4].to_string(),
        TREEMAP_COLORS[5].to_string(),
    ])]
    colors: Vec<String>,
    #[props(default = "#ffffff".to_string())] text_color: String,
    #[props(default)] active_id: Option<String>,
    #[props(default)] default_active_id: Option<String>,
    #[props(default)] on_active_change: Option<EventHandler<Option<String>>>,
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let color_refs: Vec<&str> = colors.iter().map(String::as_str).collect();
    let build = build_treemap(&data, &color_refs, &text_color);
    let build_signal = use_signal(|| build);
    let tiles_signal = use_signal(Vec::<TreemapTile>::new);
    let size_signal = use_signal(|| (1000.0f64, 625.0f64));
    let mut internal_active = use_signal(|| default_active_id.clone());
    let focused_signal = use_signal(|| None::<String>);
    let tooltip_open = use_signal(|| false);
    let tooltip_id_value = use_unique_id("treemap-tip");
    let tooltip_id_signal = use_signal(|| tooltip_id_value.clone());
    let controlled = active_id.is_some();

    use_effect(use_reactive((&active_id,), move |(active_id,)| {
        if let Some(active_id) = active_id {
            internal_active.set(Some(active_id));
        }
    }));

    // Recompute the tiling whenever the data or measured size changes.
    use_effect(move || {
        let model = build_signal();
        let (width, height) = size_signal();
        let next = layout_treemap(&model.nodes, width, height);
        if tiles_signal() != next {
            tiles_signal.clone().set(next);
        }
    });

    let context = TreemapContext {
        build: build_signal,
        tiles: tiles_signal,
        size: size_signal,
        format_value,
        active: internal_active,
        on_active_change,
        controlled,
        focused: focused_signal,
        tooltip_open,
        tooltip_id: tooltip_id_signal,
    };
    use_context_provider(|| context);

    let items = (context.build)();

    rsx! {
        section {
            "aria-label": "{label}",
            "data-slot": "treemap",
            class: cn!("min-w-0 w-full space-y-4", class),
            ..attributes,
            if let Some(children) = children {
                {children}
            } else {
                div { class: "relative",
                    TreemapPlot {}
                    TreemapTooltip {}
                }
                TreemapLegend {}
            }
            div { class: "sr-only",
                table {
                    caption { "{label} data" }
                    thead {
                        tr {
                            th { scope: "col", "Item" }
                            th { scope: "col", "Value" }
                            th { scope: "col", "Share of total" }
                            th { scope: "col", "Details" }
                        }
                    }
                    tbody {
                        for item in items.items.iter() {
                            tr { key: "{item.id}",
                                th { scope: "row", "{item.path.join(\" / \")}" }
                                td { "{context.format(item.value)}" }
                                td { {format!("{:.1}%", item.share * 100.0)} }
                                td { "{item.description.clone().unwrap_or_default()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `TreemapPlot`.
#[component]
pub fn TreemapPlot(
    #[props(default)] class: String,
    #[props(default)] tile_class_name: String,
    #[props(default)] render_tile: Option<Callback<TreemapTile, Element>>,
) -> Element {
    let context = use_context::<TreemapContext>();
    let plot_id = use_unique_id("treemap-plot");
    let plot_id_for_effect = plot_id.clone();

    // Measure the plot and keep it current through a ResizeObserver.
    use_effect(move || {
        let selector = format!("[data-beui-treemap-plot=\"{plot_id_for_effect}\"]");
        let script = format!(
            "(function(){{var el=document.querySelector({sel});if(!el)return;\
             function send(){{var r=el.getBoundingClientRect();dioxus.send({{left:0,top:0,width:r.width,height:r.height}});}}\
             send();var ro=new ResizeObserver(send);ro.observe(el);}})();",
            sel = beui_core::platform::js_string(&selector),
        );
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            while let Ok(rect) = eval.recv::<beui_core::platform::Rect>().await {
                let next = (rect.width, rect.height);
                if (context.size)() != next {
                    let mut size = context.size;
                    size.set(next);
                }
            }
        });
    });

    let tiles = (context.tiles)();
    let items_empty = (context.build)().items.is_empty();

    rsx! {
        div {
            "data-beui-treemap-plot": "{plot_id}",
            role: "group",
            "aria-label": "Treemap tiles",
            tabindex: "-1",
            "data-slot": "treemap-plot",
            class: cn!(
                "relative h-80 w-full min-w-0 rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-ring",
                class,
            ),
            onpointerleave: move |_| {
                let mut open = context.tooltip_open;
                open.set(false);
            },
            for tile in tiles.iter() {
                TreemapCell {
                    key: "{tile.id}",
                    tile: tile.clone(),
                    class: tile_class_name.clone(),
                    render_tile,
                }
            }
            if tiles.is_empty() {
                p { class: "flex h-full items-center justify-center text-sm text-muted-foreground",
                    if items_empty { "No treemap data" } else { "No positive values" }
                }
            }
        }
    }
}

/// `TreemapCell`.
#[component]
pub fn TreemapCell(
    tile: TreemapTile,
    #[props(default)] class: String,
    #[props(default)] render_tile: Option<Callback<TreemapTile, Element>>,
) -> Element {
    let context = use_context::<TreemapContext>();
    let can_hover = beui_core::use_hover_capable();
    let (width, height) = (context.size)();
    let active = (context.active)();
    let tooltip_open = (context.tooltip_open)();
    let focused = (context.focused)();
    let tiles = (context.tiles)();
    let first_id = tiles.first().map(|tile| tile.id.clone());
    let is_focus_target = focused.clone().or(first_id).as_deref() == Some(tile.id.as_str());
    let selected = tooltip_open && active.as_deref() == Some(tile.id.as_str());
    let show_label = tile.width >= 52.0 && tile.height >= 32.0;
    let show_value = tile.width >= 70.0 && tile.height >= 56.0;
    let width_pct = if width != 0.0 {
        tile.width / width
    } else {
        0.0
    };
    let height_pct = if height != 0.0 {
        tile.height / height
    } else {
        0.0
    };
    let aria_label = {
        let mut parts = vec![
            tile.path.join(" / "),
            context.format(tile.value),
            format!("{:.1}% of total", tile.share * 100.0),
        ];
        if let Some(description) = &tile.description {
            parts.push(description.clone());
        }
        parts.join(". ")
    };
    let style = format!(
        "left:{}%;top:{}%;width:{}%;height:{}%;background-color:{};color:{};border-radius:8px;",
        tile.x / if width != 0.0 { width } else { 1.0 } * 100.0,
        tile.y / if height != 0.0 { height } else { 1.0 } * 100.0,
        width_pct * 100.0,
        height_pct * 100.0,
        tile.color,
        tile.text_color,
    );
    let active_for_focus = context.active;
    let focused_signal = context.focused;
    let tooltip_signal = context.tooltip_open;
    let tooltip_id = (context.tooltip_id)();
    let click_context = context;
    let nav_tiles = tiles.clone();

    rsx! {
        button {
            r#type: "button",
            "data-treemap-tile": "{tile.id}",
            tabindex: if is_focus_target { "0" } else { "-1" },
            "aria-label": "{aria_label}",
            "aria-describedby": if selected { Some(tooltip_id) } else { None },
            class: cn!(
                "absolute overflow-hidden border-2 border-background text-left outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-white",
                class,
            ),
            style: "{style}",
            onpointerenter: {
                let id = tile.id.clone();
                move |_| {
                    if can_hover() {
                        click_context.set_active(Some(id.clone()));
                        {
                        let mut open = tooltip_signal;
                        open.set(true);
                    }
                    }
                }
            },
            onpointerdown: {
                let id = tile.id.clone();
                move |_| {
                    click_context.set_active(Some(id.clone()));
                    {
                        let mut open = tooltip_signal;
                        open.set(true);
                    }
                }
            },
            onclick: {
                let id = tile.id.clone();
                move |_| {
                    click_context.set_active(Some(id.clone()));
                    {
                        let mut open = tooltip_signal;
                        open.set(true);
                    }
                }
            },
            onfocus: {
                let id = tile.id.clone();
                move |_| {
                    {
                    let mut focused = focused_signal;
                    focused.set(Some(id.clone()));
                }
                    {
                    let mut active = active_for_focus;
                    active.set(Some(id.clone()));
                }
                    {
                        let mut open = tooltip_signal;
                        open.set(true);
                    }
                }
            },
            onkeydown: {
                let id = tile.id.clone();
                let tiles = nav_tiles.clone();
                move |event: KeyboardEvent| {
                    if event.key() == Key::Escape {
                        {
                        let mut open = tooltip_signal;
                        open.set(false);
                    }
                        return;
                    }
                    let current = match tiles.iter().find(|tile| tile.id == id) {
                        Some(tile) => tile.clone(),
                        None => return,
                    };
                    let next = if event.key() == Key::Home {
                        tiles.first().cloned()
                    } else if event.key() == Key::End {
                        tiles.last().cloned()
                    } else if matches!(
                        event.key(),
                        Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown
                    ) {
                        event.prevent_default();
                        let horizontal = matches!(event.key(), Key::ArrowLeft | Key::ArrowRight);
                        let direction = if matches!(event.key(), Key::ArrowLeft | Key::ArrowUp) { -1.0 } else { 1.0 };
                        let cx = current.x + current.width / 2.0;
                        let cy = current.y + current.height / 2.0;
                        let mut best = f64::INFINITY;
                        let mut chosen = None;
                        for tile in tiles.iter() {
                            let dx = tile.x + tile.width / 2.0 - cx;
                            let dy = tile.y + tile.height / 2.0 - cy;
                            let forward = (if horizontal { dx } else { dy }) * direction;
                            let cross = (if horizontal { dy } else { dx }).abs();
                            let score = forward + cross * 2.0;
                            if forward > 0.01 && score < best {
                                best = score;
                                chosen = Some(tile.clone());
                            }
                        }
                        chosen
                    } else {
                        None
                    };
                    let Some(next) = next else { return };
                    {
                        let mut open = tooltip_signal;
                        open.set(true);
                    }
                    if next.id == current.id {
                        {
                        let mut active = active_for_focus;
                        active.set(Some(current.id.clone()));
                    }
                    } else {
                        let target = next.id.clone();
                        {
                        let mut focused = focused_signal;
                        focused.set(Some(target.clone()));
                    }
                        {
                        let mut active = active_for_focus;
                        active.set(Some(target.clone()));
                    }
                        beui_core::platform::fire(&format!(
                            "var el=document.querySelector({});if(el)el.focus();",
                            beui_core::platform::js_string(&format!("[data-treemap-tile=\"{target}\"]")),
                        ));
                    }
                }
            },
            if show_label {
                span {
                    "aria-hidden": "true",
                    class: "absolute inset-x-2 top-2 min-w-0 sm:inset-x-3 sm:top-3",
                    if let Some(render_tile) = render_tile {
                        {render_tile.call(tile.clone())}
                    } else {
                        span { class: "block truncate text-xs font-semibold sm:text-sm", "{tile.label}" }
                        if show_value {
                            span { class: "mt-1 block truncate font-mono text-[10px] tabular-nums opacity-80 sm:text-xs",
                                "{context.format(tile.value)}"
                            }
                        }
                    }
                }
            }
            {
                let overlay_opacity = if selected { 1.0 } else { 0.0 };
                rsx! {
                    span {
                        "aria-hidden": "true",
                        class: "pointer-events-none absolute inset-0 bg-white/10",
                        style: "opacity:{overlay_opacity};transition:opacity 100ms cubic-bezier(0.16,1,0.3,1);",
                    }
                }
            }
        }
    }
}

/// `TreemapTooltip`.
#[component]
pub fn TreemapTooltip(
    #[props(default)] class: String,
    #[props(default)] children: Option<Element>,
) -> Element {
    let context = use_context::<TreemapContext>();
    let tiles = (context.tiles)();
    let active = (context.active)();
    let tooltip_open = (context.tooltip_open)();
    let (width, height) = (context.size)();
    if !tooltip_open {
        return rsx! {};
    }
    let tooltip_id_value = (context.tooltip_id)();
    let tile = active
        .as_ref()
        .and_then(|id| tiles.iter().find(|tile| &tile.id == id))
        .map(ToOwned::to_owned);
    let Some(tile) = tile else {
        return rsx! {};
    };
    let x = if width != 0.0 {
        (tile.x + tile.width / 2.0) / width * 100.0
    } else {
        50.0
    };
    let y = if height != 0.0 {
        (tile.y + tile.height / 2.0) / height * 100.0
    } else {
        50.0
    };
    let parent_path = if tile.path.len() > 1 {
        Some(tile.path[..tile.path.len() - 1].join(" / "))
    } else {
        None
    };

    rsx! {
        div {
            id: "{tooltip_id_value}",
            role: "tooltip",
            class: cn!("pointer-events-none absolute z-10 max-w-64 -translate-x-1/2 -translate-y-1/2 rounded-xl border border-border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-md", class),
            style: "left:{x}%;top:{y}%",
            if let Some(children) = children {
                {children}
            } else {
                span { class: "flex flex-col gap-1.5 text-xs",
                    if let Some(parent_path) = parent_path {
                        span { class: "text-muted-foreground", "{parent_path}" }
                    }
                    span { class: "font-medium", "{tile.label}" }
                    span { class: "inline-flex items-center gap-2 font-mono tabular-nums",
                        "{context.format(tile.value)}"
                        span { class: "text-muted-foreground", "·" }
                        span { class: "text-muted-foreground", {format!("{:.1}%", tile.share * 100.0)} }
                    }
                    if let Some(description) = tile.description.clone() {
                        span { class: "text-muted-foreground", "{description}" }
                    }
                }
            }
        }
    }
}

/// `TreemapLegend`.
#[component]
pub fn TreemapLegend(
    #[props(default)] show_values: bool,
    #[props(default)] class: String,
) -> Element {
    let context = use_context::<TreemapContext>();
    let nodes = (context.build)().nodes;
    if nodes.is_empty() {
        return rsx! {};
    }
    rsx! {
        ul { "aria-label": "Treemap legend", class: cn!("flex flex-wrap gap-x-5 gap-y-2 text-xs", class),
            for node in nodes.iter() {
                li { key: "{node.id}", class: "inline-flex items-center gap-1.5",
                    span {
                        "aria-hidden": "true",
                        class: "size-1.5 shrink-0 rounded-full",
                        style: "background-color:{node.color};",
                    }
                    span { class: "text-muted-foreground", "{node.label}" }
                    if show_values {
                        span { class: "font-mono tabular-nums", "{context.format(node.value)}" }
                    }
                }
            }
        }
    }
}

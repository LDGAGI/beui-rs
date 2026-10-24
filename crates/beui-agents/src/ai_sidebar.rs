//! `agents/ai-sidebar.tsx` — the resource tree sidebar.
//!
//! The source's tree operations (`flattenResources`, `findResource`,
//! `containsResource`, `removeResource`, `insertResource`, `moveResource`,
//! `renameResource`) are ported as pure functions over [`SidebarResource`]; the
//! row rendering keeps the same `role="tree"`/`treeitem` ARIA, roving tabindex,
//! keyboard map and `data-*` hooks, and the row menu carries the same four moves
//! as `Alt+Shift+Arrow`.
//!
//! Differences from the source:
//! - the row menu is a lane-local absolute popover (motion `MorphPopover`'s
//!   morph/portal is the motion lane's);
//! - `onMove` / `onRename` are synchronous events, so a rejected async move or
//!   rename cannot roll back the optimistic update (the source awaits and
//!   restores);
//! - the drag-over `inside`/`before`/`after` threshold uses the row's 36px
//!   minimum height rather than a live `getBoundingClientRect`.

use beui::prelude::*;

use crate::common::{use_unique_id, Icon};
use crate::icons::IconName;

/// Row height used for the drag-over position ratio (`min-h-9` = 36px).
const ROW_HEIGHT: f64 = 36.0;

const MARQUEE_KEYFRAMES: &str =
    "@keyframes beui-sidebar-marquee{from{transform:translateX(0)}to{transform:translateX(var(--beui-marquee-distance,0px))}}";

/// `SidebarResourceKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SidebarResourceKind {
    Folder,
    Project,
    #[default]
    File,
    Bookmark,
}

/// `SidebarResource`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SidebarResource {
    pub id: String,
    pub label: String,
    pub kind: SidebarResourceKind,
    pub children: Vec<SidebarResource>,
    pub disabled: bool,
}

impl SidebarResource {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: SidebarResourceKind) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            children: Vec::new(),
            disabled: false,
        }
    }

    pub fn folder(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(id, label, SidebarResourceKind::Folder)
    }

    pub fn file(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(id, label, SidebarResourceKind::File)
    }

    pub fn children(mut self, children: Vec<SidebarResource>) -> Self {
        self.children = children;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// `SidebarResourceDropPosition`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarResourceDropPosition {
    Before,
    Inside,
    After,
}

/// `SidebarResourceMove`.
#[derive(Clone, Debug, PartialEq)]
pub struct SidebarResourceMove {
    pub item_id: String,
    pub target_id: Option<String>,
    pub position: SidebarResourceDropPosition,
}

/// A flattened, visible row.
#[derive(Clone, Debug, PartialEq)]
pub struct FlatResource {
    pub item: SidebarResource,
    pub depth: usize,
    pub parent_id: Option<String>,
}

/// `canContain`.
pub fn can_contain(item: &SidebarResource) -> bool {
    matches!(
        item.kind,
        SidebarResourceKind::Folder | SidebarResourceKind::Project
    )
}

/// `flattenResources`.
pub fn flatten_resources(items: &[SidebarResource], expanded: &[String]) -> Vec<FlatResource> {
    fn walk(
        items: &[SidebarResource],
        expanded: &[String],
        depth: usize,
        parent_id: Option<&str>,
        out: &mut Vec<FlatResource>,
    ) {
        for item in items {
            out.push(FlatResource {
                item: item.clone(),
                depth,
                parent_id: parent_id.map(str::to_string),
            });
            if !item.children.is_empty() && expanded.iter().any(|id| id == &item.id) {
                walk(&item.children, expanded, depth + 1, Some(&item.id), out);
            }
        }
    }
    let mut out = Vec::new();
    walk(items, expanded, 0, None, &mut out);
    out
}

/// `findResource`.
pub fn find_resource<'a>(items: &'a [SidebarResource], id: &str) -> Option<&'a SidebarResource> {
    for item in items {
        if item.id == id {
            return Some(item);
        }
        if let Some(found) = find_resource(&item.children, id) {
            return Some(found);
        }
    }
    None
}

/// `containsResource`.
pub fn contains_resource(item: &SidebarResource, id: &str) -> bool {
    item.id == id
        || item
            .children
            .iter()
            .any(|child| contains_resource(child, id))
}

/// `removeResource`.
pub fn remove_resource(
    items: &[SidebarResource],
    id: &str,
) -> (Vec<SidebarResource>, Option<SidebarResource>) {
    let mut removed = None;
    let mut next = Vec::new();
    for item in items {
        if item.id == id {
            removed = Some(item.clone());
            continue;
        }
        if !item.children.is_empty() {
            let (children, child_removed) = remove_resource(&item.children, id);
            if child_removed.is_some() {
                removed = child_removed;
                let mut item = item.clone();
                item.children = children;
                next.push(item);
                continue;
            }
        }
        next.push(item.clone());
    }
    (next, removed)
}

/// `insertResource`.
pub fn insert_resource(
    items: &[SidebarResource],
    resource: &SidebarResource,
    target_id: Option<&str>,
    position: SidebarResourceDropPosition,
) -> Vec<SidebarResource> {
    let Some(target_id) = target_id else {
        let mut next = items.to_vec();
        next.push(resource.clone());
        return next;
    };
    let mut next = Vec::new();
    for item in items {
        if item.id == target_id {
            match position {
                SidebarResourceDropPosition::Before => {
                    next.push(resource.clone());
                    next.push(item.clone());
                }
                SidebarResourceDropPosition::After => {
                    next.push(item.clone());
                    next.push(resource.clone());
                }
                SidebarResourceDropPosition::Inside => {
                    let mut item = item.clone();
                    item.children.push(resource.clone());
                    next.push(item);
                }
            }
            continue;
        }
        if !item.children.is_empty() {
            let mut item = item.clone();
            item.children = insert_resource(&item.children, resource, Some(target_id), position);
            next.push(item);
        } else {
            next.push(item.clone());
        }
    }
    next
}

/// `moveResource`.
pub fn move_resource(
    items: &[SidebarResource],
    mv: &SidebarResourceMove,
) -> Option<Vec<SidebarResource>> {
    let source = find_resource(items, &mv.item_id)?;
    if source.disabled {
        return None;
    }
    if let Some(target_id) = &mv.target_id {
        if contains_resource(source, target_id) {
            return None;
        }
    }
    let target = mv
        .target_id
        .as_deref()
        .and_then(|id| find_resource(items, id));
    if mv.position == SidebarResourceDropPosition::Inside
        && target.is_none_or(|target| target.disabled || !can_contain(target))
    {
        return None;
    }
    let (remaining, removed) = remove_resource(items, &mv.item_id);
    let removed = removed?;
    Some(insert_resource(
        &remaining,
        &removed,
        mv.target_id.as_deref(),
        mv.position,
    ))
}

/// `renameResource`.
pub fn rename_resource(items: &[SidebarResource], id: &str, label: &str) -> Vec<SidebarResource> {
    items
        .iter()
        .map(|item| {
            let mut next = item.clone();
            if next.id == id {
                next.label = label.to_string();
            }
            if !next.children.is_empty() {
                next.children = rename_resource(&next.children, id, label);
            }
            next
        })
        .collect()
}

fn default_icon(item: &SidebarResource, expanded: bool) -> IconName {
    match item.kind {
        SidebarResourceKind::Folder | SidebarResourceKind::Project => {
            if expanded {
                IconName::FolderOpen
            } else {
                IconName::Folder
            }
        }
        SidebarResourceKind::Bookmark => IconName::Bookmark,
        SidebarResourceKind::File => IconName::FileText,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DropTarget {
    id: Option<String>,
    position: SidebarResourceDropPosition,
}

/// `MarqueeLabel` — scrolls the label on hover when it overflows.
#[component]
fn MarqueeLabel(active: bool, label: String) -> Element {
    let reduce = use_reduced_motion()();
    let viewport = use_unique_id("marquee-view");
    let viewport_for_effect = viewport.clone();
    let label_id = use_unique_id("marquee-label");
    let label_for_effect = label_id.clone();
    let mut distance = use_signal(|| 0.0f64);

    use_effect(use_reactive((&label,), move |_| {
        let script = format!(
            "(function(){{var v=document.querySelector({view});var l=document.querySelector({label});if(!v||!l)return;dioxus.send(l.scrollWidth>v.clientWidth?l.scrollWidth+24:0);}})();",
            view = beui_core::platform::js_string(&format!("[data-beui-marquee-view=\"{viewport_for_effect}\"]")),
            label = beui_core::platform::js_string(&format!("[data-beui-marquee-label=\"{label_for_effect}\"]")),
        );
        let Some(mut eval) = beui_core::platform::eval(&script) else {
            return;
        };
        spawn(async move {
            if let Ok(value) = eval.recv::<f64>().await {
                distance.set(value);
            }
        });
    }));

    let running = active && distance() > 0.0 && !reduce;
    let duration = (distance() / 34.0).max(2.4);
    let marquee_style = if running {
        format!(
            "--beui-marquee-distance:-{distance}px;animation:beui-sidebar-marquee {duration}s linear infinite;animation-delay:2s;"
        )
    } else {
        String::new()
    };

    rsx! {
        span {
            "data-beui-marquee-view": "{viewport}",
            class: "block min-w-0 flex-1 overflow-hidden",
            span {
                class: "flex w-max items-center gap-6 whitespace-nowrap",
                style: "{marquee_style}",
                span { "data-beui-marquee-label": "{label_id}", "{label}" }
                if running {
                    span { "aria-hidden": "true", "{label}" }
                }
            }
        }
    }
}

fn key_is(event: &KeyboardData, key: Key) -> bool {
    event.key() == key
}

fn space_pressed(event: &KeyboardData) -> bool {
    matches!(event.key(), Key::Character(ref c) if c == " ")
}

#[component]
fn ResourceMenuAction(icon: IconName, on_select: EventHandler<()>, children: Element) -> Element {
    rsx! {
        button {
            r#type: "button",
            onclick: move |_| on_select.call(()),
            class: "flex h-8 w-full items-center gap-2 rounded-lg px-2.5 text-left text-xs text-foreground outline-none transition-colors hover:bg-muted focus-visible:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
            Icon { name: icon, class: "size-3.5 shrink-0", "aria-hidden": "true" }
            span { class: "min-w-0 truncate", {children} }
        }
    }
}

/// `AISidebar`.
#[component]
pub fn AISidebar(
    #[props(default)] items: Option<Vec<SidebarResource>>,
    #[props(default)] default_items: Vec<SidebarResource>,
    #[props(default)] on_items_change: Option<EventHandler<Vec<SidebarResource>>>,
    #[props(default)] on_move: Option<EventHandler<SidebarResourceMove>>,
    #[props(default)] on_rename: Option<EventHandler<(SidebarResource, String)>>,
    #[props(default)] active_id: Option<String>,
    #[props(default)] default_active_id: Option<String>,
    #[props(default)] on_active_change: Option<EventHandler<String>>,
    #[props(default)] default_expanded_ids: Vec<String>,
    #[props(default = "Resources".to_string())] aria_label: String,
    #[props(default)] class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let can_touch = beui_core::use_touch_capable();
    let mut internal_items = use_signal(|| items.clone().unwrap_or(default_items));
    let mut internal_active = use_signal(|| default_active_id.clone());
    // Kept as a flat list; membership is what matters.
    let mut expanded_ids = use_signal(|| default_expanded_ids.clone());
    let mut focused_id = use_signal(|| active_id.clone().or(default_active_id));
    let mut dragging_id = use_signal(|| None::<String>);
    let mut drop_target = use_signal(|| None::<DropTarget>);
    let mut menu_open_id = use_signal(|| None::<String>);
    let mut renaming_id = use_signal(|| None::<String>);
    let mut hovered_id = use_signal(|| None::<String>);
    let mut rename_draft = use_signal(String::new);
    let mut announcement = use_signal(String::new);
    let _ = reduce;

    let items_controlled = items.is_some();
    use_effect(use_reactive((&items,), move |(items,)| {
        if let Some(items) = items {
            internal_items.set(items);
            let _ = items_controlled;
        }
    }));

    let rendered_items = internal_items();
    let selected_id = active_id.clone().or(internal_active());
    let flat = flatten_resources(&rendered_items, &expanded_ids());
    let focused_row = focused_id()
        .filter(|id| flat.iter().any(|row| &row.item.id == id))
        .or_else(|| flat.first().map(|row| row.item.id.clone()));
    if focused_id() != focused_row {
        focused_id.set(focused_row.clone());
    }

    let set_items = use_callback(move |next: Vec<SidebarResource>| {
        if !items_controlled {
            internal_items.set(next.clone());
        }
        if let Some(handler) = on_items_change {
            handler.call(next);
        }
    });

    let focus_row = use_callback(move |id: String| {
        focused_id.set(Some(id.clone()));
        beui_core::platform::fire(&format!(
            "var el=document.querySelector({});if(el)el.focus();",
            beui_core::platform::js_string(&format!("[data-beui-row=\"{id}\"]")),
        ));
    });

    let select = use_callback(move |id: String| {
        if active_id.is_none() {
            internal_active.set(Some(id.clone()));
        }
        if let Some(handler) = on_active_change {
            handler.call(id);
        }
    });

    let toggle = use_callback(move |id: String| {
        let mut current = expanded_ids();
        if let Some(index) = current.iter().position(|value| value == &id) {
            current.remove(index);
        } else {
            current.push(id);
        }
        expanded_ids.set(current);
    });

    let perform_move = use_callback(move |mv: SidebarResourceMove| {
        let before = internal_items();
        let next = match move_resource(&before, &mv) {
            Some(next) if next != before => next,
            _ => return,
        };
        let moved = find_resource(&before, &mv.item_id).map(|item| item.label.clone());
        let target = mv
            .target_id
            .as_deref()
            .and_then(|id| find_resource(&before, id))
            .map(|item| item.label.clone());
        set_items.call(next);
        drop_target.set(None);
        dragging_id.set(None);
        announcement.set(match (&moved, &target) {
            (Some(moved), Some(target)) => format!(
                "Moved {moved} {position} {target}.",
                position = match mv.position {
                    SidebarResourceDropPosition::Before => "before",
                    SidebarResourceDropPosition::Inside => "inside",
                    SidebarResourceDropPosition::After => "after",
                }
            ),
            (Some(moved), None) => format!("Moved {moved} to the top level."),
            _ => String::new(),
        });
        if let Some(handler) = on_move {
            handler.call(mv);
        }
    });

    // Focus the first menu item after it opens.
    use_effect(move || {
        if let Some(id) = menu_open_id() {
            beui_core::platform::fire(&format!(
                "(function(){{var m=document.querySelector({});if(!m)return;var b=m.querySelector('button,a[href]');if(b)b.focus();}})();",
                beui_core::platform::js_string(&format!("[data-sidebar-resource-menu=\"{id}\"]")),
            ));
        }
    });

    rsx! {
        div {
            role: "tree",
            "aria-label": "{aria_label}",
            "aria-multiselectable": "false",
            class: cn!(
                "relative flex min-w-0 flex-col gap-0.5 [overflow-anchor:none] group-data-[state=collapsed]/sidebar:hidden",
                if dragging_id().is_some() { "select-none pb-9" } else { "" },
                class,
            ),
            ondragover: move |event| {
                if dragging_id().is_none() {
                    return;
                }
                event.prevent_default();
                drop_target.set(Some(DropTarget { id: None, position: SidebarResourceDropPosition::After }));
            },
            ondrop: move |event| {
                event.prevent_default();
                if let (Some(dragging), Some(target)) = (dragging_id(), drop_target()) {
                    perform_move.call(SidebarResourceMove {
                        item_id: dragging,
                        target_id: target.id,
                        position: target.position,
                    });
                }
            },
            for row in flat.iter() {
                {
                    let item = row.item.clone();
                    let id = item.id.clone();
                    let accepts_children = can_contain(&item);
                    let expanded = expanded_ids().iter().any(|value| value == &id);
                    let active = selected_id.as_deref() == Some(id.as_str());
                    let is_focused = focused_row.as_deref() == Some(id.as_str());
                    let is_dragging = dragging_id().as_deref() == Some(id.as_str());
                    let menu_open = menu_open_id().as_deref() == Some(id.as_str());
                    let renaming = renaming_id().as_deref() == Some(id.as_str());
                    let hovered = hovered_id().as_deref() == Some(id.as_str());
                    let drop_position = drop_target().filter(|target| target.id.as_deref() == Some(id.as_str())).map(|target| target.position);
                    let index = flat.iter().position(|candidate| candidate.item.id == id).unwrap_or(0);
                    let previous = index.checked_sub(1).and_then(|i| flat.get(i)).cloned();
                    let next = flat.get(index + 1).cloned();
                    let parent_id = row.parent_id.clone();
                    let can_move = !item.disabled;
                    let up_target = can_move.then(|| previous.as_ref().map(|r| r.item.id.clone())).flatten();
                    let down_target = can_move.then(|| next.as_ref().map(|r| r.item.id.clone())).flatten();
                    let into_target = can_move
                        .then(|| previous.as_ref().filter(|r| can_contain(&r.item) && Some(&r.item.id) != parent_id.as_ref()).map(|r| (r.item.id.clone(), r.item.label.clone())))
                        .flatten();
                    let out_target = can_move.then(|| parent_id.clone()).flatten();
                    let padding = 12 + row.depth * 16;
                    let row_class = cn!(
                        "group/resource relative flex min-h-9 min-w-0 cursor-pointer items-center gap-2.5 rounded-xl pr-3 text-sm outline-none",
                        "text-muted-foreground transition-colors hover:bg-muted hover:text-foreground",
                        "focus-visible:bg-muted/70 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
                        "data-[menu-open=true]:bg-muted data-[menu-open=true]:text-foreground",
                        "data-[dragging=true]:opacity-40",
                        "data-[drop=inside]:bg-muted data-[drop=inside]:ring-1 data-[drop=inside]:ring-primary/45",
                        "data-[drop=before]:before:absolute data-[drop=before]:before:-top-0.5 data-[drop=before]:before:right-2 data-[drop=before]:before:left-2 data-[drop=before]:before:h-0.5 data-[drop=before]:before:rounded-full data-[drop=before]:before:bg-primary",
                        "data-[drop=after]:after:absolute data-[drop=after]:after:-bottom-0.5 data-[drop=after]:after:right-2 data-[drop=after]:after:left-2 data-[drop=after]:after:h-0.5 data-[drop=after]:after:rounded-full data-[drop=after]:after:bg-primary",
                        if !accepts_children && active { "bg-muted text-foreground" } else { "" },
                        if item.disabled { "cursor-not-allowed opacity-45" } else { "" },
                    );
                    let icon = default_icon(&item, expanded);
                    let row_for_click = item.clone();
                    let row_for_key = item.clone();
                    let row_for_menu = item.clone();
                    let row_for_drag = item.clone();
                    let flat_for_key = flat.clone();
                    rsx! {
                        div {
                            key: "{id}",
                            "data-beui-row": "{id}",
                            role: "treeitem",
                            "aria-level": "{row.depth + 1}",
                            "aria-selected": if accepts_children { None } else { Some(if active { "true" } else { "false" }) },
                            "aria-expanded": if accepts_children { Some(if expanded { "true" } else { "false" }) } else { None },
                            "aria-disabled": if item.disabled { Some("true") } else { None },
                            tabindex: if is_focused { "0" } else { "-1" },
                            draggable: if !item.disabled && !renaming { "true" } else { "false" },
                            "data-menu-open": if menu_open { Some("true") } else { None },
                            "data-drop": drop_position.map(|position| match position {
                                SidebarResourceDropPosition::Before => "before",
                                SidebarResourceDropPosition::Inside => "inside",
                                SidebarResourceDropPosition::After => "after",
                            }),
                            "data-dragging": if is_dragging { Some("true") } else { None },
                            class: "{row_class}",
                            style: "padding-left:{padding}px",
                            onfocus: { let focused = id.clone(); move |_| focused_id.set(Some(focused.clone())) },
                            onkeydown: {
                                move |event: KeyboardEvent| {
                                    handle_key_down(
                                        event,
                                        &row_for_key,
                                        &flat_for_key,
                                        expanded_ids,
                                        toggle,
                                        select,
                                        focus_row,
                                        perform_move,
                                        renaming_id,
                                        menu_open_id,
                                        rename_draft,
                                    );
                                }
                            },
                            onclick: {
                                let id = id.clone();
                                let accepts = accepts_children;
                                move |_event| {
                                    if renaming_id().as_deref() == Some(id.as_str()) || row_for_click.disabled {
                                        return;
                                    }
                                    if accepts {
                                        toggle.call(id.clone());
                                    } else {
                                        select.call(id.clone());
                                    }
                                }
                            },
                            ondoubleclick: {
                                let has_children = accepts_children;
                                let id = id.clone();
                                move |event| {
                                    if has_children || row_for_menu.disabled {
                                        return;
                                    }
                                    event.prevent_default();
                                    rename_draft.set(row_for_menu.label.clone());
                                    renaming_id.set(Some(id.clone()));
                                }
                            },
                            onmouseenter: {
                                let id = id.clone();
                                move |_| hovered_id.set(Some(id.clone()))
                            },
                            onmouseleave: {
                                let id = id.clone();
                                move |_| {
                                    if hovered_id().as_deref() == Some(id.as_str()) {
                                        hovered_id.set(None);
                                    }
                                }
                            },
                            ondragstart: {
                                let id = id.clone();
                                move |event: DragEvent| {
                                    dragging_id.set(Some(id.clone()));
                                    let transfer = event.data().data_transfer();
                                    transfer.set_effect_allowed("move");
                                    let _ = transfer.set_data("text/plain", &id);
                                }
                            },
                            ondragend: move |_| {
                                dragging_id.set(None);
                                drop_target.set(None);
                            },
                            ondragover: {
                                let id = id.clone();
                                move |event: DragEvent| {
                                    let dragging = match dragging_id() {
                                        Some(value) if value != id => value,
                                        _ => return,
                                    };
                                    let source = match find_resource(&internal_items(), &dragging) {
                                        Some(source) => source.clone(),
                                        None => return,
                                    };
                                    if contains_resource(&source, &id) {
                                        return;
                                    }
                                    event.prevent_default();
                                    event.stop_propagation();
                                    let ratio = event.data().element_coordinates().y / ROW_HEIGHT;
                                    let accepts = can_contain(&row_for_drag) && !row_for_drag.disabled;
                                    let position = if accepts && (0.25..=0.75).contains(&ratio) {
                                        SidebarResourceDropPosition::Inside
                                    } else if ratio < 0.5 {
                                        SidebarResourceDropPosition::Before
                                    } else {
                                        SidebarResourceDropPosition::After
                                    };
                                    drop_target.set(Some(DropTarget { id: Some(id.clone()), position }));
                                }
                            },
                            ondrop: {
                                let id = id.clone();
                                move |event: DragEvent| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    if let (Some(dragging), Some(target)) = (dragging_id(), drop_target()) {
                                        let _ = &id;
                                        perform_move.call(SidebarResourceMove {
                                            item_id: dragging,
                                            target_id: target.id,
                                            position: target.position,
                                        });
                                    }
                                }
                            },
                            span { "aria-hidden": "true", class: "grid size-5 shrink-0 place-items-center",
                                Icon { name: icon, class: "size-4" }
                            }
                            if renaming {
                                input {
                                    value: "{rename_draft()}",
                                    "aria-label": "Rename {item.label}",
                                    draggable: "false",
                                    onclick: move |event| event.stop_propagation(),
                                    ondoubleclick: move |event| event.stop_propagation(),
                                    oninput: move |event| rename_draft.set(event.value()),
                                    onblur: {
                                        let id = id.clone();
                                        let original = item.label.clone();
                                        move |_| {
                                            commit_rename(
                                                &id,
                                                &original,
                                                &rename_draft(),
                                                internal_items,
                                                set_items,
                                                renaming_id,
                                                announcement,
                                                on_rename,
                                            );
                                        }
                                    },
                                    onkeydown: {
                                        let id = id.clone();
                                        let original = item.label.clone();
                                        move |event: KeyboardEvent| {
                                            event.stop_propagation();
                                            if key_is(&event, Key::Enter) {
                                                commit_rename(
                                                    &id,
                                                    &original,
                                                    &rename_draft(),
                                                    internal_items,
                                                    set_items,
                                                    renaming_id,
                                                    announcement,
                                                    on_rename,
                                                );
                                            }
                                            if key_is(&event, Key::Escape) {
                                                renaming_id.set(None);
                                            }
                                        }
                                    },
                                    class: "mx-1 h-7 min-w-0 flex-1 rounded-md border border-border bg-background px-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring",
                                }
                            } else {
                                MarqueeLabel { active: hovered || menu_open, label: item.label.clone() }
                            }
                            if !renaming && !item.disabled {
                                div { class: "relative",
                                    button {
                                        r#type: "button",
                                        draggable: "false",
                                        tabindex: "-1",
                                        "aria-label": "Actions for {item.label}",
                                        onclick: {
                                            let id = id.clone();
                                            move |event| {
                                                event.stop_propagation();
                                                if menu_open_id().as_deref() == Some(id.as_str()) {
                                                    menu_open_id.set(None);
                                                    focus_row.call(id.clone());
                                                } else {
                                                    menu_open_id.set(Some(id.clone()));
                                                }
                                            }
                                        },
                                        class: cn!(
                                            "grid size-7 shrink-0 place-items-center rounded-lg outline-none transition-opacity hover:bg-foreground/5 focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-ring group-hover/resource:opacity-100 group-data-[menu-open=true]/resource:opacity-100",
                                            if can_touch() { "opacity-100" } else { "opacity-0" },
                                        ),
                                        Icon { name: IconName::MoreHorizontal, class: "size-4", "aria-hidden": "true" }
                                    }
                                    if menu_open {
                                        div { class: "absolute right-0 top-full z-50 mt-1 w-40 rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-md",
                                            div { "data-sidebar-resource-menu": "{id}",
                                                ResourceMenuAction {
                                                    icon: IconName::Pencil,
                                                    on_select: {
                                                        let id = id.clone();
                                                        move |_| {
                                                            menu_open_id.set(None);
                                                            if let Some(item) = find_resource(&internal_items(), &id) {
                                                                rename_draft.set(item.label.clone());
                                                            }
                                                            renaming_id.set(Some(id.clone()));
                                                        }
                                                    },
                                                    "Rename"
                                                }
                                                if up_target.is_some() || down_target.is_some() || into_target.is_some() || out_target.is_some() {
                                                    div { "aria-hidden": "true", class: "my-1 h-px bg-border" }
                                                }
                                                if let Some(target) = up_target.clone() {
                                                    ResourceMenuAction {
                                                        icon: IconName::ArrowUp,
                                                        on_select: {
                                                            let id = id.clone();
                                                            move |_| {
                                                                menu_open_id.set(None);
                                                                perform_move.call(SidebarResourceMove {
                                                                    item_id: id.clone(),
                                                                    target_id: Some(target.clone()),
                                                                    position: SidebarResourceDropPosition::Before,
                                                                });
                                                            }
                                                        },
                                                        "Move up"
                                                    }
                                                }
                                                if let Some(target) = down_target.clone() {
                                                    ResourceMenuAction {
                                                        icon: IconName::ArrowDown,
                                                        on_select: {
                                                            let id = id.clone();
                                                            move |_| {
                                                                menu_open_id.set(None);
                                                                perform_move.call(SidebarResourceMove {
                                                                    item_id: id.clone(),
                                                                    target_id: Some(target.clone()),
                                                                    position: SidebarResourceDropPosition::After,
                                                                });
                                                            }
                                                        },
                                                        "Move down"
                                                    }
                                                }
                                                if let Some((target, label)) = into_target.clone() {
                                                    ResourceMenuAction {
                                                        icon: IconName::FolderInput,
                                                        on_select: {
                                                            let id = id.clone();
                                                            move |_| {
                                                                menu_open_id.set(None);
                                                                {
                                                                    let mut current = expanded_ids();
                                                                    if !current.iter().any(|value| value == &target) {
                                                                        current.push(target.clone());
                                                                        expanded_ids.set(current);
                                                                    }
                                                                }
                                                                perform_move.call(SidebarResourceMove {
                                                                    item_id: id.clone(),
                                                                    target_id: Some(target.clone()),
                                                                    position: SidebarResourceDropPosition::Inside,
                                                                });
                                                            }
                                                        },
                                                        "Move into {label}"
                                                    }
                                                }
                                                if let Some(target) = out_target.clone() {
                                                    ResourceMenuAction {
                                                        icon: IconName::Undo2,
                                                        on_select: {
                                                            let id = id.clone();
                                                            move |_| {
                                                                menu_open_id.set(None);
                                                                perform_move.call(SidebarResourceMove {
                                                                    item_id: id.clone(),
                                                                    target_id: Some(target.clone()),
                                                                    position: SidebarResourceDropPosition::After,
                                                                });
                                                            }
                                                        },
                                                        "Move out"
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
            }

            if dragging_id().is_some() {
                div {
                    "aria-hidden": "true",
                    "data-active": if drop_target().is_none_or(|target| target.id.is_none()) { Some("true") } else { None },
                    class: "absolute inset-x-1 bottom-0 flex h-8 items-center justify-center rounded-lg border border-dashed border-border text-[10px] text-muted-foreground data-[active=true]:border-primary/50 data-[active=true]:bg-muted data-[active=true]:text-foreground",
                    "Move to top level"
                }
            }
        }
        style { dangerous_inner_html: "{MARQUEE_KEYFRAMES}" }
        span { class: "sr-only", "aria-live": "polite", "{announcement}" }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_key_down(
    event: KeyboardEvent,
    item: &SidebarResource,
    flat: &[FlatResource],
    mut expanded_ids: Signal<Vec<String>>,
    toggle: Callback<String>,
    select: Callback<String>,
    focus_row: Callback<String>,
    perform_move: Callback<SidebarResourceMove>,
    mut renaming_id: Signal<Option<String>>,
    mut menu_open_id: Signal<Option<String>>,
    mut rename_draft: Signal<String>,
) {
    let index = flat
        .iter()
        .position(|row| row.item.id == item.id)
        .unwrap_or(0);
    let previous = index.checked_sub(1).and_then(|i| flat.get(i));
    let next = flat.get(index + 1);
    let move_modifier = event.modifiers().alt() && event.modifiers().shift();
    let parent_id = flat.get(index).and_then(|row| row.parent_id.clone());

    if key_is(&event, Key::ArrowDown) && !move_modifier {
        if let Some(next) = next {
            event.prevent_default();
            focus_row.call(next.item.id.clone());
        }
        return;
    }
    if key_is(&event, Key::ArrowUp) && !move_modifier {
        if let Some(previous) = previous {
            event.prevent_default();
            focus_row.call(previous.item.id.clone());
        }
        return;
    }
    if key_is(&event, Key::Home) {
        if let Some(first) = flat.first() {
            event.prevent_default();
            focus_row.call(first.item.id.clone());
        }
        return;
    }
    if key_is(&event, Key::End) {
        if let Some(last) = flat.last() {
            event.prevent_default();
            focus_row.call(last.item.id.clone());
        }
        return;
    }

    if item.disabled {
        if key_is(&event, Key::ArrowLeft) {
            if let Some(parent) = parent_id {
                event.prevent_default();
                focus_row.call(parent);
            }
        }
        return;
    }

    if move_modifier && key_is(&event, Key::ArrowUp) {
        if let Some(previous) = previous {
            event.prevent_default();
            perform_move.call(SidebarResourceMove {
                item_id: item.id.clone(),
                target_id: Some(previous.item.id.clone()),
                position: SidebarResourceDropPosition::Before,
            });
        }
        return;
    }
    if move_modifier && key_is(&event, Key::ArrowDown) {
        if let Some(next) = next {
            event.prevent_default();
            perform_move.call(SidebarResourceMove {
                item_id: item.id.clone(),
                target_id: Some(next.item.id.clone()),
                position: SidebarResourceDropPosition::After,
            });
        }
        return;
    }
    if move_modifier && key_is(&event, Key::ArrowRight) {
        if let Some(previous) = previous.filter(|row| can_contain(&row.item)) {
            event.prevent_default();
            let mut current = expanded_ids();
            if !current.iter().any(|value| value == &previous.item.id) {
                current.push(previous.item.id.clone());
                expanded_ids.set(current);
            }
            perform_move.call(SidebarResourceMove {
                item_id: item.id.clone(),
                target_id: Some(previous.item.id.clone()),
                position: SidebarResourceDropPosition::Inside,
            });
        }
        return;
    }
    if move_modifier && key_is(&event, Key::ArrowLeft) {
        if let Some(parent) = parent_id {
            event.prevent_default();
            perform_move.call(SidebarResourceMove {
                item_id: item.id.clone(),
                target_id: Some(parent),
                position: SidebarResourceDropPosition::After,
            });
        }
        return;
    }

    if key_is(&event, Key::ArrowRight) && can_contain(item) {
        event.prevent_default();
        let is_expanded = expanded_ids().iter().any(|value| value == &item.id);
        if !is_expanded {
            toggle.call(item.id.clone());
        } else if let Some(next) =
            next.filter(|row| row.parent_id.as_deref() == Some(item.id.as_str()))
        {
            focus_row.call(next.item.id.clone());
        }
    } else if key_is(&event, Key::ArrowLeft) {
        event.prevent_default();
        let is_expanded = expanded_ids().iter().any(|value| value == &item.id);
        if is_expanded {
            toggle.call(item.id.clone());
        } else if let Some(parent) = parent_id {
            focus_row.call(parent);
        }
    } else if key_is(&event, Key::Enter) || space_pressed(&event) {
        event.prevent_default();
        if can_contain(item) {
            toggle.call(item.id.clone());
        } else {
            select.call(item.id.clone());
        }
    } else if key_is(&event, Key::F2) {
        event.prevent_default();
        rename_draft.set(item.label.clone());
        renaming_id.set(Some(item.id.clone()));
    } else if key_is(&event, Key::ContextMenu)
        || (event.modifiers().shift() && key_is(&event, Key::F10))
    {
        event.prevent_default();
        menu_open_id.set(Some(item.id.clone()));
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_rename(
    id: &str,
    original: &str,
    label: &str,
    internal_items: Signal<Vec<SidebarResource>>,
    set_items: Callback<Vec<SidebarResource>>,
    mut renaming_id: Signal<Option<String>>,
    mut announcement: Signal<String>,
    on_rename: Option<EventHandler<(SidebarResource, String)>>,
) {
    let trimmed = label.trim().to_string();
    renaming_id.set(None);
    if trimmed.is_empty() || trimmed == original {
        return;
    }
    let before = internal_items();
    let item = find_resource(&before, id).cloned();
    set_items.call(rename_resource(&before, id, &trimmed));
    if let (Some(item), Some(handler)) = (item, on_rename) {
        handler.call((item, trimmed));
    } else {
        announcement.set(format!("Renamed {original} to {trimmed}."));
    }
}

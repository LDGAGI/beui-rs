//! `motion/file-tree.tsx` — a keyboard-navigable tree with a shared hover pill.
//!
//! The source's `FileTreeFolder` / `FileTreeFile` parts are declarative markers
//! that the root reads back out of the children; a Dioxus `Element` cannot be
//! introspected, so the API is a [`FileTreeItem`] list (the same adaptation as
//! `SharedLayoutBg`). The hover pill is that component; row selection is a plain
//! class, as in the source.

use crate::icons::{ChevronRight, File, Folder, FolderOpen};
use crate::prelude::*;
use crate::shared_layout_bg::{SharedLayoutBg, SharedLayoutItem};
use beui_core::platform::{focus_selector, js_string};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager};

/// Whether a row is a file or a folder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileTreeKind {
    #[default]
    File,
    Folder,
}

/// One row (with optional children).
#[derive(Clone, PartialEq)]
pub struct FileTreeItem {
    pub value: String,
    pub name: String,
    pub kind: FileTreeKind,
    pub children: Vec<FileTreeItem>,
    pub icon: Option<Element>,
    pub disabled: bool,
    pub class: Option<String>,
}

impl FileTreeItem {
    /// A file row.
    pub fn file(value: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            name: name.into(),
            kind: FileTreeKind::File,
            children: Vec::new(),
            icon: None,
            disabled: false,
            class: None,
        }
    }

    /// A folder row.
    pub fn folder(value: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            kind: FileTreeKind::Folder,
            ..Self::file(value, name)
        }
    }

    /// Attach an icon.
    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Disable the row.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add a class to the row.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Set the children.
    pub fn children(mut self, children: Vec<FileTreeItem>) -> Self {
        self.children = children;
        self
    }
}

/// Per-part class overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileTreeClassNames {
    pub tree: Option<String>,
    pub item: Option<String>,
    pub icon: Option<String>,
    pub label: Option<String>,
}

/// Props for [`FileTree`].
#[derive(Props, Clone, PartialEq)]
pub struct FileTreeProps {
    pub items: Vec<FileTreeItem>,
    #[props(default)]
    pub value: Option<String>,
    #[props(default)]
    pub default_value: Option<String>,
    #[props(default)]
    pub on_value_change: Option<EventHandler<String>>,
    #[props(default)]
    pub expanded_ids: Option<Vec<String>>,
    #[props(default)]
    pub default_expanded_ids: Vec<String>,
    #[props(default)]
    pub on_expanded_change: Option<EventHandler<Vec<String>>>,
    #[props(default = "Files".to_string())]
    pub aria_label: String,
    /// Indent per depth level in px. Defaults to `18`.
    #[props(default = 18.0)]
    pub indent: f64,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub class_names: FileTreeClassNames,
}

/// One flattened row.
#[derive(Clone, PartialEq)]
struct FlatRow {
    item: FileTreeItem,
    depth: usize,
    parent_id: Option<String>,
    position: usize,
    set_size: usize,
}

fn flatten(
    items: &[FileTreeItem],
    expanded: &[String],
    depth: usize,
    parent_id: Option<&str>,
    out: &mut Vec<FlatRow>,
) {
    for (index, item) in items.iter().enumerate() {
        out.push(FlatRow {
            item: item.clone(),
            depth,
            parent_id: parent_id.map(|parent| parent.to_string()),
            position: index + 1,
            set_size: items.len(),
        });
        if item.kind == FileTreeKind::Folder
            && expanded.iter().any(|id| id == &item.value)
            && !item.children.is_empty()
        {
            flatten(&item.children, expanded, depth + 1, Some(&item.value), out);
        }
    }
}

/// The keyboard actions a row reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeNav {
    Down,
    Up,
    Home,
    End,
    Right,
    Left,
    Activate,
}

fn focus_row(value: &str) {
    let selector = format!("[data-beui-tree-row={}]", js_string(value));
    focus_selector(&selector, true);
}

/// A keyboard-navigable file tree.
#[component]
pub fn FileTree(props: FileTreeProps) -> Element {
    let FileTreeProps {
        items,
        value,
        default_value,
        on_value_change,
        expanded_ids,
        default_expanded_ids,
        on_expanded_change,
        aria_label,
        indent,
        class,
        class_names,
    } = props;

    let reduce = use_reduced_motion();
    let mut internal_value = use_signal(|| default_value.clone());
    let mut internal_expanded = use_signal(|| default_expanded_ids.clone());
    let mut focused = use_signal(|| value.clone().or(default_value.clone()));
    let mut nav_request = use_signal(|| None::<(String, TreeNav)>);
    let value_controlled = value.is_some();
    let expanded_controlled = expanded_ids.is_some();
    let selected_id = match value {
        Some(value) => Some(value),
        None => internal_value(),
    };
    let expanded = match &expanded_ids {
        Some(ids) => ids.clone(),
        None => internal_expanded(),
    };

    let mut rows = Vec::new();
    flatten(&items, &expanded, 0, None, &mut rows);
    let focused_id = focused()
        .filter(|id| rows.iter().any(|row| &row.item.value == id))
        .or_else(|| rows.first().map(|row| row.item.value.clone()));

    // A `Callback` is `Copy`, so this can be shared by every row and the
    // keyboard effect without cloning a closure per row.
    let activate = EventHandler::new({
        let expanded = expanded.clone();
        move |item: FileTreeItem| {
            if item.disabled {
                return;
            }
            if !value_controlled {
                internal_value.set(Some(item.value.clone()));
            }
            if let Some(handler) = on_value_change {
                handler.call(item.value.clone());
            }
            if item.kind != FileTreeKind::Folder {
                return;
            }
            let mut next = expanded.clone();
            if let Some(index) = next.iter().position(|id| id == &item.value) {
                next.remove(index);
            } else {
                next.push(item.value.clone());
            }
            if !expanded_controlled {
                internal_expanded.set(next.clone());
            }
            if let Some(handler) = on_expanded_change {
                handler.call(next);
            }
        }
    });
    let set_expanded = EventHandler::new(move |next: Vec<String>| {
        if !expanded_controlled {
            internal_expanded.set(next.clone());
        }
        if let Some(handler) = on_expanded_change {
            handler.call(next);
        }
    });

    // Keyboard navigation, resolved against the flattened rows. The request is
    // a `Signal`, so it is *read* here (tracked) rather than passed through
    // `use_reactive`, which only compares non-reactive data on a re-render.
    let rows_for_nav = rows.clone();
    let expanded_for_nav = expanded.clone();
    use_effect(move || {
        let Some((id, nav)) = nav_request.read().clone() else {
            return;
        };
        nav_request.set(None);
        let Some(index) = rows_for_nav.iter().position(|row| row.item.value == id) else {
            return;
        };
        let row = rows_for_nav[index].clone();
        let is_folder = row.item.kind == FileTreeKind::Folder;
        let is_open = expanded_for_nav.iter().any(|item| item == &row.item.value);
        let previous = index.checked_sub(1).and_then(|i| rows_for_nav.get(i));
        let next = rows_for_nav.get(index + 1);
        match nav {
            TreeNav::Down => {
                if let Some(next) = next {
                    focused.set(Some(next.item.value.clone()));
                    focus_row(&next.item.value);
                }
            }
            TreeNav::Up => {
                if let Some(previous) = previous {
                    focused.set(Some(previous.item.value.clone()));
                    focus_row(&previous.item.value);
                }
            }
            TreeNav::Home => {
                if let Some(first) = rows_for_nav.first() {
                    focused.set(Some(first.item.value.clone()));
                    focus_row(&first.item.value);
                }
            }
            TreeNav::End => {
                if let Some(last) = rows_for_nav.last() {
                    focused.set(Some(last.item.value.clone()));
                    focus_row(&last.item.value);
                }
            }
            TreeNav::Right => {
                if is_folder {
                    if !is_open && !row.item.disabled {
                        let mut next = expanded_for_nav.clone();
                        next.push(row.item.value.clone());
                        set_expanded.call(next);
                    } else if let Some(next) = next {
                        if next.parent_id.as_deref() == Some(row.item.value.as_str()) {
                            focused.set(Some(next.item.value.clone()));
                            focus_row(&next.item.value);
                        }
                    }
                }
            }
            TreeNav::Left => {
                if is_folder && is_open && !row.item.disabled {
                    let next: Vec<String> = expanded_for_nav
                        .iter()
                        .filter(|id| id != &&row.item.value)
                        .cloned()
                        .collect();
                    set_expanded.call(next);
                } else if let Some(parent) = row.parent_id.clone() {
                    focused.set(Some(parent.clone()));
                    focus_row(&parent);
                }
            }
            TreeNav::Activate => activate.call(row.item.clone()),
        }
    });

    let reduce_value = reduce();
    let indent_value = indent;
    let mut shared_items: Vec<SharedLayoutItem> = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        let value = row.item.value.clone();
        let is_open = row.item.kind == FileTreeKind::Folder
            && expanded.iter().any(|id| id == &row.item.value);
        let is_selected = selected_id.as_deref() == Some(row.item.value.as_str());
        let is_focused = focused_id.as_deref() == Some(row.item.value.as_str());
        let activate_item = row.item.clone();
        let nav_value = value.clone();
        let focus_value = value.clone();
        let class_names = class_names.clone();

        shared_items.push(SharedLayoutItem::new(
            value,
            rsx! {
                TreeRow {
                    row_value: nav_value.clone(),
                    name: row.item.name.clone(),
                    is_folder: row.item.kind == FileTreeKind::Folder,
                    is_open,
                    is_selected,
                    is_focused,
                    disabled: row.item.disabled,
                    icon: row.item.icon.clone(),
                    depth: row.depth,
                    position: row.position,
                    set_size: row.set_size,
                    indent: indent_value,
                    reduce: reduce_value,
                    row_class: row.item.class.clone(),
                    item_class: class_names.item.clone(),
                    icon_class: class_names.icon.clone(),
                    label_class: class_names.label.clone(),
                    on_activate: EventHandler::new(move |_| activate.call(activate_item.clone())),
                    on_focus: EventHandler::new(move |_| focused.set(Some(focus_value.clone()))),
                    on_nav: EventHandler::new(move |nav: TreeNav| {
                        nav_request.set(Some((nav_value.clone(), nav)));
                    }),
                }
            },
        ));
    }

    rsx! {
        SharedLayoutBg {
            items: shared_items,
            inset: 0.0,
            pill_class: "rounded-xl bg-muted",
            pill_container_class: "inset-y-auto top-0 h-9",
            class: cn!("min-w-0", class, class_names.tree.clone()),
            role: "tree",
            aria_label,
            aria_multiselectable: "false",
        }
    }
}

/// One tree row.
#[allow(clippy::too_many_arguments)]
#[component]
fn TreeRow(
    row_value: String,
    name: String,
    is_folder: bool,
    is_open: bool,
    is_selected: bool,
    is_focused: bool,
    disabled: bool,
    #[props(default)] icon: Option<Element>,
    depth: usize,
    position: usize,
    set_size: usize,
    indent: f64,
    reduce: bool,
    #[props(default)] row_class: Option<String>,
    #[props(default)] item_class: Option<String>,
    #[props(default)] icon_class: Option<String>,
    #[props(default)] label_class: Option<String>,
    on_activate: EventHandler<()>,
    on_focus: EventHandler<()>,
    on_nav: EventHandler<TreeNav>,
) -> Element {
    let mut enter = use_motion(0.0f32).expect("0.0 is finite");
    let mut chevron = use_motion(if is_open { 1.0f32 } else { 0.0 }).expect("0/1 are finite");
    use_effect(move || {
        let delay = if reduce {
            0
        } else {
            ((position as f64 * 0.025).min(0.1) * 1000.0).round() as u64
        };
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            beui_core::ease::tween_ease_out(220)
                .with_delay(dioxus_motion::Duration::from_millis(delay))
        };
        let _ = enter.animate_to(1.0, config);
    });
    use_effect(use_reactive((&is_open,), move |(is_open,)| {
        let config = if reduce {
            beui_core::ease::tween_ease_out(0)
        } else {
            AnimationConfig::spring(SPRING_SWAP)
        };
        let _ = chevron.animate_to(if is_open { 1.0 } else { 0.0 }, config);
    }));

    let enter_p = f64::from(*enter.current().read());
    let chevron_p = f64::from(*chevron.current().read());
    let row_style = format!(
        "opacity:{};transform:translateY({}px)",
        if disabled { 0.42 } else { 1.0 },
        -6.0 * (1.0 - enter_p)
    );
    let branch_style = format!(
        "left:{}px;opacity:{enter_p};transform:scaleY({enter_p})",
        16.0 + (depth as f64 - 1.0) * indent
    );
    let chevron_style = format!("transform:rotate({}deg)", chevron_p * 90.0);
    let folder_closed_style = format!("opacity:{}", 1.0 - chevron_p);
    let folder_open_style = format!("opacity:{chevron_p}");
    let button_class = cn!(
        "group/file-tree relative flex h-9 w-full items-center gap-2 overflow-hidden rounded-lg pr-2 text-left text-sm text-muted-foreground outline-none",
        "transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
        "aria-disabled:cursor-not-allowed",
        if is_selected { "bg-muted font-medium text-foreground" } else { "" },
        item_class,
        row_class,
    );
    let button_style = format!("padding-left:{}px", 8.0 + depth as f64 * indent);

    rsx! {
        div { style: row_style,
            button {
                r#type: "button",
                role: "treeitem",
                "data-beui-tree-row": "{row_value}",
                aria_level: "{depth + 1}",
                aria_posinset: "{position}",
                aria_setsize: "{set_size}",
                aria_selected: if is_selected { "true" } else { "false" },
                aria_expanded: if is_folder {
                    Some(if is_open { "true" } else { "false" })
                } else {
                    None
                },
                "aria-disabled": if disabled { Some("true") } else { None },
                tabindex: if is_focused { "0" } else { "-1" },
                class: button_class,
                style: button_style,
                onfocus: move |_| on_focus.call(()),
                onclick: move |_| on_activate.call(()),
                onkeydown: move |event: Event<KeyboardData>| {
                    let nav = match event.key() {
                        Key::ArrowDown => TreeNav::Down,
                        Key::ArrowUp => TreeNav::Up,
                        Key::Home => TreeNav::Home,
                        Key::End => TreeNav::End,
                        Key::ArrowRight => TreeNav::Right,
                        Key::ArrowLeft => TreeNav::Left,
                        Key::Enter => TreeNav::Activate,
                        Key::Character(ref text) if text == " " => TreeNav::Activate,
                        _ => return,
                    };
                    event.prevent_default();
                    on_nav.call(nav);
                },
                if depth > 0 {
                    span {
                        aria_hidden: "true",
                        class: "absolute top-0 bottom-0 w-px origin-top bg-border/70",
                        style: branch_style,
                    }
                }
                span {
                    aria_hidden: "true",
                    class: cn!(
                        "relative z-10 grid size-4 shrink-0 place-items-center",
                        if is_folder { "" } else { "opacity-0" },
                    ),
                    style: chevron_style,
                    ChevronRight { class: Some("size-3.5".to_string()) }
                }
                span {
                    aria_hidden: "true",
                    class: cn!(
                        "relative z-10 grid size-4 shrink-0 place-items-center text-muted-foreground transition-colors group-hover/file-tree:text-foreground",
                        if is_folder && is_open { "text-foreground" } else { "" },
                        icon_class,
                    ),
                    if let Some(icon) = icon {
                        {icon}
                    } else if is_folder {
                        span { class: "relative inline-grid size-4 place-items-center",
                            span {
                                class: "absolute inset-0 grid place-items-center",
                                style: folder_closed_style.clone(),
                                Folder { class: Some("size-4".to_string()) }
                            }
                            span {
                                class: "absolute inset-0 grid place-items-center",
                                style: folder_open_style.clone(),
                                FolderOpen { class: Some("size-4".to_string()) }
                            }
                        }
                    } else {
                        File { class: Some("size-4".to_string()) }
                    }
                }
                span { class: cn!("relative z-10 min-w-0 flex-1 truncate", label_class), "{name}" }
            }
        }
    }
}

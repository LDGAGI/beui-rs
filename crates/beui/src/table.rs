//! `motion/table/index.tsx` + `motion/table/*` — the data table.
//!
//! One table with row selection, per-column sort (asc → desc → off), column
//! resize, column reorder, inline editing and a fixed-row-height virtual
//! window; plus the two handles the source draws over the header and the
//! hovered row.
//!
//! Deviations from the source, all forced by the platform:
//!
//! - **Rows are data, not a generic `T`.** `TableColumn<T>` / `TableProps<T>`
//!   cannot be expressed over a Dioxus `Element` cell renderer, so a row is a
//!   [`TableRow`] carrying, per column key, an optional prebuilt `Element`
//!   (the source's `column.cell(row)`), a `text` value (its `row[key]`, also
//!   the editable input's value) and an explicit `sort` value (its
//!   `column.sortValue(row)`). `getRowId` goes away with it: the id lives on the
//!   row, and an empty one falls back to the row index.
//! - **A renamable column carries its header text.** The source reads
//!   `typeof column.header === "string" ? column.header : ""` for the rename
//!   input; an `Element` cannot be introspected, so the text travels in
//!   [`TableColumn::header_text`] and anything else renames from `""`.
//! - **The virtualizer is ported by hand.** `@tanstack/react-virtual` with a
//!   constant `estimateSize` is a window over `scrollTop`, so the port reads
//!   those numbers straight off the `onscroll` event (`scroll_top` /
//!   `client_height`) and slices the sorted rows with the same `overscan`.
//! - **The row/column handles and their menus render in place at
//!   `position: fixed`.** The source portals them to `document.body`, and a
//!   portaled subtree loses its `onclick` on the current Dioxus renderers.
//!   They hide from the table's own
//!   scroll listener instead of a window-level capture listener.
//! - **Sort compares lexicographically.** The source falls back to
//!   `String(a).localeCompare(String(b))`, which is locale-aware; the preview
//!   data is ASCII and the sorted columns differ only in case, so a plain
//!   comparison agrees with it there.
//! - **Reorder measures the header cells once at drag start** rather than on
//!   every pointer move; the header does not move while the drag is captured,
//!   so the drop index is the same.
//! - **`sort` is controlled by `Option<SortState>`** (`Some` = controlled), the
//!   convention the repo's other two-state components use, so a controlled
//!   `null` is not representable.
//! - **Escape is handled on the menu trigger**, where focus sits, rather than
//!   on a window listener.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use dioxus_motion::prelude::{Duration, MotionHandle, Time, TimeProvider};
use dioxus_motion::AnimationManager;

use crate::motion::motion_style;
use crate::prelude::*;
use beui_core::platform::Rect;

// --- constants --------------------------------------------------------------

/// `CHECKBOX_PX` — the select-all column, in px.
pub const CHECKBOX_PX: f64 = 48.0;
/// `CHECKBOX_WIDTH` — the same, as the CSS length on the `<col>`.
pub const CHECKBOX_WIDTH: &str = "48px";
/// `COLUMN_ACTIVE_SHADOW` — highlights the top edge of the active column header.
pub const COLUMN_ACTIVE_SHADOW: &str = "inset 0 1px 0 var(--color-primary)";

/// `INPUT_COLUMN_WIDTH` — the floor for a column of bare inputs.
const INPUT_COLUMN_WIDTH: f64 = 120.0;
/// `DEFAULT_ROOT_FONT_SIZE` — Tailwind's assumed root size, the pre-measure guess.
const DEFAULT_ROOT_FONT_SIZE: f64 = 16.0;
/// `MENU_WIDTH` — the row/column menu's fixed width, in px.
const MENU_WIDTH: f64 = 188.0;
/// The source's `deactivateColumn` / `deactivateRow` delayed leave, in ms.
const DEACTIVATE_DELAY_MS: u64 = 100;

// --- public types -----------------------------------------------------------

/// `align` — cell text alignment.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TableAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// `SortDirection`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// `SortState`.
#[derive(Clone, PartialEq, Debug)]
pub struct SortState {
    pub key: String,
    pub direction: SortDirection,
}

impl SortState {
    pub fn new(key: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            key: key.into(),
            direction,
        }
    }
}

/// `InsertPosition`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InsertPosition {
    Before,
    After,
}

/// The `string | number` a column sorts by.
#[derive(Clone, PartialEq, Debug)]
pub enum TableSortValue {
    Number(f64),
    Text(String),
}

impl TableSortValue {
    fn as_string(&self) -> String {
        match self {
            TableSortValue::Number(value) => js_number_text(*value),
            TableSortValue::Text(text) => text.clone(),
        }
    }
}

impl From<f64> for TableSortValue {
    fn from(value: f64) -> Self {
        TableSortValue::Number(value)
    }
}

impl From<i64> for TableSortValue {
    fn from(value: i64) -> Self {
        TableSortValue::Number(value as f64)
    }
}

impl From<&str> for TableSortValue {
    fn from(value: &str) -> Self {
        TableSortValue::Text(value.to_string())
    }
}

impl From<String> for TableSortValue {
    fn from(value: String) -> Self {
        TableSortValue::Text(value)
    }
}

/// `TableColumn<T>` — one column.
#[derive(Clone, PartialEq)]
pub struct TableColumn {
    /// Stable key; also the object property read for the cell + sort value.
    pub key: String,
    /// Header content.
    pub header: Element,
    /// The header as plain text, for the rename input (the source reads a
    /// string header directly; an `Element` cannot be taken apart).
    pub header_text: Option<String>,
    /// Allow clicking the header to sort by this column.
    pub sortable: bool,
    /// Cell text alignment.
    pub align: TableAlign,
    /// Column width as a CSS length, e.g. `160px` or `1.4fr`. `None` shares
    /// the remaining space equally.
    pub width: Option<String>,
    /// Render an inline text input for this column's cells (ignored when the
    /// row supplies a `cell` for the key).
    pub editable: bool,
}

impl TableColumn {
    pub fn new(key: impl Into<String>, header: Element) -> Self {
        Self {
            key: key.into(),
            header,
            header_text: None,
            sortable: false,
            align: TableAlign::Left,
            width: None,
            editable: false,
        }
    }

    /// A text header: sets both the rendered header and its rename value.
    pub fn text(key: impl Into<String>, label: impl Into<String>) -> Self {
        let label = label.into();
        Self::new(key, rsx! { "{label}" }).header_text(label)
    }

    pub fn header_text(mut self, text: impl Into<String>) -> Self {
        self.header_text = Some(text.into());
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn align(mut self, align: TableAlign) -> Self {
        self.align = align;
        self
    }

    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn editable(mut self) -> Self {
        self.editable = true;
        self
    }
}

/// `TableRow<T>` — a data row paired with its stable id.
#[derive(Clone, PartialEq, Default)]
pub struct TableRow {
    pub id: String,
    pub cells: BTreeMap<String, Element>,
    pub text: BTreeMap<String, String>,
    pub sort: BTreeMap<String, TableSortValue>,
}

impl TableRow {
    /// A row with a stable id.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Self::default()
        }
    }

    /// A prebuilt cell for `key` (the source's `column.cell`).
    pub fn cell(mut self, key: impl Into<String>, content: Element) -> Self {
        self.cells.insert(key.into(), content);
        self
    }

    /// The plain value for `key` (the source's `row[key]`).
    pub fn text(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.text.insert(key.into(), value.into());
        self
    }

    /// The explicit sort value for `key` (the source's `column.sortValue`).
    pub fn sort(mut self, key: impl Into<String>, value: impl Into<TableSortValue>) -> Self {
        self.sort.insert(key.into(), value.into());
        self
    }
}

/// The rows, shared and compared by pointer.
///
/// A `Vec<TableRow>` prop is cloned by the runtime on every re-render, and the
/// 10k-row preview re-renders on every scroll frame; holding the rows behind an
/// `Rc` makes the copy free, and the pointer comparison keeps the memo key and
/// the prop diff O(1) instead of walking ten thousand rows.
#[derive(Clone)]
pub struct TableRows(Rc<Vec<TableRow>>);

impl TableRows {
    pub fn new(rows: Vec<TableRow>) -> Self {
        Self(Rc::new(rows))
    }
}

impl PartialEq for TableRows {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for TableRows {
    type Target = Vec<TableRow>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<TableRow>> for TableRows {
    fn from(rows: Vec<TableRow>) -> Self {
        Self::new(rows)
    }
}

/// One entry of a row/column menu.
#[derive(Clone, PartialEq)]
pub struct TableMenuItem {
    pub label: String,
    pub icon: Option<Element>,
    pub destructive: bool,
    pub on_select: EventHandler<()>,
}

impl TableMenuItem {
    pub fn new(label: impl Into<String>, on_select: EventHandler<()>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            destructive: false,
            on_select,
        }
    }

    pub fn icon(mut self, icon: Element) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

// --- helpers ----------------------------------------------------------------

/// `alignFlex`.
fn align_flex(align: TableAlign) -> &'static str {
    match align {
        TableAlign::Right => "justify-end",
        TableAlign::Center => "justify-center",
        TableAlign::Left => "justify-start",
    }
}

/// `alignText`.
fn align_text(align: TableAlign) -> &'static str {
    match align {
        TableAlign::Right => "text-right",
        TableAlign::Center => "text-center",
        TableAlign::Left => "text-left",
    }
}

/// JS `String(number)` for the sort fallback: whole values drop their `.0`.
fn js_number_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1.0e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// `readSortValue(row, column)`.
fn read_sort_value(row: &TableRow, column: &TableColumn) -> TableSortValue {
    row.sort.get(&column.key).cloned().unwrap_or_else(|| {
        TableSortValue::Text(row.text.get(&column.key).cloned().unwrap_or_default())
    })
}

/// Whether a column's cells are bare inputs — what `minTableWidth` falls back
/// to for content it cannot measure.
fn is_input_only(data: &[TableRow], column: &TableColumn, renamable: bool) -> bool {
    renamable || (column.editable && !data.iter().any(|row| row.cells.contains_key(&column.key)))
}

/// `resolveColumnWidth` — the absolute width a column declared, or `None` when
/// it declared a share of the remainder (`fr`, `%`, `auto`, `calc()`, nothing).
fn resolve_column_width(width: Option<&str>, root_font_size: f64) -> Option<f64> {
    let width = width?;
    let token: String = width
        .chars()
        .skip_while(|c| !c.is_ascii_digit() && *c != '.' && *c != '-')
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    let value: f64 = token.parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    if width.ends_with("px") {
        return Some(value);
    }
    if width.ends_with("rem") {
        return Some(value * root_font_size);
    }
    None
}

/// `useColumnReorder`'s `orderedColumns` memo: apply the order, placing columns
/// added at runtime where their left neighbor is rather than appending them.
fn order_columns(columns: &[TableColumn], order: &[String]) -> Vec<TableColumn> {
    let by_key: BTreeMap<&str, &TableColumn> = columns
        .iter()
        .map(|column| (column.key.as_str(), column))
        .collect();
    let mut result_keys: Vec<String> = order
        .iter()
        .filter(|key| by_key.contains_key(key.as_str()))
        .cloned()
        .collect();
    let mut present: BTreeSet<String> = result_keys.iter().cloned().collect();
    for (index, column) in columns.iter().enumerate() {
        if present.contains(&column.key) {
            continue;
        }
        let at = if index == 0 {
            0
        } else {
            match result_keys
                .iter()
                .position(|key| key == &columns[index - 1].key)
            {
                Some(position) => position + 1,
                None => index,
            }
        };
        let at = at.min(result_keys.len());
        result_keys.insert(at, column.key.clone());
        present.insert(column.key.clone());
    }
    result_keys
        .iter()
        .filter_map(|key| by_key.get(key.as_str()).copied().cloned())
        .collect()
}

/// `dropIndexFor` over a rect snapshot.
fn drop_index_for(rects: &[(String, Rect)], client_x: f64) -> usize {
    for (index, (_, rect)) in rects.iter().enumerate() {
        if client_x < rect.left + rect.width / 2.0 {
            return index;
        }
    }
    rects.len()
}

/// The current value of a number motion, clamped for a 0→1 track.
fn progress(handle: &MotionHandle<f32>) -> f32 {
    handle.get_value().clamp(0.0, 1.0)
}

// --- header environment -----------------------------------------------------

/// Everything a header cell needs from the table around it.
#[derive(Clone, PartialEq)]
struct HeaderEnv {
    reduce: bool,
    row_height: f64,
    resizable: bool,
    reorderable: bool,
    column_count: usize,
    sort: Option<SortState>,
    active_column: Option<String>,
    drag_key: Option<String>,
    drop_index: Option<usize>,
    th_refs: Signal<BTreeMap<String, Rc<MountedData>>>,
    on_toggle_sort: EventHandler<String>,
    on_activate_column: EventHandler<String>,
    on_deactivate_column: EventHandler<()>,
    on_resize_start: EventHandler<(String, f64)>,
    on_resize_move: EventHandler<f64>,
    on_resize_end: EventHandler<()>,
    on_reorder_start: EventHandler<(String, f64)>,
    on_reorder_move: EventHandler<f64>,
    on_reorder_end: EventHandler<()>,
    on_column_rename: Option<EventHandler<(String, String)>>,
}

// --- the table --------------------------------------------------------------

/// `Table`.
#[component]
pub fn Table(
    /// The rows, in source order, behind an `Rc` (see [`TableRows`]).
    data: TableRows,
    columns: Vec<TableColumn>,
    #[props(default = false)] selectable: bool,
    #[props(default = None)] selected_row_ids: Option<Vec<String>>,
    #[props(default = None)] default_selected_row_ids: Option<Vec<String>>,
    #[props(default = None)] on_selection_change: Option<EventHandler<Vec<String>>>,
    #[props(default = None)] sort: Option<SortState>,
    #[props(default = None)] default_sort: Option<SortState>,
    #[props(default = None)] on_sort_change: Option<EventHandler<Option<SortState>>>,
    #[props(default = false)] resizable: bool,
    #[props(default = 64.0)] min_column_width: f64,
    #[props(default = None)] on_column_resize: Option<EventHandler<(String, f64)>>,
    #[props(default = false)] reorderable: bool,
    #[props(default = None)] on_column_order_change: Option<EventHandler<Vec<String>>>,
    #[props(default = None)] on_cell_edit: Option<EventHandler<(String, String, String)>>,
    #[props(default = None)] on_column_rename: Option<EventHandler<(String, String)>>,
    #[props(default = None)] on_insert_row: Option<EventHandler<(usize, InsertPosition)>>,
    #[props(default = None)] on_delete_row: Option<EventHandler<(String, usize)>>,
    #[props(default = None)] on_insert_column: Option<EventHandler<(usize, InsertPosition)>>,
    #[props(default = None)] on_delete_column: Option<EventHandler<(String, usize)>>,
    #[props(default = 48.0)] row_height: f64,
    #[props(default = 440.0)] height: f64,
    #[props(default = 10)] overscan: usize,
    #[props(default = None)] on_end_reached: Option<EventHandler<()>>,
    #[props(default = false)] loading: bool,
    #[props(default = 3)] skeleton_rows: usize,
    #[props(default = None)] empty_state: Option<Element>,
    #[props(default = None)] class: Option<String>,
) -> Element {
    let reduce = use_reduced_motion()();

    // --- reorder --------------------------------------------------------
    let mut order = use_signal(|| {
        columns
            .iter()
            .map(|column| column.key.clone())
            .collect::<Vec<_>>()
    });
    let mut drag_key = use_signal(|| None::<String>);
    let mut drop_index = use_signal(|| None::<usize>);
    let mut reorder_rects = use_signal(Vec::<(String, Rect)>::new);

    // --- resize ---------------------------------------------------------
    let mut widths = use_signal(BTreeMap::<String, f64>::new);
    let mut resize = use_signal(|| None::<(String, f64, f64)>);

    // --- sort / selection ------------------------------------------------
    let mut internal_sort = use_signal(|| default_sort.clone());
    // The rendered selection is always signal-backed (so event closures capture a
    // `Copy` handle rather than the `Vec`); an incoming controlled prop wins
    // whenever it differs, and a toggle also lands here so a parent that ignores
    // `on_selection_change` cannot freeze the checkboxes.
    let mut selection = use_signal(|| default_selected_row_ids.clone().unwrap_or_default());
    if let Some(controlled) = selected_row_ids.as_ref() {
        if selection.peek().as_slice() != controlled.as_slice() {
            selection.set(controlled.clone());
        }
    }

    // --- virtualization / async -----------------------------------------
    let mut scroll_top = use_signal(|| 0.0f64);
    let mut viewport_h = use_signal(|| height);
    let mut end_reached = use_signal(|| false);

    // --- handles ---------------------------------------------------------
    let th_refs: Signal<BTreeMap<String, Rc<MountedData>>> = use_signal(BTreeMap::new);
    let mut row_refs: Signal<BTreeMap<String, Rc<MountedData>>> = use_signal(BTreeMap::new);
    let mut active_column = use_signal(|| None::<String>);
    let mut column_coords = use_signal(|| None::<(f64, f64)>);
    let mut column_token = use_signal(|| 0u32);
    let mut active_row = use_signal(|| None::<(String, usize)>);
    let mut row_coords = use_signal(|| None::<(f64, f64)>);
    let mut row_token = use_signal(|| 0u32);

    // --- root font size (the pre-measure guess, then measured once) -------
    let mut root_font_size = use_signal(|| DEFAULT_ROOT_FONT_SIZE);
    use_effect(move || {
        spawn(async move {
            let script = "const s = getComputedStyle(document.documentElement).fontSize;\n\
                          return parseFloat(s);";
            if let Some(measured) = platform::json::<f64>(script).await {
                if measured > 0.0 {
                    root_font_size.set(measured);
                }
            }
        });
    });

    // --- derive order / sort / selection ---------------------------------
    let ordered_columns: Rc<Vec<TableColumn>> = Rc::new(order_columns(&columns, &order.read()));
    let current_sort = if sort.is_some() {
        sort.clone()
    } else {
        internal_sort.read().clone()
    };
    let current_selected = selection.read().clone();

    // `useColumnSort`'s `sortedRows`, as the index order into `data` so a
    // re-render (every scroll frame) never copies the rows. `use_memo` keys it
    // on the rows, the columns and the sort.
    let sorted_indices = use_memo(use_reactive!(|data, current_sort, columns| {
        let mut indices: Vec<usize> = (0..data.len()).collect();
        if let Some(state) = current_sort.clone() {
            if let Some(column) = columns.iter().find(|column| column.key == state.key) {
                // Stable, as `Array.prototype.sort` is.
                indices.sort_by(|a, b| {
                    let av = read_sort_value(&data[*a], column);
                    let bv = read_sort_value(&data[*b], column);
                    let cmp = match (&av, &bv) {
                        (TableSortValue::Number(x), TableSortValue::Number(y)) => {
                            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        _ => av.as_string().cmp(&bv.as_string()),
                    };
                    match state.direction {
                        SortDirection::Asc => cmp,
                        SortDirection::Desc => cmp.reverse(),
                    }
                });
            }
        }
        indices
    }));

    // `useRowSelection`'s commit, then its two toggles.
    let commit_selection = EventHandler::new(move |next: Vec<String>| {
        selection.set(next.clone());
        if let Some(callback) = on_selection_change {
            callback.call(next);
        }
    });
    let all_selected = !sorted_indices.is_empty()
        && sorted_indices
            .iter()
            .all(|index| current_selected.contains(&data[*index].id));
    let some_selected = sorted_indices
        .iter()
        .any(|index| current_selected.contains(&data[*index].id));
    let toggle_all = {
        let indices = sorted_indices;
        let rows = data.clone();
        let selected = current_selected.clone();
        EventHandler::new(move |_: ()| {
            let mut next = selected.clone();
            if !indices.is_empty() && indices.iter().all(|index| next.contains(&rows[*index].id)) {
                for index in indices.iter() {
                    let id = &rows[*index].id;
                    next.retain(|candidate| candidate != id);
                }
            } else {
                for index in indices.iter() {
                    let id = &rows[*index].id;
                    if !next.contains(id) {
                        next.push(id.clone());
                    }
                }
            }
            commit_selection.call(next);
        })
    };

    // `useColumnSort`'s `toggleSort`.
    let toggle_sort = EventHandler::new(move |key: String| {
        let current = if sort.is_some() {
            sort.clone()
        } else {
            internal_sort.read().clone()
        };
        let next = match current {
            None => Some(SortState::new(key, SortDirection::Asc)),
            Some(state) if state.key != key => Some(SortState::new(key, SortDirection::Asc)),
            Some(state) if state.direction == SortDirection::Asc => {
                Some(SortState::new(key, SortDirection::Desc))
            }
            Some(_) => None,
        };
        if sort.is_none() {
            internal_sort.set(next.clone());
        }
        if let Some(callback) = on_sort_change {
            callback.call(next);
        }
    });

    // --- resize ----------------------------------------------------------
    let on_resize_start = {
        let ordered = ordered_columns.clone();
        EventHandler::new(move |(key, client_x): (String, f64)| {
            let ordered = ordered.clone();
            let existing = widths.read().clone();
            let measured_min = min_column_width;
            let refs = th_refs;
            spawn(async move {
                let mut snapshot = existing;
                for column in ordered.iter() {
                    if snapshot.contains_key(&column.key) {
                        continue;
                    }
                    let node = refs.peek().get(&column.key).cloned();
                    let measured = match node {
                        Some(node) => platform::client_rect(&node)
                            .await
                            .map(|rect| rect.width)
                            .filter(|width| *width > 0.0)
                            .map(|width| width.round()),
                        None => None,
                    };
                    snapshot.insert(column.key.clone(), measured.unwrap_or(measured_min));
                }
                let start_width = snapshot.get(&key).copied().unwrap_or(measured_min);
                widths.set(snapshot);
                resize.set(Some((key, client_x, start_width)));
            });
        })
    };
    let on_resize_move = EventHandler::new(move |client_x: f64| {
        let Some((key, start_x, start_width)) = resize.read().clone() else {
            return;
        };
        let width = min_column_width.max(start_width + (client_x - start_x));
        let mut next = widths.read().clone();
        next.insert(key, width);
        widths.set(next);
    });
    let on_resize_end = EventHandler::new(move |_: ()| {
        let Some((key, _, start_width)) = resize.read().clone() else {
            return;
        };
        resize.set(None);
        if let Some(callback) = on_column_resize {
            let width = widths.read().get(&key).copied().unwrap_or(start_width);
            callback.call((key, width));
        }
    });

    // --- reorder ---------------------------------------------------------
    let on_reorder_start = {
        let ordered = ordered_columns.clone();
        EventHandler::new(move |(key, _client_x): (String, f64)| {
            drag_key.set(Some(key));
            let ordered = ordered.clone();
            let refs = th_refs;
            spawn(async move {
                let mut rects = Vec::with_capacity(ordered.len());
                for column in ordered.iter() {
                    let node = refs.peek().get(&column.key).cloned();
                    let Some(node) = node else {
                        continue;
                    };
                    if let Some(rect) = platform::client_rect(&node).await {
                        rects.push((column.key.clone(), rect));
                    }
                }
                reorder_rects.set(rects);
            });
        })
    };
    let on_reorder_move = EventHandler::new(move |client_x: f64| {
        if drag_key.read().is_none() {
            return;
        }
        let rects = reorder_rects.read().clone();
        drop_index.set(Some(drop_index_for(&rects, client_x)));
    });
    let on_reorder_end = {
        let ordered = ordered_columns.clone();
        EventHandler::new(move |_: ()| {
            let dragged = drag_key.read().clone();
            let target = *drop_index.read();
            if let (Some(dragged), Some(target)) = (dragged, target) {
                let keys: Vec<String> = ordered.iter().map(|column| column.key.clone()).collect();
                if let Some(from) = keys.iter().position(|key| key == &dragged) {
                    let mut without: Vec<String> = keys
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index != from)
                        .map(|(_, key)| key.clone())
                        .collect();
                    let mut to = target;
                    if from < to {
                        to -= 1;
                    }
                    let to = to.min(without.len());
                    without.insert(to, dragged);
                    order.set(without.clone());
                    if let Some(callback) = on_column_order_change {
                        callback.call(without);
                    }
                }
            }
            drag_key.set(None);
            drop_index.set(None);
        })
    };

    // --- column / row activation (100 ms leave grace) ---------------------
    let activate_column = EventHandler::new(move |key: String| {
        let next = *column_token.read() + 1;
        column_token.set(next);
        active_column.set(Some(key));
    });
    let deactivate_column = EventHandler::new(move |_: ()| {
        let token = *column_token.read() + 1;
        column_token.set(token);
        spawn(async move {
            if Time::delay(Duration::from_millis(DEACTIVATE_DELAY_MS))
                .await
                .is_err()
            {
                return;
            }
            if *column_token.read() == token {
                active_column.set(None);
                column_coords.set(None);
            }
        });
    });
    let activate_row = EventHandler::new(move |(id, index): (String, usize)| {
        let next = *row_token.read() + 1;
        row_token.set(next);
        active_row.set(Some((id, index)));
    });
    let deactivate_row = EventHandler::new(move |_: ()| {
        let token = *row_token.read() + 1;
        row_token.set(token);
        spawn(async move {
            if Time::delay(Duration::from_millis(DEACTIVATE_DELAY_MS))
                .await
                .is_err()
            {
                return;
            }
            if *row_token.read() == token {
                active_row.set(None);
                row_coords.set(None);
            }
        });
    });

    // Follow the active column/row to place its fixed-position handle.
    use_effect(move || {
        let Some(key) = active_column.read().clone() else {
            column_coords.set(None);
            return;
        };
        let node = th_refs.peek().get(&key).cloned();
        let Some(node) = node else {
            return;
        };
        spawn(async move {
            if let Some(rect) = platform::client_rect(&node).await {
                if active_column.read().as_deref() == Some(key.as_str()) {
                    column_coords.set(Some((rect.left + rect.width / 2.0, rect.top)));
                }
            }
        });
    });
    use_effect(move || {
        let Some((id, index)) = active_row.read().clone() else {
            row_coords.set(None);
            return;
        };
        let node = row_refs.peek().get(&id).cloned();
        let Some(node) = node else {
            return;
        };
        spawn(async move {
            if let Some(rect) = platform::client_rect(&node).await {
                if active_row.read().as_ref() == Some(&(id.clone(), index)) {
                    row_coords.set(Some((rect.left, rect.top + rect.height / 2.0)));
                }
            }
        });
    });

    // `loading` resetting the end-reached guard.
    use_effect(use_reactive!(|loading| {
        if !loading {
            end_reached.set(false);
        }
    }));

    // --- scroll -----------------------------------------------------------
    let on_scroll = move |event: ScrollEvent| {
        let data = event.data();
        scroll_top.set(data.scroll_top());
        viewport_h.set(data.client_height() as f64);
        // The source hides both handles on any scroll (window capture listener);
        // the table's own viewport is the only scroll that moves them here.
        active_column.set(None);
        column_coords.set(None);
        active_row.set(None);
        row_coords.set(None);
        let Some(callback) = on_end_reached else {
            return;
        };
        if loading || *end_reached.read() {
            return;
        }
        let remaining =
            data.scroll_height() as f64 - data.scroll_top() - data.client_height() as f64;
        if remaining < row_height * 4.0 {
            end_reached.set(true);
            callback.call(());
        }
    };

    // --- layout numbers ---------------------------------------------------
    let sized = !ordered_columns.is_empty()
        && ordered_columns
            .iter()
            .all(|column| widths.read().contains_key(&column.key));
    let has_row_menu = on_insert_row.is_some() || on_delete_row.is_some();
    let has_column_menu = on_insert_column.is_some() || on_delete_column.is_some();
    let lead_columns = columns.len() + usize::from(selectable);
    let min_table_width = {
        let mut total = if selectable { CHECKBOX_PX } else { 0.0 };
        for column in ordered_columns.iter() {
            if let Some(width) = widths.read().get(&column.key) {
                total += *width;
                continue;
            }
            if let Some(declared) =
                resolve_column_width(column.width.as_deref(), *root_font_size.read())
            {
                total += declared;
                continue;
            }
            total += if is_input_only(data.as_slice(), column, on_column_rename.is_some()) {
                min_column_width.max(INPUT_COLUMN_WIDTH)
            } else {
                min_column_width
            };
        }
        total.round() as i64
    };

    // --- the virtual window ------------------------------------------------
    let total_size = sorted_indices.len() as f64 * row_height;
    let viewport = viewport_h.read().max(0.0);
    let visible_top = *scroll_top.read();
    let first = (((visible_top / row_height).floor() as isize) - overscan as isize).max(0) as usize;
    let last = ((((visible_top + viewport) / row_height).ceil() as usize) + overscan)
        .min(sorted_indices.len());
    let first = first.min(last);
    let padding_top = first as f64 * row_height;
    let padding_bottom = total_size - last as f64 * row_height;
    let visible_rows: Vec<(usize, &TableRow)> = sorted_indices
        .iter()
        .enumerate()
        .skip(first)
        .take(last - first)
        .map(|(index, data_index)| (index, &data[*data_index]))
        .collect();

    let active_column_key = active_column.read().clone();
    let active_column_index = active_column_key
        .as_deref()
        .and_then(|key| ordered_columns.iter().position(|column| column.key == key))
        .unwrap_or(0);

    let header_env = HeaderEnv {
        reduce,
        row_height,
        resizable,
        reorderable,
        column_count: ordered_columns.len(),
        sort: current_sort.clone(),
        active_column: active_column.read().clone(),
        drag_key: drag_key.read().clone(),
        drop_index: *drop_index.read(),
        th_refs,
        on_toggle_sort: toggle_sort,
        on_activate_column: activate_column,
        on_deactivate_column: deactivate_column,
        on_resize_start,
        on_resize_move,
        on_resize_end,
        on_reorder_start,
        on_reorder_move,
        on_reorder_end,
        on_column_rename,
    };

    let table_style = format!("table-layout:fixed;min-width:max(100%,{min_table_width}px)");
    let scroll_style = format!("height:{height}px");
    let empty = empty_state.clone().unwrap_or_else(|| rsx! { "No data" });
    let widths_now = widths.read().clone();

    rsx! {
        div {
            class: cn!("w-full overflow-hidden border border-border bg-background text-sm", class),
            div {
                class: "overflow-auto",
                style: scroll_style,
                onscroll: on_scroll,
                table {
                    class: cn!("border-collapse", sized.then_some("w-max")),
                    style: table_style,
                    colgroup {
                        if selectable {
                            col { style: "width:{CHECKBOX_WIDTH}" }
                        }
                        for column in ordered_columns.iter() {
                            col {
                                key: "{column.key}",
                                style: widths_now
                                    .get(&column.key)
                                    .map(|width| format!("width:{width}px"))
                                    .or_else(|| column.width.clone().map(|width| format!("width:{width}"))),
                            }
                        }
                        // Empty filler owns the leftover space — no gap.
                        col {}
                    }
                    thead {
                        tr { style: "height:{row_height}px",
                            if selectable {
                                th { class: "sticky top-0 z-10 border-border border-b bg-muted",
                                    div { class: "flex items-center justify-center",
                                        Checkbox {
                                            checked: all_selected,
                                            indeterminate: !all_selected && some_selected,
                                            on_checked_change: move |_| toggle_all.call(()),
                                            aria_label: "Select all rows",
                                            class: "size-6",
                                        }
                                    }
                                }
                            }
                            for (index, column) in ordered_columns.iter().enumerate() {
                                TableHeaderCell {
                                    key: "{column.key}",
                                    column: column.clone(),
                                    index,
                                    env: header_env.clone(),
                                }
                            }
                            th { aria_hidden: "true", class: "sticky top-0 z-10 border-border border-b bg-muted" }
                        }
                    }
                    tbody {
                        if sorted_indices.is_empty() {
                            if loading {
                                SkeletonRows {
                                    count: ((height / row_height).ceil() as usize).max(1),
                                    columns: ordered_columns.as_ref().clone(),
                                    selectable,
                                    row_height,
                                }
                            } else {
                                tr {
                                    td {
                                        colspan: lead_columns + 1,
                                        class: "p-10 text-center text-muted-foreground",
                                        {empty}
                                    }
                                }
                            }
                        } else {
                            if padding_top > 0.0 {
                                tr { aria_hidden: "true", style: "height:{padding_top}px",
                                    td { colspan: lead_columns + 1 }
                                }
                            }
                            for (index, entry) in visible_rows.iter().copied() {
                                tr {
                                    key: "{entry.id}",
                                    "data-selected": if current_selected.contains(&entry.id) { "true" } else { "false" },
                                    style: "height:{row_height}px",
                                    class: "border-border/60 border-b transition-colors data-[selected=true]:bg-muted/60 hover:bg-muted/50",
                                    onmounted: {
                                        let register_id = entry.id.clone();
                                        move |event| {
                                            row_refs.write().insert(register_id.clone(), event.data());
                                        }
                                    },
                                    onpointerenter: {
                                        let enter_id = entry.id.clone();
                                        move |_| {
                                            if has_row_menu {
                                                activate_row.call((enter_id.clone(), index));
                                            }
                                        }
                                    },
                                    onpointerleave: move |_| {
                                        if has_row_menu {
                                            deactivate_row.call(());
                                        }
                                    },
                                    if selectable {
                                        td { class: "text-center",
                                            div { class: "flex items-center justify-center",
                                                Checkbox {
                                                    checked: current_selected.contains(&entry.id),
                                                    on_checked_change: {
                                                        let toggle_id = entry.id.clone();
                                                        move |_| {
                                                            let mut next = selection.read().clone();
                                                            match next.iter().position(|id| id == &toggle_id) {
                                                                Some(position) => {
                                                                    next.remove(position);
                                                                }
                                                                None => next.push(toggle_id.clone()),
                                                            }
                                                            commit_selection.call(next);
                                                        }
                                                    },
                                                    aria_label: "Select row {index + 1}",
                                                }
                                            }
                                        }
                                    }
                                    for column in ordered_columns.iter() {
                                        td {
                                            key: "{column.key}",
                                            class: cn!("truncate px-4 text-foreground", align_text(column.align)),
                                            if let Some(cell) = entry.cells.get(&column.key) {
                                                {cell.clone()}
                                            } else if column.editable {
                                                EditableCell {
                                                    value: entry.text.get(&column.key).cloned().unwrap_or_default(),
                                                    label: "{column.key} for row {index + 1}",
                                                    on_change: {
                                                        let edit_id = entry.id.clone();
                                                        let edit_key = column.key.clone();
                                                        move |next: String| {
                                                            if let Some(callback) = on_cell_edit {
                                                                callback.call((edit_id.clone(), edit_key.clone(), next));
                                                            }
                                                        }
                                                    },
                                                }
                                            } else {
                                                {entry.text.get(&column.key).cloned().unwrap_or_default()}
                                            }
                                        }
                                    }
                                    td { aria_hidden: "true" }
                                }
                            }
                            if padding_bottom > 0.0 {
                                tr { aria_hidden: "true", style: "height:{padding_bottom}px",
                                    td { colspan: lead_columns + 1 }
                                }
                            }
                            if loading {
                                SkeletonRows {
                                    count: skeleton_rows,
                                    columns: ordered_columns.as_ref().clone(),
                                    selectable,
                                    row_height,
                                }
                            }
                        }
                    }
                }
            }
            if has_row_menu {
                if let (Some((id, index)), Some((x, y))) = (active_row.read().clone(), *row_coords.read()) {
                    RowHandle {
                        id,
                        index,
                        x,
                        y,
                        on_insert_row,
                        on_delete_row,
                        on_enter: move |_| {
                            if let Some((id, index)) = active_row.read().clone() {
                                activate_row.call((id, index));
                            }
                        },
                        on_leave: move |_| deactivate_row.call(()),
                    }
                }
            }
            if has_column_menu {
                if let (Some(key), Some((x, y))) = (active_column_key.clone(), *column_coords.read()) {
                    ColumnHandle {
                        column_key: key,
                        index: active_column_index,
                        x,
                        y,
                        on_insert_column,
                        on_delete_column,
                        on_enter: move |_| {
                            if let Some(key) = active_column.read().clone() {
                                activate_column.call(key);
                            }
                        },
                        on_leave: move |_| deactivate_column.call(()),
                    }
                }
            }
        }
    }
}

// --- header cell ------------------------------------------------------------

#[component]
fn TableHeaderCell(column: TableColumn, index: usize, env: HeaderEnv) -> Element {
    let HeaderEnv {
        reduce,
        row_height,
        resizable,
        reorderable,
        column_count,
        sort,
        active_column,
        drag_key,
        drop_index,
        mut th_refs,
        on_toggle_sort,
        on_activate_column,
        on_deactivate_column,
        on_resize_start,
        on_resize_move,
        on_resize_end,
        on_reorder_start,
        on_reorder_move,
        on_reorder_end,
        on_column_rename,
    } = env;

    let key = column.key.clone();
    let active = sort.as_ref().is_some_and(|state| state.key == key);
    let descending = sort
        .as_ref()
        .is_some_and(|state| state.key == key && state.direction == SortDirection::Desc);
    let is_dragging = drag_key.as_deref() == Some(key.as_str());
    let is_active = active_column.as_deref() == Some(key.as_str());
    let is_drop = drag_key.is_some() && drop_index == Some(index);
    let is_drop_end =
        drag_key.is_some() && drop_index == Some(column_count) && index + 1 == column_count;

    let mut cell = use_style_motion(motion_style! { scale: 1.0, opacity: 1.0 });
    let mut chevron = use_style_motion(motion_style! {
        rotate: if descending { 180.0 } else { 0.0 },
        opacity: if active { 1.0 } else { 0.35 },
    });

    // `animate={{ scale, opacity }}` with `SPRING_PRESS` while dragging.
    use_effect(use_reactive!(|is_dragging, reduce| {
        let target = if reduce {
            motion_style! {
                scale: 1.0,
                opacity: if is_dragging { 0.5 } else { 1.0 },
            }
        } else {
            motion_style! {
                scale: if is_dragging { 1.04 } else { 1.0 },
                opacity: if is_dragging { 0.5 } else { 1.0 },
            }
        };
        let config = if reduce {
            tween(0, Ease::Out)
        } else {
            spring(SPRING_PRESS)
        };
        animate_style(&mut cell, target, config);
    }));

    // `transition={{ duration: 0.18, ease: EASE_OUT }}` on the sort chevron.
    use_effect(use_reactive!(|active, descending, reduce| {
        let target = motion_style! {
            rotate: if descending { 180.0 } else { 0.0 },
            opacity: if active { 1.0 } else { 0.35 },
        };
        let config = if reduce {
            tween(0, Ease::Out)
        } else {
            tween(180, Ease::Out)
        };
        animate_style(&mut chevron, target, config);
    }));

    let resize_key = key.clone();
    let reorder_key = key.clone();
    let sort_key = key.clone();
    let rename_key = key.clone();
    let mount_key = key.clone();
    let register_key = key.clone();

    rsx! {
        th {
            "aria-sort": if active {
                Some(if descending { "descending" } else { "ascending" }.to_string())
            } else {
                None
            },
            style: if is_active { Some(format!("box-shadow:{COLUMN_ACTIVE_SHADOW}")) } else { None },
            "data-drop": is_drop.then(|| "true".to_string()),
            "data-dropend": is_drop_end.then(|| "true".to_string()),
            class: "group sticky top-0 z-10 border-border border-b bg-muted p-0 font-medium text-muted-foreground data-[drop=true]:before:absolute data-[drop=true]:before:inset-y-0 data-[drop=true]:before:left-0 data-[drop=true]:before:w-0.5 data-[drop=true]:before:bg-primary data-[dropend=true]:after:absolute data-[dropend=true]:after:inset-y-0 data-[dropend=true]:after:right-0 data-[dropend=true]:after:w-0.5 data-[dropend=true]:after:bg-primary",
            onmounted: move |event| {
                th_refs.write().insert(register_key.clone(), event.data());
            },
            onpointerenter: move |_| on_activate_column.call(mount_key.clone()),
            onpointerleave: move |_| on_deactivate_column.call(()),
            div {
                class: cn!("flex h-full items-center", align_flex(column.align)),
                style: format!("height:{row_height}px;{}", css(&cell)),
                if reorderable {
                    button {
                        r#type: "button",
                        aria_label: "Reorder {reorder_key} column",
                        class: cn!(
                            "flex h-full w-6 cursor-grab touch-none items-center justify-center text-muted-foreground/60 transition-colors hover:text-foreground active:cursor-grabbing",
                            TOUCH_GESTURE_CLASS,
                        ),
                        onpointerdown: move |event: Event<PointerData>| {
                            event.prevent_default();
                            event.stop_propagation();
                            platform::capture_pointer(&event);
                            on_reorder_start.call((reorder_key.clone(), event.client_coordinates().x));
                        },
                        onpointermove: move |event: Event<PointerData>| {
                            on_reorder_move.call(event.client_coordinates().x);
                        },
                        onpointerup: move |event: Event<PointerData>| {
                            platform::release_pointer(&event);
                            on_reorder_end.call(());
                        },
                        GripVertical { class: "h-3.5 w-3.5" }
                    }
                }
                if column.sortable {
                    button {
                        r#type: "button",
                        class: cn!(
                            "flex h-full min-w-0 flex-1 select-none items-center gap-1 px-4 transition-colors hover:text-foreground",
                            align_flex(column.align),
                            active.then_some("text-foreground"),
                        ),
                        onclick: move |_| on_toggle_sort.call(sort_key.clone()),
                        span { class: "truncate", {column.header.clone()} }
                        span {
                            aria_hidden: "true",
                            class: "inline-flex shrink-0",
                            style: css(&chevron),
                            IconChevronUp { class: "h-3.5 w-3.5" }
                        }
                    }
                } else if let Some(rename_callback) = on_column_rename {
                    input {
                        value: column.header_text.clone().unwrap_or_default(),
                        aria_label: "Rename {rename_key} column",
                        size: "1",
                        class: cn!(
                            "min-w-0 flex-1 truncate appearance-none rounded-md border-0 bg-transparent px-4 font-medium text-muted-foreground outline-none transition-colors focus:bg-muted focus:text-foreground",
                            align_text(column.align),
                        ),
                        oninput: move |event| rename_callback.call((rename_key.clone(), event.value())),
                    }
                } else {
                    span {
                        class: cn!("min-w-0 flex-1 truncate px-4", align_text(column.align)),
                        {column.header.clone()}
                    }
                }
            }
            if resizable {
                button {
                    r#type: "button",
                    aria_label: "Resize {resize_key} column",
                    tabindex: "-1",
                    class: cn!(
                        "absolute top-0 right-0 h-full w-1.5 cursor-col-resize touch-none bg-transparent transition-colors hover:bg-muted",
                        TOUCH_GESTURE_CLASS,
                    ),
                    onpointerdown: move |event: Event<PointerData>| {
                        event.prevent_default();
                        event.stop_propagation();
                        platform::capture_pointer(&event);
                        on_resize_start.call((resize_key.clone(), event.client_coordinates().x));
                    },
                    onpointermove: move |event: Event<PointerData>| {
                        on_resize_move.call(event.client_coordinates().x);
                    },
                    onpointerup: move |event: Event<PointerData>| {
                        platform::release_pointer(&event);
                        on_resize_end.call(());
                    },
                }
            }
        }
    }
}

// --- editable cell ----------------------------------------------------------

#[component]
fn EditableCell(value: String, label: String, on_change: EventHandler<String>) -> Element {
    rsx! {
        input {
            value: "{value}",
            aria_label: "{label}",
            size: "1",
            placeholder: "Empty",
            class: "-mx-2 w-full min-w-0 appearance-none rounded-md border-0 bg-transparent px-2 py-1 text-foreground outline-none transition-colors placeholder:text-muted-foreground/40 focus:bg-muted focus:ring-1 focus:ring-ring",
            oninput: move |event| on_change.call(event.value()),
        }
    }
}

// --- skeleton rows ----------------------------------------------------------

#[component]
fn SkeletonRows(
    count: usize,
    columns: Vec<TableColumn>,
    selectable: bool,
    row_height: f64,
) -> Element {
    rsx! {
        for row_index in 0..count {
            tr {
                key: "{row_index}",
                style: "height:{row_height}px",
                class: "border-border/60 border-b",
                if selectable {
                    td {}
                }
                for column in columns.iter() {
                    td {
                        key: "{column.key}",
                        class: cn!("px-4", align_text(column.align)),
                        div {
                            class: cn!(
                                "h-3 animate-pulse rounded-full bg-muted",
                                if column.align == TableAlign::Right { "ml-auto w-10" } else { "w-2/3" },
                            ),
                        }
                    }
                }
                td { aria_hidden: "true" }
            }
        }
    }
}

// --- row handle -------------------------------------------------------------

#[component]
fn RowHandle(
    id: String,
    index: usize,
    x: f64,
    y: f64,
    on_insert_row: Option<EventHandler<(usize, InsertPosition)>>,
    on_delete_row: Option<EventHandler<(String, usize)>>,
    on_enter: EventHandler<()>,
    on_leave: EventHandler<()>,
) -> Element {
    let mut items = Vec::new();
    if let Some(callback) = on_insert_row {
        items.push(
            TableMenuItem::new(
                "Insert before",
                EventHandler::new(move |_| callback.call((index, InsertPosition::Before))),
            )
            .icon(rsx! { ArrowUpToLine {} }),
        );
        items.push(
            TableMenuItem::new(
                "Insert after",
                EventHandler::new(move |_| callback.call((index, InsertPosition::After))),
            )
            .icon(rsx! { ArrowDownToLine {} }),
        );
    }
    if let Some(callback) = on_delete_row {
        let row_id = id.clone();
        items.push(
            TableMenuItem::new(
                "Delete row",
                EventHandler::new(move |_| callback.call((row_id.clone(), index))),
            )
            .icon(rsx! { Trash2 {} })
            .destructive(),
        );
    }

    rsx! {
        div {
            style: "position:fixed;top:{y}px;left:{x}px;transform:translate(-50%,-50%);z-index:40",
            onpointerenter: move |_| on_enter.call(()),
            onpointerleave: move |_| on_leave.call(()),
            TableMenu {
                items,
                aria_label: "Row {index + 1} options",
                trigger_class: "flex h-6 w-2 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm transition-colors hover:bg-primary/90",
                trigger: rsx! { MoreVertical { class: "h-3 w-3" } },
            }
        }
    }
}

// --- column handle ----------------------------------------------------------

#[component]
fn ColumnHandle(
    column_key: String,
    index: usize,
    x: f64,
    y: f64,
    on_insert_column: Option<EventHandler<(usize, InsertPosition)>>,
    on_delete_column: Option<EventHandler<(String, usize)>>,
    on_enter: EventHandler<()>,
    on_leave: EventHandler<()>,
) -> Element {
    let mut items = Vec::new();
    if let Some(callback) = on_insert_column {
        items.push(
            TableMenuItem::new(
                "Insert before",
                EventHandler::new(move |_| callback.call((index, InsertPosition::Before))),
            )
            .icon(rsx! { ArrowLeftToLine {} }),
        );
        items.push(
            TableMenuItem::new(
                "Insert after",
                EventHandler::new(move |_| callback.call((index, InsertPosition::After))),
            )
            .icon(rsx! { ArrowRightToLine {} }),
        );
    }
    if let Some(callback) = on_delete_column {
        let key = column_key.clone();
        items.push(
            TableMenuItem::new(
                "Delete column",
                EventHandler::new(move |_| callback.call((key.clone(), index))),
            )
            .icon(rsx! { Trash2 {} })
            .destructive(),
        );
    }

    rsx! {
        div {
            style: "position:fixed;top:{y}px;left:{x}px;transform:translate(-50%,-50%);z-index:40",
            onpointerenter: move |_| on_enter.call(()),
            onpointerleave: move |_| on_leave.call(()),
            TableMenu {
                items,
                aria_label: "{column_key} column options",
                trigger_class: "flex h-2 w-6 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm transition-colors hover:bg-primary/90",
                trigger: rsx! { MoreHorizontal { class: "h-3 w-3" } },
            }
        }
    }
}

// --- menu -------------------------------------------------------------------

#[component]
fn TableMenu(
    items: Vec<TableMenuItem>,
    aria_label: String,
    trigger: Element,
    trigger_class: String,
) -> Element {
    let reduce = use_reduced_motion()();
    let mut coords = use_signal(|| None::<(f64, f64)>);
    let mut trigger_node: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut panel = use_number_motion(0.0);
    let open = coords.read().is_some();

    use_effect(use_reactive!(|open, reduce| {
        let target = if open { 1.0f32 } else { 0.0f32 };
        let config = if open && !reduce {
            spring(SPRING_PANEL)
        } else {
            tween(0, Ease::Out)
        };
        let _ = panel.animate_to(target, config);
    }));

    let close = EventHandler::new(move |_: ()| coords.set(None));

    rsx! {
        button {
            r#type: "button",
            aria_label: "{aria_label}",
            aria_haspopup: "menu",
            aria_expanded: if open { "true" } else { "false" },
            class: "{trigger_class}",
            onmounted: move |event| {
                trigger_node.set(Some(event.data()));
            },
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::Escape {
                    close.call(());
                }
            },
            onclick: move |event: MouseEvent| {
                event.stop_propagation();
                if coords.read().is_some() {
                    coords.set(None);
                    return;
                }
                let Some(node) = trigger_node.read().clone() else {
                    return;
                };
                spawn(async move {
                    if let Some(rect) = platform::client_rect(&node).await {
                        coords.set(Some((
                            rect.top + rect.height + 4.0,
                            (rect.left + rect.width - MENU_WIDTH).max(8.0),
                        )));
                    }
                });
            },
            {trigger}
        }
        if let Some((top, left)) = *coords.read() {
            div {
                class: "fixed inset-0 z-40",
                onpointerdown: move |_| close.call(()),
            }
            div {
                role: "menu",
                class: "fixed z-50 overflow-hidden rounded-xl border border-border bg-background p-1 shadow-xl",
                style: {
                    let p = progress(&panel);
                    let (scale, y) = if reduce {
                        (1.0, 0.0)
                    } else {
                        (0.96 + 0.04 * p, -4.0 + 4.0 * p)
                    };
                    format!(
                        "top:{top}px;left:{left}px;width:{MENU_WIDTH}px;opacity:{p};transform:translateY({y}px) scale({scale})"
                    )
                },
                onkeydown: move |event: KeyboardEvent| {
                    if event.key() == Key::Escape {
                        close.call(());
                    }
                },
                for item in items.iter() {
                    button {
                        key: "{item.label}",
                        r#type: "button",
                        role: "menuitem",
                        class: cn!(
                            "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left text-sm transition-colors [&_svg]:h-4 [&_svg]:w-4",
                            if item.destructive {
                                "text-rose-500 hover:bg-rose-500/10"
                            } else {
                                "text-foreground hover:bg-muted"
                            },
                        ),
                        onclick: {
                            let select = item.on_select;
                            move |_| {
                                close.call(());
                                select.call(());
                            }
                        },
                        if let Some(icon) = item.icon.clone() {
                            {icon}
                        }
                        "{item.label}"
                    }
                }
            }
        }
    }
}

//! `motion/table`, `motion/table-editable`, `motion/table-async` — the three
//! beUI table previews.
//!
//! Data is deterministic so the server and the client render the same rows (the
//! source is explicit about that): `10,000 rows` in the data preview, page
//! loading in the async one, and the editable one drives every callback the
//! table exposes.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use beui::prelude::*;
use beui::{locale_integer, InsertPosition, SortDirection, SortState, TableRows};
use dioxus_motion::prelude::{Duration, Time, TimeProvider};

// --- shared data ------------------------------------------------------------

const FIRST: [&str; 10] = [
    "Ava", "Leo", "Mia", "Kai", "Zoe", "Eli", "Noa", "Ren", "Ivy", "Jude",
];
const LAST: [&str; 10] = [
    "Cole", "Frost", "Vale", "Reyes", "Okafor", "Sato", "Lund", "Marsh", "Bose", "Quinn",
];
const ROLES: [&str; 4] = ["Owner", "Admin", "Member", "Viewer"];
const STATUSES: [&str; 3] = ["active", "invited", "suspended"];

/// `STATUS_STYLES`.
fn status_badge(status: &str) -> Element {
    let class = match status {
        "active" => "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
        "invited" => "bg-amber-500/10 text-amber-600 dark:text-amber-400",
        _ => "bg-rose-500/10 text-rose-600 dark:text-rose-400",
    };
    rsx! {
        span { class: cn!("rounded-full px-2 py-0.5 font-medium text-xs capitalize", class), "{status}" }
    }
}

/// `buildPeople` — one deterministic person at index `n`.
fn person_row(n: usize) -> TableRow {
    let first = FIRST[n % FIRST.len()];
    let last = LAST[(n * 7) % LAST.len()];
    let name = format!("{first} {last}");
    let email = format!(
        "{}.{}{}@beui.dev",
        first.to_lowercase(),
        last.to_lowercase(),
        n
    );
    let role = ROLES[(n * 3) % ROLES.len()];
    let status = STATUSES[(n * 5) % STATUSES.len()];
    let mrr = 12 + ((n * 37) % 488);
    let name_cell = name.clone();
    let mrr_cell = mrr;
    TableRow::new(n.to_string())
        .cell(
            "name",
            rsx! { span { class: "font-medium", "{name_cell}" } },
        )
        .text("email", email)
        .text("role", role)
        .cell("status", status_badge(status))
        .cell(
            "mrr",
            rsx! { span { class: "tabular-nums", "${mrr_cell}" } },
        )
        .sort("name", name)
        .sort("role", role)
        .sort("mrr", mrr as f64)
}

/// The five person columns, shared by the data and async previews.
fn person_columns() -> Vec<TableColumn> {
    vec![
        TableColumn::text("name", "Name").sortable().width("1.4fr"),
        TableColumn::text("email", "Email").width("1.8fr"),
        TableColumn::text("role", "Role").sortable().width("120px"),
        TableColumn::text("status", "Status").width("130px"),
        TableColumn::text("mrr", "MRR")
            .sortable()
            .align(TableAlign::Right)
            .width("110px"),
    ]
}

/// The page's own header block.
fn page_header(kicker: &str, title: &str, description: &str) -> Element {
    rsx! {
        header { class: "flex flex-col gap-3",
            p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground", "{kicker}" }
            h1 { class: "text-balance text-3xl font-semibold tracking-tight", "{title}" }
            p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground", "{description}" }
        }
    }
}

// --- data preview -----------------------------------------------------------

/// `table.preview.tsx` — 10k virtualized rows.
#[component]
pub fn TablePage() -> Element {
    let data = use_hook(|| TableRows::new((0..10_000).map(person_row).collect()));
    let mut selected = use_signal(Vec::<String>::new);
    let rows = data.len();
    let selected_count = selected().len();

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            {page_header(
                "Motion · table",
                "Table",
                "Virtualized data table that stays smooth at 10k+ rows, with sortable headers, row selection, column resize and reorder, and a sticky header.",
            )}
            div { class: "flex w-full justify-center p-4",
                div { class: "flex w-full flex-col gap-2",
                    div { class: "flex items-center justify-between px-1 text-muted-foreground text-xs",
                        span { "{locale_integer(rows as f64)} rows" }
                        if selected_count > 0 {
                            span { "{locale_integer(selected_count as f64)} selected" }
                        }
                    }
                    Table {
                        data: data.clone(),
                        columns: person_columns(),
                        selectable: true,
                        resizable: true,
                        reorderable: true,
                        selected_row_ids: Some(selected()),
                        on_selection_change: move |ids| selected.set(ids),
                        default_sort: Some(SortState::new("mrr", SortDirection::Desc)),
                        height: 420.0,
                        row_height: 52.0,
                        class: "rounded-2xl",
                    }
                }
            }
        }
    }
}

// --- editable preview -------------------------------------------------------

#[derive(Clone, PartialEq)]
struct EditableRow {
    id: String,
    fields: BTreeMap<String, String>,
}

fn fields(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn initial_rows() -> Vec<EditableRow> {
    vec![
        EditableRow {
            id: "r1".into(),
            fields: fields(&[("name", "Ava Cole"), ("role", "Owner"), ("team", "Design")]),
        },
        EditableRow {
            id: "r2".into(),
            fields: fields(&[("name", "Leo Frost"), ("role", "Admin"), ("team", "Growth")]),
        },
        EditableRow {
            id: "r3".into(),
            fields: fields(&[("name", "Mia Vale"), ("role", "Member"), ("team", "Design")]),
        },
        EditableRow {
            id: "r4".into(),
            fields: fields(&[
                ("name", "Kai Reyes"),
                ("role", "Member"),
                ("team", "Platform"),
            ]),
        },
    ]
}

fn initial_labels() -> BTreeMap<String, String> {
    [("name", "Name"), ("role", "Role"), ("team", "Team")]
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

/// `table-editable.preview.tsx` — insert/delete rows and columns, edit cells.
#[component]
pub fn TableEditablePage() -> Element {
    let mut rows = use_signal(initial_rows);
    let mut keys = use_signal(|| vec!["name".to_string(), "role".to_string(), "team".to_string()]);
    let mut labels = use_signal(initial_labels);
    let mut editable = use_signal(|| true);
    let next_row = use_hook(|| Rc::new(Cell::new(5u32)));
    let next_col = use_hook(|| Rc::new(Cell::new(1u32)));

    let on_cell_edit = EventHandler::new(move |(row_id, key, value): (String, String, String)| {
        let mut next = rows();
        for row in next.iter_mut() {
            if row.id == row_id {
                row.fields.insert(key.clone(), value.clone());
            }
        }
        rows.set(next);
    });

    let insert_row = {
        let counter = next_row.clone();
        EventHandler::new(move |(index, position): (usize, InsertPosition)| {
            let at = if position == InsertPosition::After {
                index + 1
            } else {
                index
            };
            let id = format!("r{}", counter.get());
            counter.set(counter.get() + 1);
            let mut next = rows();
            let at = at.min(next.len());
            next.insert(
                at,
                EditableRow {
                    id,
                    fields: BTreeMap::new(),
                },
            );
            rows.set(next);
        })
    };

    let delete_row = EventHandler::new(move |(row_id, _index): (String, usize)| {
        rows.set(rows().into_iter().filter(|row| row.id != row_id).collect());
    });

    let insert_column = {
        let counter = next_col.clone();
        EventHandler::new(move |(index, position): (usize, InsertPosition)| {
            let number = counter.get();
            counter.set(number + 1);
            let key = format!("field{number}");
            let at = if position == InsertPosition::After {
                index + 1
            } else {
                index
            };
            labels
                .write()
                .insert(key.clone(), format!("Field {number}"));
            let mut next_keys = keys();
            let at = at.min(next_keys.len());
            next_keys.insert(at, key.clone());
            keys.set(next_keys);
            let mut next = rows();
            for row in next.iter_mut() {
                row.fields.insert(key.clone(), String::new());
            }
            rows.set(next);
        })
    };

    let rename_column = EventHandler::new(move |(key, value): (String, String)| {
        labels.write().insert(key, value);
    });

    let delete_column = EventHandler::new(move |(key, _index): (String, usize)| {
        keys.write().retain(|current| current != &key);
        let mut next = rows();
        for row in next.iter_mut() {
            row.fields.remove(&key);
        }
        rows.set(next);
    });

    let editable_now = editable();
    let columns: Vec<TableColumn> = keys()
        .iter()
        .enumerate()
        .map(|(index, key)| {
            let label = labels().get(key).cloned().unwrap_or_else(|| key.clone());
            let mut column = TableColumn::text(key.clone(), label);
            if editable_now {
                column = column.editable();
            }
            if index != 0 {
                column = column.width("180px");
            }
            column
        })
        .collect();

    let data = TableRows::new(
        rows()
            .iter()
            .map(|row| {
                let mut entry = TableRow::new(row.id.clone());
                for (key, value) in row.fields.iter() {
                    entry = entry.text(key.clone(), value.clone());
                }
                entry
            })
            .collect(),
    );
    let body_height = rows().len().clamp(1, 6) as f64 * 48.0;

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            {page_header(
                "Motion · table",
                "Editable Table",
                "Edit cells inline and insert or delete rows and columns via border handles; the table re-renders from the updated data and column defs.",
            )}
            div { class: "flex w-full flex-col gap-3 p-4",
                div { class: "flex items-center justify-between",
                    p { class: "text-muted-foreground text-xs",
                        if editable_now {
                            "Click a cell to edit. Use the column and row handles to insert or delete."
                        } else {
                            "Read-only."
                        }
                    }
                    Switch {
                        checked: editable_now,
                        on_checked_change: move |next| editable.set(next),
                        label: "Editable",
                    }
                }
                Table {
                    data,
                    columns,
                    row_height: 48.0,
                    height: body_height,
                    on_cell_edit: editable_now.then_some(on_cell_edit),
                    on_column_rename: editable_now.then_some(rename_column),
                    on_insert_row: editable_now.then_some(insert_row),
                    on_delete_row: editable_now.then_some(delete_row),
                    on_insert_column: editable_now.then_some(insert_column),
                    on_delete_column: editable_now.then_some(delete_column),
                    empty_state: Some(rsx! {
                        button {
                            r#type: "button",
                            class: "rounded-full border border-border px-3 py-1.5 font-medium text-foreground text-xs transition-colors hover:bg-muted",
                            onclick: move |_| insert_row.call((0, InsertPosition::Before)),
                            "Insert first row"
                        }
                    }),
                }
            }
        }
    }
}

// --- async preview ----------------------------------------------------------

const PAGE_SIZE: usize = 20;
const MAX_PAGES: usize = 8;

/// `table-async.preview.tsx` — skeleton first load, then infinite scroll.
#[component]
pub fn TableAsyncPage() -> Element {
    let mut rows = use_signal(Vec::<TableRow>::new);
    let mut loading = use_signal(|| true);
    let page_ref = use_hook(|| Rc::new(Cell::new(0usize)));
    let loading_ref = use_hook(|| Rc::new(Cell::new(false)));

    let load_more = {
        let page = page_ref.clone();
        let busy = loading_ref.clone();
        EventHandler::new(move |_: ()| {
            if busy.get() || page.get() >= MAX_PAGES {
                return;
            }
            busy.set(true);
            loading.set(true);
            let page = page.clone();
            let busy = busy.clone();
            spawn(async move {
                // `setTimeout(…, 700)`, through the animation clock rather than a
                // per-frame `document::eval`.
                let _ = Time::delay(Duration::from_millis(700)).await;
                let current = page.get();
                let mut next = rows();
                let start = current * PAGE_SIZE;
                for n in start..start + PAGE_SIZE {
                    next.push(person_row(n));
                }
                rows.set(next);
                page.set(current + 1);
                busy.set(false);
                loading.set(false);
            });
        })
    };

    use_effect(move || load_more.call(()));

    let loaded = rows().len();
    let loading_now = loading();
    let done = page_ref.get() >= MAX_PAGES;

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            {page_header(
                "Motion · table",
                "Async Table",
                "Loads pages on demand — skeleton rows on first load, then infinite scroll via onEndReached as the virtualized list nears the bottom.",
            )}
            div { class: "flex w-full justify-center p-4",
                div { class: "flex w-full flex-col gap-2",
                    div { class: "flex items-center justify-between px-1 text-muted-foreground text-xs",
                        span { "{locale_integer(loaded as f64)} loaded" }
                        span {
                            if loading_now {
                                "Loading…"
                            } else if done {
                                "All loaded"
                            } else {
                                "Scroll for more"
                            }
                        }
                    }
                    Table {
                        data: TableRows::new(rows()),
                        columns: person_columns(),
                        height: 420.0,
                        row_height: 52.0,
                        on_end_reached: Some(load_more),
                        loading: loading_now,
                        class: "rounded-2xl",
                    }
                }
            }
        }
    }
}

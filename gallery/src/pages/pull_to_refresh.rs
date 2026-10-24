//! `PullToRefresh` — the beUI preview: an activity feed inside a phone-shaped
//! card, refreshed by pulling it down.

use beui::icons::{Bell, CheckCircle2, PackageCheck, Sparkles};
use beui::prelude::*;
use beui::PullToRefresh;
use beui_core::platform;

#[derive(Clone, PartialEq)]
struct Update {
    id: u32,
    title: String,
    detail: String,
    time: String,
    // `GitPullRequest` / `MessageCircle` / `Bell` / `CheckCircle2` in the
    // source; the equivalent icons available in `beui::icons`.
    icon: u8,
}

fn initial_updates() -> Vec<Update> {
    [
        (
            "Motion review approved",
            "The pull request is ready to merge.",
            "4m",
            0u8,
        ),
        (
            "New component feedback",
            "The spring feels much closer to native now.",
            "18m",
            1,
        ),
        (
            "Registry checks passed",
            "All component files resolved successfully.",
            "31m",
            2,
        ),
        (
            "Preview deployed",
            "The latest build is ready to inspect.",
            "1h",
            3,
        ),
        (
            "Docs comment resolved",
            "The usage example now covers async refreshes.",
            "2h",
            1,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (title, detail, time, icon))| Update {
        id: index as u32 + 1,
        title: title.to_string(),
        detail: detail.to_string(),
        time: time.to_string(),
        icon,
    })
    .collect()
}

#[component]
pub fn PullToRefreshPage() -> Element {
    let mut refresh_count = use_signal(|| 0u32);
    let mut latest_update = use_signal(|| None::<u32>);
    let mut next_id = use_signal(|| 100u32);
    let mut refreshing = use_signal(|| false);

    let mut updates = initial_updates();
    if let Some(id) = latest_update() {
        updates.insert(
            0,
            Update {
                id,
                title: "You’re all caught up".to_string(),
                detail: "The activity feed was refreshed just now.".to_string(),
                time: "now".to_string(),
                icon: 3,
            },
        );
    }
    let badge = if refresh_count() > 0 {
        format!("{} refreshed", refresh_count())
    } else {
        "live".to_string()
    };

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "PullToRefresh" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Pull the top of a scroller down to refresh. The distance is resisted, the indicator lifts, tilts and stretches with the pull, and the content holds while an async refresh runs."
                }
            }
            div { class: "flex min-h-[34rem] w-full items-center justify-center p-4",
                div { class: "w-full max-w-sm overflow-hidden rounded-[2rem] border border-border bg-background shadow-2xl",
                    PullToRefresh {
                        aria_label: "Activity feed".to_string(),
                        class: Some("h-[30rem]".to_string()),
                        refreshing: refreshing(),
                        on_refresh: EventHandler::new(move |_| {
                            spawn(async move {
                                refreshing.set(true);
                                let _ = platform::json::<bool>(
                                    "await new Promise((resolve) => setTimeout(resolve, 900));\nreturn true;",
                                )
                                .await;
                                refresh_count += 1;
                                let id = next_id();
                                next_id.set(id + 1);
                                latest_update.set(Some(id));
                                refreshing.set(false);
                            });
                        }),
                        header { class: "sticky top-0 z-10 flex items-center justify-between border-b border-border bg-background/90 px-5 py-4 backdrop-blur-md",
                            div {
                                p { class: "font-semibold text-foreground", "Activity" }
                                p { class: "text-xs text-muted-foreground", "Pull down to check for updates" }
                            }
                            span { class: "rounded-full bg-muted px-2.5 py-1 font-mono text-[10px] text-muted-foreground", "{badge}" }
                        }
                        div { class: "divide-y divide-border px-2 pb-3",
                            for update in updates {
                                article { key: "{update.id}", class: "flex gap-3 px-3 py-4",
                                    div { class: "grid h-9 w-9 shrink-0 place-items-center rounded-xl border border-border bg-card text-muted-foreground shadow-sm",
                                        match update.icon {
                                            0 => rsx! { PackageCheck { class: Some("h-4 w-4".to_string()) } },
                                            1 => rsx! { Sparkles { class: Some("h-4 w-4".to_string()) } },
                                            2 => rsx! { CheckCircle2 { class: Some("h-4 w-4".to_string()) } },
                                            _ => rsx! { Bell { class: Some("h-4 w-4".to_string()) } },
                                        }
                                    }
                                    div { class: "min-w-0 flex-1",
                                        div { class: "flex items-start justify-between gap-3",
                                            p { class: "text-sm font-medium text-foreground", "{update.title}" }
                                            span { class: "shrink-0 font-mono text-[10px] text-muted-foreground", "{update.time}" }
                                        }
                                        p { class: "mt-1 text-xs leading-relaxed text-muted-foreground", "{update.detail}" }
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

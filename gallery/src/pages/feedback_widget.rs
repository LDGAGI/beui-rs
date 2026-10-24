//! `FeedbackWidget` — the beUI preview: a faux app surface with the corner
//! trigger. The first submission fails, the retry succeeds.

use beui::prelude::*;
use beui::FeedbackWidget;
use beui_core::platform;

#[component]
pub fn FeedbackWidgetPage() -> Element {
    let mut attempts = use_signal(|| 0u32);
    let mut pending = use_signal(|| false);
    let mut failed = use_signal(|| false);

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "FeedbackWidget" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A corner trigger that morphs into a panel. The shell leads the reveal with its own timing, the field is armed only once the open morph settles, and a successful send bursts sprinkles around a drawn tick."
                }
            }
            div { class: "relative flex h-80 w-full max-w-md overflow-hidden rounded-2xl border border-border bg-background",
                div { class: "border-b border-border px-5 py-3",
                    div { class: "h-2.5 w-24 rounded-full bg-muted-foreground/20" }
                }
                div { class: "flex flex-col gap-3 p-5",
                    div { class: "h-2.5 w-3/4 rounded-full bg-muted-foreground/15" }
                    div { class: "h-2.5 w-1/2 rounded-full bg-muted-foreground/15" }
                    div { class: "h-24 w-full rounded-xl bg-muted-foreground/[0.06]" }
                    div { class: "h-2.5 w-2/3 rounded-full bg-muted-foreground/15" }
                }
                FeedbackWidget {
                    error: failed(),
                    pending: pending(),
                    on_submit: Some(EventHandler::new(move |_message: String| {
                        // The preview's submit: 900ms, failing the first time.
                        pending.set(true);
                        failed.set(false);
                        spawn(async move {
                            let _ = platform::json::<bool>(
                                "await new Promise((resolve) => setTimeout(resolve, 900));\nreturn true;",
                            )
                            .await;
                            let next = attempts() + 1;
                            attempts.set(next);
                            pending.set(false);
                            if next == 1 {
                                failed.set(true);
                            }
                        });
                    })),
                }
            }
        }
    }
}

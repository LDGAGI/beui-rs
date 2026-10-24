//! `blocks/availability-scheduler.preview.tsx` — the default week.
//!
//! No props: the preview mounts the scheduler on its own `defaultWeek`
//! (Mon–Fri 9–5, weekends off) and lets every control be exercised in place.

use beui::prelude::*;

#[component]
pub fn AvailabilitySchedulerPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight",
                    "Availability Scheduler"
                }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Weekly availability editor where each day springs between available and unavailable, time ranges add and remove with blur-slide motion, times pick from a scrollable dropdown, and a copy menu clones hours to other days."
                }
            }
            div { class: "flex w-full justify-center",
                AvailabilityScheduler {}
            }
        }
    }
}

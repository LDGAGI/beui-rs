//! Radio, on screen — `radio.preview.tsx` and `radio-description.preview.tsx`.

use crate::pages::common::*;
use beui::prelude::*;

#[component]
pub fn RadioPage() -> Element {
    let mut plan = use_signal(|| "pro".to_string());
    let mut schedule = use_signal(|| "once".to_string());

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "Radio",
                lede: "One dot shared between items: when the selection moves it glides from the old item's box to the new one through the flip registry."
            }

            Section {
                title: "Group",
                note: "A controlled group; the dot is a layoutId.",
                Stage { class: "justify-start",
                    RadioGroup {
                        value: plan(),
                        on_value_change: move |next| plan.set(next),
                        class: "min-w-48",
                        RadioGroupItem { value: "starter", label: "Starter, free" }
                        RadioGroupItem { value: "pro", label: "Pro, $12/mo" }
                        RadioGroupItem { value: "team", label: "Team, $29/mo" }
                        RadioGroupItem { value: "legacy", label: "Legacy plan", disabled: true }
                    }
                }
            }

            Section {
                title: "With description",
                note: "The description is announced as the description, not folded into the name.",
                Stage { class: "justify-start",
                    RadioGroup {
                        value: schedule(),
                        on_value_change: move |next| schedule.set(next),
                        class: "w-full max-w-64",
                        RadioGroupItem { value: "once", label: "Run once", description: "Runs a single time, right now." }
                        RadioGroupItem { value: "schedule", label: "Run on a schedule", description: "Repeats on a set interval until stopped." }
                        RadioGroupItem { value: "trigger", label: "Run on a trigger", description: "Waits for an event before it starts." }
                        RadioGroupItem { value: "manual", label: "Manual approval", description: "Not available on your plan.", disabled: true }
                    }
                }
            }
        }
    }
}

//! Switch, on screen — `switch.preview.tsx`.

use crate::pages::common::*;
use beui::prelude::*;

#[component]
pub fn SwitchPage() -> Element {
    let mut on = use_signal(|| true);

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "Switch",
                lede: "A heavy spring {{800, 80, 4}} drives the thumb, which squishes while pressed and shakes if you press it disabled."
            }

            Section {
                title: "States",
                note: "on, off and disabled.",
                Stage { class: "justify-start",
                    div { class: "flex flex-col gap-3",
                        Switch { checked: on(), on_checked_change: move |next| on.set(next), label: "Enable notifications" }
                        Switch { checked: false, on_checked_change: move |_| {}, label: "Off" }
                        Switch { checked: true, disabled: true, on_checked_change: move |_| {}, label: "Disabled" }
                    }
                }
            }
        }
    }
}

//! Checkbox, on screen — `checkbox.preview.tsx` and
//! `checkbox-description.preview.tsx`.

use crate::pages::common::*;
use beui::prelude::*;

#[component]
pub fn CheckboxPage() -> Element {
    let mut terms = use_signal(|| true);
    let mut updates = use_signal(|| false);
    let mut comments = use_signal(|| true);
    let mut mentions = use_signal(|| true);
    let mut digest = use_signal(|| false);

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "Checkbox",
                lede: "A 0.92 press spring and a mark that fades, scales and draws its path — with an indeterminate state that rolls the glyph."
            }

            Section {
                title: "States",
                note: "checked, unchecked, indeterminate and disabled.",
                Stage { class: "justify-start",
                    div { class: "flex flex-col gap-3",
                        Checkbox { checked: terms(), on_checked_change: move |next| terms.set(next), label: "Accept terms and conditions" }
                        Checkbox { checked: updates(), on_checked_change: move |next| updates.set(next), label: "Email me product updates" }
                        Checkbox { checked: true, indeterminate: true, on_checked_change: move |_| {}, label: "Select all (partial)" }
                        Checkbox { checked: true, disabled: true, on_checked_change: move |_| {}, label: "Disabled" }
                    }
                }
            }

            Section {
                title: "With description",
                note: "The description is announced as the description, not folded into the name.",
                Stage { class: "justify-start",
                    div { class: "flex w-full max-w-64 flex-col gap-3",
                        Checkbox {
                            checked: comments(),
                            on_checked_change: move |next| comments.set(next),
                            label: "Comments",
                            description: "Replies on threads you started.",
                        }
                        Checkbox {
                            checked: mentions(),
                            on_checked_change: move |next| mentions.set(next),
                            label: "Mentions",
                            description: "Whenever someone tags you by name.",
                        }
                        Checkbox {
                            checked: digest(),
                            on_checked_change: move |next| digest.set(next),
                            label: "Weekly digest",
                            description: "A summary of activity every Monday.",
                        }
                        Checkbox {
                            checked: false,
                            disabled: true,
                            on_checked_change: move |_| {},
                            label: "SMS alerts",
                            description: "Add a phone number to enable.",
                        }
                    }
                }
            }
        }
    }
}

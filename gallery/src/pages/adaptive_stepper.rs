//! AdaptiveStepper, on screen — `adaptive-stepper.preview.tsx`.
//!
//! Start at 2 of 0–3: pushing to a bound disables that button and lets the
//! value field widen into its space.

use crate::pages::common::*;
use beui::prelude::*;

#[component]
pub fn AdaptiveStepperPage() -> Element {
    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "AdaptiveStepper",
                lede: "A stepper that re-lays itself as it reaches its bounds: the spent button drifts away and fades while the value field widens to take the space."
            }

            Section {
                title: "Guests",
                note: "0 to 3. The value rolls on a spring as it changes.",
                Stage { class: "min-h-80",
                    AdaptiveStepper { default_value: 2.0, min: 0.0, max: 3.0, aria_label: "Guests",
                        AdaptiveStepperDecrement {}
                        AdaptiveStepperValue {}
                        AdaptiveStepperIncrement {}
                    }
                }
            }
        }
    }
}

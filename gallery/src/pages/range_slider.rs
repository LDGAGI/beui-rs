//! The RangeSlider family, on screen — `range-slider*.preview.tsx`.
//!
//! Six designs over one value model: the base handle, the inline label slider,
//! the thumbless fluid pill, the equalizer wave, the velocity-leaning bubble and
//! the scrolling ruler.

use crate::pages::common::*;
use beui::prelude::*;

#[component]
pub fn RangeSliderPage() -> Element {
    let mut base = use_signal(|| 40.0);
    let mut fluid = use_signal(|| 35.0);
    let mut wave = use_signal(|| 45.0);
    let mut bubble = use_signal(|| 28.0);
    let mut ruler = use_signal(|| 72.5);

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "RangeSlider",
                lede: "One value model, six designs. The value, snapping, pointer capture, keyboard and ARIA all come from the shared slider hook; only the geometry changes."
            }

            Section {
                title: "Base",
                note: "A 4px handle over a rounded track, with a spring-smoothed fill and tick dots.",
                Stage { class: "justify-start",
                    div { class: "flex w-full max-w-sm flex-col gap-3",
                        div { class: "flex items-center justify-between text-sm text-muted-foreground",
                            span { "Drag the handle" }
                            span { class: "tabular-nums text-foreground", "{base()}" }
                        }
                        RangeSlider {
                            value: base(),
                            on_value_change: move |next| base.set(next),
                            step: 5.0,
                            aria_label: "Value",
                        }
                    }
                }
            }

            Section {
                title: "Inline",
                note: "An always-visible label and readout inside the track, with ten snap stops.",
                Stage { class: "justify-start",
                    div { class: "w-full max-w-sm",
                        InlineSlider {
                            label: "Icon size",
                            default_value: 48.0,
                            min: 8.0,
                            max: 128.0,
                            step: 8.0,
                            aria_label: "Icon size",
                            value_text: Callback::new(|v: f64| format!("{v} pixels")),
                        }
                    }
                }
            }

            Section {
                title: "Fluid",
                note: "Thumbless: the whole pill is the control, and the label inverts as the fill covers it.",
                Stage { class: "justify-start",
                    div { class: "w-full max-w-sm",
                        FluidSlider {
                            value: fluid(),
                            on_value_change: move |next| fluid.set(next),
                            label: "Brightness",
                            aria_label: "Brightness",
                        }
                    }
                }
            }

            Section {
                title: "Wave",
                note: "Bars rise into a crest around the handle and fall back as it passes.",
                Stage { class: "justify-start",
                    div { class: "flex w-full max-w-md flex-col gap-2",
                        div { class: "flex items-center justify-between text-sm text-muted-foreground",
                            span { "Gain" }
                            span { class: "tabular-nums text-foreground", "{wave()}" }
                        }
                        WaveSlider { value: wave(), on_value_change: move |next| wave.set(next), aria_label: "Gain" }
                    }
                }
            }

            Section {
                title: "Bubble",
                note: "A value bubble pops out on grab and leans into the direction of travel.",
                Stage { class: "justify-start",
                    div { class: "flex w-full max-w-sm flex-col gap-1",
                        span { class: "text-sm text-muted-foreground", "Drag fast and the bubble leans" }
                        BubbleSlider { value: bubble(), on_value_change: move |next| bubble.set(next), aria_label: "Value" }
                    }
                }
            }

            Section {
                title: "Ruler",
                note: "The scale scrolls under a fixed needle and snaps onto the nearest tick.",
                Stage { class: "justify-start",
                    div { class: "w-full max-w-sm",
                        RulerSlider {
                            value: ruler(),
                            on_value_change: move |next| ruler.set(next),
                            min: 40.0,
                            max: 120.0,
                            step: 0.5,
                            gap: 12.0,
                            major_every: 10.0,
                            unit: "kg",
                            aria_label: "Weight",
                        }
                    }
                }
            }
        }
    }
}

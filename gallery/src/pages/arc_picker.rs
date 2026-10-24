//! `arc-picker.preview.tsx` — a time-of-day picker whose curve side is chosen
//! above it, plus an uncontrolled instance so both value modes are observable.

use beui::prelude::*;

/// `SIDES` — the four curve directions.
const SIDES: [(&str, ArcPickerSide); 4] = [
    ("top", ArcPickerSide::Top),
    ("bottom", ArcPickerSide::Bottom),
    ("left", ArcPickerSide::Left),
    ("right", ArcPickerSide::Right),
];

/// `OPTIONS` — the preview's time-of-day list, verbatim.
const OPTIONS: [(&str, &str); 22] = [
    ("first-light", "First light"),
    ("dawn", "Dawn"),
    ("daybreak", "Daybreak"),
    ("sunrise", "Sunrise"),
    ("early-morning", "Early morning"),
    ("morning", "Morning"),
    ("late-morning", "Late morning"),
    ("midday", "Midday"),
    ("noon", "Noon"),
    ("afternoon", "Afternoon"),
    ("golden-hour", "Golden hour"),
    ("sunset", "Sunset"),
    ("blue-hour", "Blue hour"),
    ("dusk", "Dusk"),
    ("twilight", "Twilight"),
    ("evening", "Evening"),
    ("nightfall", "Nightfall"),
    ("night", "Night"),
    ("late-night", "Late night"),
    ("midnight", "Midnight"),
    ("deep-night", "Deep night"),
    ("starlight", "Starlight"),
];

fn options() -> Vec<ArcPickerOption> {
    OPTIONS
        .iter()
        .map(|(value, label)| ArcPickerOption::new(*value, *label))
        .collect()
}

#[component]
pub fn ArcPickerPage() -> Element {
    let mut value = use_signal(|| "golden-hour".to_string());
    let mut side = use_signal(|| ArcPickerSide::Right);
    let mut uncontrolled = use_signal(|| "midday".to_string());
    let items = use_hook(options);

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-14 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · arc-picker"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Arc Picker" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A radial picker whose labels ride a curve: drag, scroll or use the arrow keys, and the selection frame morphs to the choice under the center."
                }
            }

            section { class: "w-full max-w-xl",
                fieldset { class: "flex flex-wrap items-center justify-center gap-1 px-4",
                    legend { class: "sr-only", "Curve side" }
                    for (name, direction) in SIDES {
                        button {
                            key: "{name}",
                            r#type: "button",
                            aria_pressed: if side() == direction { "true" } else { "false" },
                            class: cn!(
                                "press inline-flex h-8 items-center rounded-lg px-3 text-sm capitalize transition-colors",
                                if side() == direction {
                                    "bg-secondary text-secondary-foreground"
                                } else {
                                    "text-muted-foreground hover:bg-muted hover:text-foreground"
                                },
                            ),
                            onclick: move |_| side.set(direction),
                            "{name}"
                        }
                    }
                }
                div { class: "my-8 flex min-h-108 items-center sm:my-10",
                    ArcPicker {
                        options: items.clone(),
                        value: Some(value()),
                        on_value_change: EventHandler::new(move |next| value.set(next)),
                        aria_label: "Time of day",
                        side: side(),
                        radius: 260.0,
                        visible_count: 9,
                        item_height: 48.0,
                    }
                }
                p { class: "px-6 text-center text-xs text-muted-foreground",
                    "Drag "
                    {if matches!(side(), ArcPickerSide::Top | ArcPickerSide::Bottom) { "horizontally" } else { "vertically" }}
                    ", scroll or use the arrow keys."
                }
                p { class: "mt-2 px-6 text-center text-xs text-muted-foreground",
                    "Controlled value: "
                    span { "data-beui-arc-value": "controlled", class: "font-medium text-foreground", "{value()}" }
                }
            }

            section { class: "w-full max-w-xl",
                h2 { class: "mb-1 text-lg font-medium tracking-tight", "Uncontrolled" }
                p { class: "mb-6 text-sm text-muted-foreground",
                    "With only a " code { "defaultValue" } ", the picker owns its own state."
                }
                div { class: "flex min-h-108 items-center",
                    ArcPicker {
                        options: items,
                        default_value: Some("midday".to_string()),
                        on_value_change: EventHandler::new(move |next| uncontrolled.set(next)),
                        aria_label: "Uncontrolled time of day",
                        side: ArcPickerSide::Right,
                        radius: 260.0,
                        visible_count: 9,
                        item_height: 48.0,
                    }
                }
                p { class: "mt-2 text-center text-xs text-muted-foreground",
                    "Last published: "
                    span { "data-beui-arc-value": "uncontrolled", class: "font-medium text-foreground", "{uncontrolled()}" }
                }
            }
        }
    }
}

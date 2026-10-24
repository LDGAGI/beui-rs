//! `Magnetic` — no beUI preview of its own (the button variants wrap it), so
//! this page shows the raw component with the three strengths beUI uses.

use beui::prelude::*;
use beui::Magnetic;

#[component]
pub fn MagneticPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · pointer"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Magnetic" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A wrapper whose contents lean toward the cursor on two SPRING_MOUSE axes. Skipped on touch and under reduced motion."
                }
            }
            div { class: "flex flex-wrap items-center justify-center gap-4",
                Magnetic { strength: 0.35,
                    button { r#type: "button", class: "press inline-flex h-10 items-center rounded-full bg-primary px-4 text-sm font-medium text-primary-foreground",
                        "Hover me"
                    }
                }
                Magnetic { strength: 0.25,
                    button { r#type: "button", class: "press inline-flex h-10 items-center rounded-full border border-border bg-card px-4 text-sm font-medium text-foreground",
                        "Subtle pull"
                    }
                }
                Magnetic { strength: 0.5,
                    button { r#type: "button", class: "press inline-flex h-10 items-center rounded-full border border-border px-4 text-sm font-medium text-foreground",
                        "Strong pull"
                    }
                }
            }
        }
    }
}

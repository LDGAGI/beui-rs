//! `popover.preview.tsx` — a click popover with a form and a hover one.

use beui::prelude::*;
use beui::{
    Popover, PopoverAlign, PopoverContent, PopoverSide, PopoverTrigger, PopoverTriggerMode,
};

#[component]
pub fn PopoverPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Popover" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "The clip is what moves, not the panel: the trigger rect and the panel rect sit in one box, and both the goo body and the content are clipped to the rounded rectangle between them. A blur-and-sharpen filter over that body welds the two into one liquid shape while the neck is short."
                }
            }
            Preview {}
        }
    }
}

#[component]
fn Preview() -> Element {
    let width = use_signal(|| "100%".to_string());
    let height = use_signal(|| "auto".to_string());
    let mut set_width = width;
    let mut set_height = height;

    rsx! {
        section { class: "flex flex-col gap-4",
            h2 { class: "text-lg font-semibold tracking-tight", "Popover" }
            p { class: "max-w-3xl text-sm text-muted-foreground",
                "Click opens the first, hover opens the second; both animate the goo, and each clips the panel to the same morph."
            }
            div { class: "flex flex-wrap items-center justify-center gap-4 py-8",
                Popover { side: PopoverSide::Bottom, align: PopoverAlign::Start,
                    PopoverTrigger { class: "inline-flex h-9 items-center rounded-lg bg-secondary px-4 text-sm font-medium text-secondary-foreground",
                        "Edit profile"
                    }
                    PopoverContent { class: "w-72",
                        p { class: "text-sm font-medium text-foreground", "Dimensions" }
                        p { class: "mt-1 text-xs text-muted-foreground",
                            "Set the width and height for the layer."
                        }
                        div { class: "mt-3 flex flex-col gap-2",
                            label { class: "flex items-center justify-between gap-3 text-sm",
                                span { class: "text-muted-foreground", "Width" }
                                input {
                                    value: "{width}",
                                    oninput: move |event| set_width.set(event.value()),
                                    class: "h-8 w-32 rounded-lg border border-border bg-background px-2.5 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-foreground/20",
                                }
                            }
                            label { class: "flex items-center justify-between gap-3 text-sm",
                                span { class: "text-muted-foreground", "Height" }
                                input {
                                    value: "{height}",
                                    oninput: move |event| set_height.set(event.value()),
                                    class: "h-8 w-32 rounded-lg border border-border bg-background px-2.5 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-foreground/20",
                                }
                            }
                        }
                    }
                }
                Popover { trigger: PopoverTriggerMode::Hover, side: PopoverSide::Top,
                    PopoverTrigger { class: "inline-flex h-9 items-center rounded-lg border border-border px-4 text-sm font-medium text-foreground",
                        "Hover me"
                    }
                    PopoverContent { class: "w-56",
                        p { class: "text-sm text-foreground",
                            "Opens on hover, with a grace window so you can move into the panel."
                        }
                    }
                }
            }
        }
    }
}

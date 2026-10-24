//! `BloomMenu` — the beUI preview, same copy and structure.

use beui::prelude::*;
use beui::BloomMenu;

#[component]
pub fn BloomMenuPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Motion · block"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "BloomMenu" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A trigger that blooms into a panel. Trigger and panel share one layoutId (beui-core's flip registry); the content plays an iris reveal and the items stagger radially by distance from the centre."
                }
            }
            div { class: "flex min-h-[420px] w-full items-start justify-center pt-24",
                BloomMenu {}
            }
        }
    }
}

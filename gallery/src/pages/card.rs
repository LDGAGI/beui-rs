//! Card, on screen — `card.preview.tsx`.
//!
//! Every card shape: a selectable document card (the checkbox stretched over
//! the whole card with `CARD_TARGET_CLASS`), a muted settings card, an order
//! summary with a footer, and an image card whose image runs to the top edge.

use crate::pages::common::*;
use beui::prelude::*;

const DOCUMENTS: [(&str, &str, &str); 2] = [
    (
        "contract",
        "Signed contract",
        "The final PDF with every signature and the audit trail.",
    ),
    (
        "invoice",
        "Invoice",
        "Itemised billing for the period, ready for accounting.",
    ),
];

const SUMMARY: [(&str, &str); 3] = [
    ("Plan", "Team, yearly"),
    ("Seats", "12"),
    ("Billing email", "billing@example.com"),
];

#[component]
pub fn CardPage() -> Element {
    let mut notifications = use_signal(|| true);
    let mut digest = use_signal(|| false);

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "Card",
                lede: "A surface with two variants and two sizes. An interactive card springs to 0.985 on press, driven by hand so it never becomes an unnamed tab stop."
            }

            div { class: "grid w-full max-w-3xl items-start gap-4 md:grid-cols-2",
                div { class: "flex flex-col gap-3",
                    for (id, title, description) in DOCUMENTS {
                        DocumentChoice { key: "{id}", id, title, description }
                    }

                    Card { variant: CardVariant::Muted, size: CardSize::Compact,
                        CardHeader {
                            CardTitle { as_: CardTitleAs::H3, "Notifications" }
                            CardDescription { "Choose what reaches your inbox." }
                        }
                        CardContent { class: "flex flex-col gap-3",
                            div { class: "flex items-center justify-between gap-4",
                                span { "Mentions and replies" }
                                Switch {
                                    checked: notifications(),
                                    on_checked_change: move |next| notifications.set(next),
                                    aria_label: "Mentions and replies",
                                }
                            }
                            div { class: "flex items-center justify-between gap-4",
                                span { "Weekly digest" }
                                Switch {
                                    checked: digest(),
                                    on_checked_change: move |next| digest.set(next),
                                    aria_label: "Weekly digest",
                                }
                            }
                        }
                    }
                }

                div { class: "flex flex-col gap-4",
                    Card {
                        CardHeader {
                            CardTitle { as_: CardTitleAs::H3, "Review order" }
                            CardDescription { "Check the details before you confirm." }
                            CardAction {
                                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, "Edit" }
                            }
                        }
                        CardContent {
                            dl { class: "divide-y divide-border",
                                for (label, value) in SUMMARY {
                                    div { key: "{label}", class: "flex items-center justify-between gap-4 py-2.5 first:pt-0 last:pb-0",
                                        dt { class: "text-muted-foreground", "{label}" }
                                        dd { class: "font-medium text-foreground", "{value}" }
                                    }
                                }
                            }
                        }
                        CardFooter { class: "justify-end",
                            Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, "Back" }
                            Button { size: ButtonSize::Sm, "Confirm order" }
                        }
                    }

                    Card { class: "overflow-hidden pt-0",
                        img {
                            src: "https://images.unsplash.com/photo-1497366754035-f200968a6e72?auto=format&fit=crop&w=720&q=80",
                            alt: "Bright open-plan studio with long desks and plants",
                            width: "720",
                            height: "405",
                            loading: "lazy",
                            class: "aspect-video w-full object-cover",
                        }
                        CardHeader {
                            CardTitle { as_: CardTitleAs::H3, "Studio, Lisbon" }
                            CardDescription { "Desks for 12 people, booked for the March offsite." }
                        }
                        CardFooter { class: "justify-between",
                            span { class: "text-sm text-muted-foreground",
                                span { class: "font-medium text-foreground", "€1,840" }
                                " · 3 days"
                            }
                            Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, "View booking" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DocumentChoice(id: String, title: String, description: String) -> Element {
    let mut checked = use_signal(|| id == "contract");
    let description_id = use_hook(|| format!("beui-card-{}-description", id));

    rsx! {
        Card { size: CardSize::Compact, interactive: true, selected: checked(),
            CardHeader {
                CardTitle { "{title}" }
                CardDescription { id: description_id.clone(), "{description}" }
                CardAction {
                    Checkbox {
                        checked: checked(),
                        on_checked_change: move |next| checked.set(next),
                        aria_label: title.clone(),
                        aria_describedby: description_id.clone(),
                        class: CARD_TARGET_CLASS,
                    }
                }
            }
        }
    }
}

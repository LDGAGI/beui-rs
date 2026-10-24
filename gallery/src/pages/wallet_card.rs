//! `WalletCard` — the beUI preview: three accounts, a change pill and a
//! simulated balance move. The source's avatars come from DiceBear; this passes
//! initials so the page stays offline.

use beui::prelude::*;
use beui::{WalletAccount, WalletCard};

fn accounts() -> Vec<WalletAccount> {
    [
        (
            "main",
            "Main Wallet",
            "0x8f3Cb1a29e4D7c6F1B2a3E9d0C4b5A6f7D8e9C0b",
        ),
        (
            "trading",
            "Trading",
            "0x1a2B3c4D5e6F7a8B9c0D1e2F3a4B5c6D7e8F9a0B",
        ),
        (
            "cold",
            "Cold Storage",
            "0x9F8e7D6c5B4a3E2d1C0b9A8f7E6d5C4b3A2e1F0d",
        ),
    ]
    .into_iter()
    .map(|(id, name, address)| WalletAccount::new(id, name, address).avatar(initials(name, id)))
    .collect()
}

fn initials(name: &str, id: &str) -> Element {
    let letters: String = name
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_uppercase();
    let tone = match id {
        "main" => "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
        "trading" => "bg-sky-500/15 text-sky-600 dark:text-sky-400",
        _ => "bg-violet-500/15 text-violet-600 dark:text-violet-400",
    };
    rsx! {
        span { class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold {tone}",
            "{letters}"
        }
    }
}

#[component]
pub fn WalletCardPage() -> Element {
    let mut balance = use_signal(|| 12480.32f64);
    let mut step = use_signal(|| 0u32);

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "WalletCard" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A composed wallet overview: the account trigger morphs into a full-width panel over the header icons, the search icon becomes a bar, and the balance swaps for asterisks on one baseline with a per-letter cascade."
                }
            }
            div { class: "flex w-full flex-col items-center gap-4 p-6",
                WalletCard {
                    accounts: accounts(),
                    balance: balance(),
                    default_change: Some(124.5),
                    search_recent: vec![
                        "vitalik.eth".to_string(),
                        "0xA0b8…6EB4".to_string(),
                        "Uniswap".to_string(),
                        "Send to Trading".to_string(),
                    ],
                    has_notifications: true,
                    on_send: Some(EventHandler::new(move |_| step.set(1))),
                    on_deposit: Some(EventHandler::new(move |_| step.set(2))),
                }
                button {
                    r#type: "button",
                    class: "h-9 rounded-full border border-border px-4 text-sm font-medium text-foreground transition-colors hover:bg-muted",
                    onclick: move |_| {
                        let next = balance() + if balance() > 12000.0 { -186.4 } else { 124.5 };
                        balance.set(next);
                    },
                    "Simulate balance change"
                }
                p { class: "text-xs text-muted-foreground",
                    "Action fired: " span { class: "font-mono text-foreground",
                        match step() { 0 => "none", 1 => "send", _ => "deposit" }
                    }
                }
            }
        }
    }
}

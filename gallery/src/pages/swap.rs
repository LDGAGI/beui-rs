//! `MultiChainSwap` — the beUI preview: the block with its own sample chains and
//! tokens (the source ships them in `swap/data.ts`).

use beui::prelude::*;
use beui::{Chain, MultiChainSwap, Token};

fn chains() -> Vec<Chain> {
    vec![
        Chain::new("eth", "Ethereum", "Ξ", "bg-primary text-primary-foreground"),
        Chain::new(
            "sol",
            "Solana",
            "◎",
            "bg-secondary text-secondary-foreground",
        ),
        Chain::new("base", "Base", "B", "bg-accent text-accent-foreground"),
        Chain::new("arb", "Arbitrum", "A", "bg-muted text-muted-foreground"),
    ]
}

fn tokens() -> Vec<Token> {
    vec![
        Token::new("eth-eth", "ETH", "Ether", "eth")
            .balance(1.245)
            .usd(3142.0)
            .popular(),
        Token::new("eth-weth", "WETH", "Wrapped Ether", "eth")
            .balance(0.5)
            .usd(3142.0)
            .popular(),
        Token::new("eth-usdc", "USDC", "USD Coin", "eth")
            .balance(4521.0)
            .usd(1.0)
            .popular()
            .address("0xA0b8…6EB4"),
        Token::new("eth-dai", "DAI", "Dai Stablecoin", "eth")
            .balance(800.0)
            .usd(1.0),
        Token::new("sol-sol", "SOL", "Solana", "sol")
            .balance(24.8)
            .usd(148.0)
            .popular(),
        Token::new("sol-usdc", "USDC", "USD Coin", "sol")
            .balance(1200.0)
            .usd(1.0),
        Token::new("base-eth", "ETH", "Ether", "base")
            .balance(0.31)
            .usd(3142.0),
        Token::new("arb-arb", "ARB", "Arbitrum", "arb")
            .balance(1450.0)
            .usd(1.12),
    ]
}

#[component]
pub fn SwapPage() -> Element {
    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "MultiChainSwap" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "A swap form across chains. The rate comes from the two tokens' USD prices, the quote flickers for 450ms after any change, the flip button spins 180° and exchanges the tokens, and the picker is a bottom sheet with chain filters."
                }
            }
            div { class: "flex w-full items-center justify-center",
                MultiChainSwap { chains: Some(chains()), tokens: Some(tokens()) }
            }
        }
    }
}

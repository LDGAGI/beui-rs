//! `motion/swap/` — the multi-chain swap block: two token fields, a quote row, a
//! destination row and a token-picker sheet.
//!
//! The source spreads this over nine files; the port keeps it in one. Everything
//! that fixes behaviour is the source's: `isValidAddress` (a 40-hex `0x` or an
//! `.eth` / `.sol` / `.bnb` name longer than five characters), `truncateAddress`,
//! `sanitizeAmount` (digits and one dot) and `formatAmount` (two decimals from
//! 1000 up, otherwise the requested precision); the derived rates
//! (`from.usd / to.usd`) and `toAmount = amount * rate`; the 450ms quoting
//! window; the flip that adds 180° to the rotation and exchanges the two tokens;
//! the picker's rule that choosing the token already on the other side swaps
//! them; every label ("You pay" / "You get", "Enter an amount", the
//! insufficient-balance and "Swap + Send to …" variants) and every spring
//! (`{380, 26, 0.6}` on the flip, `SPRING_PANEL` on the sheet, `SPRING_PRESS` on
//! the action).
//!
//! Differences from the source: the destination row's height animates
//! through the CSS `grid-template-rows` 0fr/1fr trick rather than motion's
//! `height: auto`, and the `AnimatePresence` fades of the quoting spinner, the
//! action label and the destination icons are keyed CSS animations that play on
//! entry only.

use crate::icons::{
    ArrowDownUp, Check, ChevronDown, LoaderCircle, Search, Send, Settings, Wallet, X,
};
use crate::prelude::*;
use beui_core::platform::{js_string, json, next_scope_id};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

const SPRING_FLIP: Spring = Spring {
    stiffness: 380.0,
    damping: 26.0,
    mass: 0.6,
};

const ANIMATIONS: &str = "\
@keyframes beui-swap-fade{0%{opacity:0}100%{opacity:1}}\
@keyframes beui-swap-pop{0%{opacity:0;transform:scale(0.85)}100%{opacity:1;transform:scale(1)}}";

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// A chain the tokens live on.
#[derive(Clone, Debug, PartialEq)]
pub struct Chain {
    pub id: String,
    pub name: String,
    /// Tailwind classes for the dot, e.g. `bg-sky-500 text-white`.
    pub tone: String,
    pub symbol: String,
}

impl Chain {
    /// A chain with its dot tone.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        symbol: impl Into<String>,
        tone: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            symbol: symbol.into(),
            tone: tone.into(),
        }
    }
}

/// A token, with the numbers the fields show.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub chain_id: String,
    pub address: Option<String>,
    pub balance: Option<f64>,
    pub usd: Option<f64>,
    pub trending: bool,
    pub popular: bool,
}

impl Token {
    /// A token on a chain.
    pub fn new(
        id: impl Into<String>,
        symbol: impl Into<String>,
        name: impl Into<String>,
        chain_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            symbol: symbol.into(),
            name: name.into(),
            chain_id: chain_id.into(),
            address: None,
            balance: None,
            usd: None,
            trending: false,
            popular: false,
        }
    }

    /// Set the wallet balance.
    pub fn balance(mut self, balance: f64) -> Self {
        self.balance = Some(balance);
        self
    }

    /// Set the USD price.
    pub fn usd(mut self, usd: f64) -> Self {
        self.usd = Some(usd);
        self
    }

    /// Set the contract address, shown in the picker.
    pub fn address(mut self, address: impl Into<String>) -> Self {
        self.address = Some(address.into());
        self
    }

    /// Mark it as popular.
    pub fn popular(mut self) -> Self {
        self.popular = true;
        self
    }
}

/// Which side of the form a token sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenSide {
    From,
    To,
}

/// `isValidAddress`.
pub fn is_valid_address(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() == 42 && value.starts_with("0x") {
        return value[2..].chars().all(|c| c.is_ascii_hexdigit());
    }
    (value.ends_with(".eth") || value.ends_with(".sol") || value.ends_with(".bnb"))
        && value.len() > 5
}

/// `truncateAddress`.
pub fn truncate_address(value: &str) -> String {
    if value.starts_with("0x") && value.len() == 42 {
        format!("{}...{}", &value[..6], &value[value.len() - 4..])
    } else {
        value.to_string()
    }
}

/// `sanitizeAmount`: digits and one dot.
pub fn sanitize_amount(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|character| character.is_ascii_digit() || *character == '.')
        .collect();
    let mut parts = cleaned.splitn(2, '.');
    let whole = parts.next().unwrap_or("").to_string();
    match parts.next() {
        None => whole,
        Some(decimal) => format!(
            "{whole}.{}",
            decimal
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
        ),
    }
}

/// `formatAmount`: two decimals from 1000 up, otherwise `max`.
pub fn format_amount(value: f64, max: usize) -> String {
    if !value.is_finite() || value == 0.0 {
        return "0".to_string();
    }
    let digits = if value >= 1000.0 { 2 } else { max };
    let negative = value < 0.0;
    let scale = 10f64.powi(digits as i32);
    let scaled = (value.abs() * scale).round();
    let whole = (scaled / scale).trunc() as u64;
    let fraction = if digits > 0 {
        let raw = format!(
            "{:0width$}",
            (scaled as u64) % (scale as u64),
            width = digits
        );
        let trimmed = raw.trim_end_matches('0');
        if trimmed.is_empty() {
            String::new()
        } else {
            format!(".{trimmed}")
        }
    } else {
        String::new()
    };
    let text = whole.to_string();
    let mut grouped = String::new();
    for (index, character) in text.chars().enumerate() {
        if index > 0 && (text.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    format!("{}{grouped}{fraction}", if negative { "-" } else { "" })
}

/// A chain's dot.
#[component]
fn ChainDot(chain: Chain, #[props(default = 16.0)] size: f64) -> Element {
    rsx! {
        span {
            class: cn!("inline-flex shrink-0 items-center justify-center rounded-full text-[9px] font-bold", chain.tone),
            style: "width:{size}px;height:{size}px",
            "{chain.symbol}"
        }
    }
}

/// A token's dot with its chain badge.
#[component]
fn TokenDot(token: Token, chain: Chain, #[props(default = 28.0)] size: f64) -> Element {
    let badge = size * 0.42;
    let initials: String = token.symbol.chars().take(2).collect();
    rsx! {
        span { class: "relative inline-flex shrink-0", style: "width:{size}px;height:{size}px",
            span { class: "absolute inset-0 inline-flex items-center justify-center rounded-full border border-border bg-background text-[11px] font-bold text-foreground",
                "{initials}"
            }
            span {
                class: cn!("absolute -bottom-0.5 -right-0.5 inline-flex items-center justify-center rounded-full border-2 border-card text-[7px] font-bold", chain.tone),
                style: "width:{badge}px;height:{badge}px",
                "{chain.symbol}"
            }
        }
    }
}

/// A chain filter chip.
#[component]
fn ChainChip(
    active: bool,
    #[props(default)] chain: Option<Chain>,
    #[props(default)] label: Option<String>,
    on_click: EventHandler<()>,
) -> Element {
    let class = cn!(
        "inline-flex h-9 shrink-0 items-center justify-center rounded-xl border transition-colors active:scale-[0.97]",
        if chain.is_some() { "w-9" } else { "px-3" },
        if active {
            "border-primary/20 bg-muted/60 text-foreground"
        } else {
            "border-border/60 bg-background/40 text-foreground hover:border-border"
        },
    );
    let title = chain
        .as_ref()
        .map(|chain| chain.name.clone())
        .or(label.clone())
        .unwrap_or_default();

    rsx! {
        button {
            r#type: "button",
            aria_pressed: if active { "true" } else { "false" },
            class,
            title,
            onclick: move |_| on_click.call(()),
            if let Some(chain) = chain {
                ChainDot { chain, size: 20.0 }
            } else {
                span { class: "text-xs font-semibold", {label.unwrap_or_default()} }
            }
        }
    }
}

/// Props for [`SwapField`].
#[derive(Props, Clone, PartialEq)]
pub struct SwapFieldProps {
    pub side: TokenSide,
    pub token: Token,
    pub chain: Chain,
    pub amount: String,
    #[props(default)]
    pub on_amount: Option<EventHandler<String>>,
    #[props(default)]
    pub editable: bool,
    #[props(default)]
    pub quoting: bool,
    pub on_open_picker: EventHandler<()>,
}

/// One side of the form.
#[component]
pub fn SwapField(props: SwapFieldProps) -> Element {
    let SwapFieldProps {
        side,
        token,
        chain,
        amount,
        on_amount,
        editable,
        quoting,
        on_open_picker,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let usd_value = amount.parse::<f64>().unwrap_or(0.0) * token.usd.unwrap_or(0.0);
    let label = if side == TokenSide::From {
        "You pay"
    } else {
        "You get"
    };
    let spinner_key = "quote-spinner";
    let shown = if amount.is_empty() {
        "0".to_string()
    } else {
        amount.clone()
    };
    let quote_style = if reduce_value {
        String::new()
    } else {
        format!(
            "opacity:{};filter:blur({}px);transition:opacity 180ms ease-out, filter 180ms ease-out",
            if quoting { 0.55 } else { 1.0 },
            if quoting { 2.0 } else { 0.0 }
        )
    };
    let buton_class = "group inline-flex h-10 items-center gap-2 rounded-full border border-border bg-card pl-1 pr-2.5 text-sm font-semibold text-foreground transition-transform hover:border-border active:scale-[0.97]";

    rsx! {
        div { class: "relative rounded-2xl border border-border/50 bg-background/40 p-3.5",
            label { class: "mb-2 block text-[11px] font-medium uppercase tracking-wider text-muted-foreground",
                "{label}"
            }
            div { class: "flex items-center justify-between gap-3",
                div { class: "min-w-0 flex-1",
                    if editable {
                        input {
                            inputmode: "decimal",
                            value: "{amount}",
                            placeholder: "0",
                            class: "w-full bg-transparent text-2xl font-semibold tracking-tight text-foreground tabular-nums outline-none placeholder:text-muted-foreground/60",
                            oninput: move |event: Event<FormData>| {
                                if let Some(handler) = on_amount {
                                    handler.call(sanitize_amount(&event.value()));
                                }
                            },
                        }
                    } else {
                        div { class: "flex h-9 items-center gap-2 text-2xl font-semibold tracking-tight tabular-nums",
                            span { class: "text-foreground", style: quote_style, "{shown}" }
                            if quoting {
                                span {
                                    key: "{spinner_key}",
                                    class: "inline-flex",
                                    style: if reduce_value { String::new() } else { "animation:beui-swap-pop 140ms ease-out".to_string() },
                                    LoaderCircle { class: Some("h-4 w-4 animate-spin text-muted-foreground".to_string()) }
                                }
                            }
                        }
                    }
                    p { class: "mt-1 text-[11px] text-muted-foreground tabular-nums",
                        "≈ ${format_amount(usd_value, 2)}"
                    }
                }
                button {
                    r#type: "button",
                    class: buton_class,
                    onclick: move |_| on_open_picker.call(()),
                    TokenDot { token: token.clone(), chain: chain.clone() }
                    span { "{token.symbol}" }
                    ChevronDown { class: Some("h-3.5 w-3.5 text-muted-foreground".to_string()) }
                }
            }
            div { class: "mt-2 flex items-center justify-between text-[11px] text-muted-foreground",
                span { class: "inline-flex items-center gap-1",
                    Wallet { class: Some("h-3 w-3".to_string()) }
                    span { class: "tabular-nums",
                        if token.balance.is_some() {
                            "{format_amount(token.balance.unwrap_or(0.0), 6)}"
                        } else {
                            "0.00"
                        }
                    }
                    span { "· {chain.name}" }
                }
                if side == TokenSide::From && token.balance.is_some() {
                    button {
                        r#type: "button",
                        class: "rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                        onclick: {
                            let balance = token.balance.unwrap_or(0.0);
                            move |_| {
                                if let Some(handler) = on_amount {
                                    handler.call(format_amount(balance, 6));
                                }
                            }
                        },
                        "Max"
                    }
                }
            }
        }
    }
}

/// The rate / fee / slippage / ETA grid.
#[component]
pub fn SwapQuoteRow(
    from: Token,
    to: Token,
    rate: f64,
    fee: f64,
    slippage: f64,
    eta: String,
    quoting: bool,
) -> Element {
    rsx! {
        div { class: "mt-3 grid grid-cols-2 gap-x-4 gap-y-1.5 rounded-xl border border-border/50 bg-background/40 px-3.5 py-2.5 text-[11px]",
            span { class: "text-muted-foreground", "Rate" }
            span { class: "text-right tabular-nums text-foreground",
                if quoting {
                    LoaderCircle { class: Some("ml-auto inline h-3 w-3 animate-spin text-muted-foreground".to_string()) }
                } else {
                    "1 {from.symbol} ≈ {format_amount(rate, 6)} {to.symbol}"
                }
            }
            span { class: "text-muted-foreground", "Network fee" }
            span { class: "text-right tabular-nums text-foreground", "${fee:.2}" }
            span { class: "text-muted-foreground", "Slippage" }
            span { class: "text-right tabular-nums text-foreground", "{slippage:.2}%" }
            span { class: "text-muted-foreground", "ETA" }
            span { class: "text-right text-foreground", "{eta}" }
        }
    }
}

/// The button between the two fields.
#[component]
fn FlipButton(rotation: f64, reduce: bool, on_click: EventHandler<()>) -> Element {
    let mut rotate = use_motion(rotation as f32).expect("rotation is finite");
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    use_effect(use_reactive((&rotation,), move |(rotation,)| {
        let _ = rotate.animate_to(rotation as f32, AnimationConfig::spring(SPRING_FLIP));
    }));

    rsx! {
        div { class: "relative -my-4 flex justify-center", style: "z-index:1",
            button {
                r#type: "button",
                aria_label: "Reverse direction",
                class: "inline-flex h-9 w-9 items-center justify-center rounded-full border-[3px] border-card bg-muted text-foreground backdrop-blur",
                style: format!(
                    "transform:rotate({}deg) scale({})",
                    current(rotate),
                    if reduce { 1.0 } else { f64::from(current(scale)) }
                ),
                onclick: move |_| on_click.call(()),
                onpointerdown: move |_| {
                    if !reduce {
                        let _ = scale.animate_to(0.9, AnimationConfig::spring(SPRING_PRESS));
                    }
                },
                onpointerup: move |_| {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                },
                onpointercancel: move |_| {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                },
                ArrowDownUp { class: Some("h-3.5 w-3.5".to_string()) }
            }
        }
    }
}

/// The destination-address disclosure.
#[component]
fn DestinationRow(
    show: bool,
    address: String,
    reduce: bool,
    on_toggle: EventHandler<()>,
    on_address: EventHandler<String>,
) -> Element {
    let scope = use_hook(next_scope_id);
    let has_address = !address.is_empty();
    let valid = is_valid_address(&address);
    let summary = if show && has_address && valid {
        format!("To: {}", truncate_address(&address))
    } else {
        "Send to different address".to_string()
    };
    // 0fr → 1fr is the CSS way to animate to an intrinsic height.
    let body_style = format!(
        "display:grid;grid-template-rows:{};transition:{};overflow:hidden",
        if show { "1fr" } else { "0fr" },
        if reduce {
            "none".to_string()
        } else {
            "grid-template-rows 220ms cubic-bezier(0.16, 1, 0.3, 1)".to_string()
        }
    );
    let invalid_class = if has_address && !valid {
        "border-destructive/40"
    } else {
        "border-border"
    };
    let check_key = "dest-check";
    let clear_key = "dest-clear";

    rsx! {
        div { class: "mt-1 overflow-hidden rounded-xl border border-border/50 bg-background/40",
            button {
                r#type: "button",
                class: "flex w-full items-center justify-between gap-2 px-3.5 py-2.5 text-[12px] text-muted-foreground hover:text-foreground",
                onclick: move |_| on_toggle.call(()),
                span { class: "flex items-center gap-2",
                    Send { class: Some("h-3.5 w-3.5 shrink-0".to_string()) }
                    span { "{summary}" }
                }
                span { class: "inline-flex", style: format!(
                    "transform:rotate({}deg);transition:transform 200ms ease-out",
                    if show { 180 } else { 0 }
                ),
                    ChevronDown { class: Some("h-3.5 w-3.5".to_string()) }
                }
            }
            div {
                style: body_style,
                onmounted: {
                    let selector = format!("[data-beui-swap-dest=\"{scope}\"]");
                    move |_| {
                        let selector = selector.clone();
                        spawn(async move {
                            let script = format!(
                                "await new Promise((resolve) => requestAnimationFrame(resolve));\n\
                                 const el = document.querySelector({});\n\
                                 if (el) el.focus({{ preventScroll: true }});\n\
                                 return true;",
                                js_string(&selector)
                            );
                            let _ = json::<bool>(&script).await;
                        });
                    }
                },
                div { class: "min-h-0",
                    div { class: "border-t border-border/50 px-3.5 pb-3 pt-2.5",
                        div { class: cn!("flex items-center gap-2 rounded-lg border px-2.5 py-2 transition-colors", invalid_class),
                            input {
                                "data-beui-swap-dest": "{scope}",
                                value: "{address}",
                                placeholder: "0x... or name.eth",
                                spellcheck: "false",
                                class: "min-w-0 flex-1 bg-transparent font-mono text-[12px] text-foreground outline-none placeholder:text-muted-foreground/60",
                                oninput: move |event: Event<FormData>| {
                                    on_address.call(event.value().trim().to_string());
                                },
                            }
                            if has_address {
                                if valid {
                                    span {
                                        key: "{check_key}",
                                        class: "inline-flex",
                                        style: if reduce { String::new() } else { "animation:beui-swap-pop 140ms ease-out".to_string() },
                                        Check { class: Some("h-3.5 w-3.5 shrink-0 text-primary".to_string()) }
                                    }
                                } else {
                                    button {
                                        key: "{clear_key}",
                                        r#type: "button",
                                        aria_label: "Clear address",
                                        class: "shrink-0 text-muted-foreground hover:text-foreground",
                                        style: if reduce { String::new() } else { "animation:beui-swap-pop 140ms ease-out".to_string() },
                                        onclick: move |_| on_address.call(String::new()),
                                        X { class: Some("h-3.5 w-3.5".to_string()) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The primary action.
#[component]
fn SwapActionButton(from: Token, to: Token, amount: f64, dest_address: String) -> Element {
    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    let no_amount = amount <= 0.0;
    let over_balance = from.balance.is_some_and(|balance| amount > balance);
    let valid_dest = !dest_address.is_empty() && is_valid_address(&dest_address);
    let label = if no_amount {
        "Enter an amount".to_string()
    } else if over_balance {
        format!("Insufficient {}", from.symbol)
    } else if valid_dest {
        format!("Swap + Send to {}", truncate_address(&dest_address))
    } else {
        format!("Swap {} → {}", from.symbol, to.symbol)
    };
    let disabled = no_amount || over_balance;
    let class = cn!(
        "mt-3 inline-flex h-12 w-full items-center justify-center rounded-2xl text-sm font-semibold transition-colors",
        if disabled {
            "cursor-not-allowed bg-muted text-muted-foreground"
        } else {
            "bg-primary text-primary-foreground hover:bg-primary/90"
        },
    );

    rsx! {
        button {
            r#type: "button",
            disabled,
            class,
            style: if reduce_value { String::new() } else { format!("transform:scale({})", current(scale)) },
            onpointerdown: move |_| {
                if !disabled && !reduce_value {
                    let _ = scale.animate_to(0.97, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
            },
            onpointercancel: move |_| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
            },
            span {
                key: "{label}",
                class: "inline-flex",
                style: if reduce_value { String::new() } else { "animation:beui-swap-fade 140ms ease-out".to_string() },
                "{label}"
            }
        }
    }
}

/// The bottom-sheet token picker.
#[component]
fn TokenPicker(
    open: bool,
    side: TokenSide,
    chains: Vec<Chain>,
    tokens: Vec<Token>,
    selected_id: String,
    reduce: bool,
    on_pick: EventHandler<String>,
    on_close: EventHandler<()>,
) -> Element {
    let scope = use_hook(next_scope_id);
    let mut chain_filter = use_signal(|| "all".to_string());
    let mut query = use_signal(String::new);
    let mut rendered = use_signal(|| open);

    use_effect(use_reactive((&open,), move |(open,)| {
        if open {
            rendered.set(true);
        } else {
            spawn(async move {
                let _ = json::<bool>(
                    "await new Promise((resolve) => setTimeout(resolve, 420));\nreturn true;",
                )
                .await;
                if !open {
                    rendered.set(false);
                }
            });
        }
    }));
    use_effect(use_reactive((&open,), move |(open,)| {
        if open {
            query.set(String::new());
            chain_filter.set("all".to_string());
        }
    }));

    if !rendered() {
        return rsx! {};
    }

    let needle = query().trim().to_lowercase();
    let filtered: Vec<Token> = tokens
        .iter()
        .filter(|token| {
            if chain_filter() != "all" && token.chain_id != chain_filter() {
                return false;
            }
            if needle.is_empty() {
                return true;
            }
            let chain_name = chains
                .iter()
                .find(|chain| chain.id == token.chain_id)
                .map(|chain| chain.name.clone())
                .unwrap_or_default();
            [
                token.symbol.clone(),
                token.name.clone(),
                chain_name,
                token.address.clone().unwrap_or_default(),
            ]
            .iter()
            .any(|field| field.to_lowercase().contains(&needle))
        })
        .cloned()
        .collect();
    let popular: Vec<Token> = tokens
        .iter()
        .filter(|token| token.popular)
        .take(6)
        .cloned()
        .collect();
    let chain_of = |id: &str| chains.iter().find(|chain| chain.id == id).cloned();
    let sheet_style = if reduce {
        format!(
            "opacity:{};transition:opacity 180ms ease-out",
            if open { 1.0 } else { 0.0 }
        )
    } else {
        format!(
            "transform:translateY({});opacity:{}",
            if open { "0%" } else { "100%" },
            if open { 1.0 } else { 0.0 }
        )
    };
    let backdrop_style = format!(
        "opacity:{};transition:opacity 200ms ease-out",
        if open { 1.0 } else { 0.0 }
    );
    let section_label = if !needle.is_empty() {
        "Results"
    } else if chain_filter() == "all" {
        "Trending"
    } else {
        "Tokens"
    };
    let picker_selector = format!("[data-beui-swap-search=\"{scope}\"]");
    let on_close_handler = on_close;

    rsx! {
        button {
            r#type: "button",
            aria_label: "Close",
            tabindex: "-1",
            class: "absolute inset-0 z-10 cursor-default bg-background/40 backdrop-blur-sm",
            style: backdrop_style,
            onclick: move |_| on_close_handler.call(()),
        }
        div {
            role: "dialog",
            aria_modal: "true",
            aria_label: if side == TokenSide::From { "Select from token" } else { "Select to token" },
            class: "absolute inset-x-0 bottom-0 z-20 flex max-h-[92%] flex-col rounded-t-3xl border-t border-border bg-card shadow-2xl",
            style: sheet_style,
            onmounted: move |_| {
                let selector = picker_selector.clone();
                spawn(async move {
                    let script = format!(
                        "await new Promise((resolve) => requestAnimationFrame(resolve));\n\
                         const el = document.querySelector({});\n\
                         if (el) el.focus({{ preventScroll: true }});\n\
                         return true;",
                        js_string(&selector)
                    );
                    let _ = json::<bool>(&script).await;
                });
            },
            onkeydown: {
                move |event: Event<KeyboardData>| {
                    if event.key() == Key::Escape {
                        on_close.call(());
                    }
                }
            },
            div { class: "flex justify-center pb-1 pt-2.5",
                span { class: "h-1 w-9 rounded-full bg-muted" }
            }
            div { class: "flex items-center gap-2 border-b border-border px-4 pb-3",
                Search { class: Some("h-4 w-4 shrink-0 text-muted-foreground".to_string()) }
                input {
                    "data-beui-swap-search": "{scope}",
                    value: "{query()}",
                    placeholder: "Search name or paste address",
                    class: "w-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground/70",
                    oninput: {
                        let mut query = query;
                        move |event: Event<FormData>| query.set(event.value())
                    },
                }
                button {
                    r#type: "button",
                    aria_label: "Close",
                    class: "inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                    onclick: {
                                move |_| on_close.call(())
                    },
                    X { class: Some("h-3.5 w-3.5".to_string()) }
                }
            }
            div { class: "flex items-center gap-1.5 overflow-x-auto border-b border-border px-3 py-5 [scrollbar-width:none]",
                ChainChip {
                    active: chain_filter() == "all",
                    label: Some("All".to_string()),
                    on_click: {
                        let mut chain_filter = chain_filter;
                        move |_| chain_filter.set("all".to_string())
                    },
                }
                for chain in chains.iter() {
                    ChainChip {
                        key: "{chain.id}",
                        active: chain_filter() == chain.id,
                        chain: Some(chain.clone()),
                        on_click: {
                            let mut chain_filter = chain_filter;
                            let id = chain.id.clone();
                            move |_| chain_filter.set(id.clone())
                        },
                    }
                }
            }
            div { class: "flex-1 overflow-y-auto px-3 pb-4 pt-3",
                if needle.is_empty() && chain_filter() == "all" {
                    p { class: "px-1 pb-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground",
                        "Most popular"
                    }
                    div { class: "flex items-center gap-1.5 overflow-x-auto pb-3 [scrollbar-width:none]",
                        for token in popular.iter() {
                            if let Some(chain) = chain_of(&token.chain_id) {
                                button {
                                    key: "{token.id}",
                                    r#type: "button",
                                    class: "inline-flex shrink-0 items-center gap-2 rounded-full border border-border bg-background/50 py-1 pl-1 pr-3 text-sm font-semibold text-foreground transition-transform hover:border-border active:scale-[0.97]",
                                    onclick: {
                                        let id = token.id.clone();
                                        move |_| on_pick.call(id.clone())
                                    },
                                    TokenDot { token: token.clone(), chain, size: 22.0 }
                                    "{token.symbol}"
                                }
                            }
                        }
                    }
                }
                p { class: "px-1 pb-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground",
                    "{section_label}"
                }
                ul { class: "flex flex-col gap-0.5",
                    if filtered.is_empty() {
                        li { class: "py-8 text-center text-xs text-muted-foreground", "No tokens found" }
                    }
                    for token in filtered.iter() {
                        if let Some(chain) = chain_of(&token.chain_id) {
                            li { key: "{token.id}",
                                button {
                                    r#type: "button",
                                    class: cn!(
                                        "flex w-full items-center justify-between rounded-xl px-2 py-2 text-left transition-colors active:scale-[0.97]",
                                        if token.id == selected_id { "bg-muted/60" } else { "hover:bg-muted/60" },
                                    ),
                                    onclick: {
                                        let id = token.id.clone();
                                        move |_| on_pick.call(id.clone())
                                    },
                                    span { class: "flex min-w-0 items-center gap-2.5",
                                        TokenDot { token: token.clone(), chain, size: 32.0 }
                                        span { class: "flex min-w-0 flex-col",
                                            span { class: "truncate text-sm font-semibold text-foreground", "{token.name}" }
                                            span { class: "truncate text-[11px] text-muted-foreground", "{token.symbol}" }
                                        }
                                    }
                                    span { class: "shrink-0 text-right text-[11px] tabular-nums text-muted-foreground",
                                        if let Some(address) = token.address.clone() {
                                            "{address}"
                                        } else if let Some(balance) = token.balance {
                                            "{format_amount(balance, 6)}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Props for [`MultiChainSwap`].
#[derive(Props, Clone, PartialEq)]
pub struct MultiChainSwapProps {
    #[props(default)]
    pub chains: Option<Vec<Chain>>,
    #[props(default)]
    pub tokens: Option<Vec<Token>>,
    #[props(default = "eth-eth".to_string())]
    pub default_from_id: String,
    #[props(default = "sol-sol".to_string())]
    pub default_to_id: String,
    #[props(default)]
    pub class: Option<String>,
}

/// The swap block.
#[component]
pub fn MultiChainSwap(props: MultiChainSwapProps) -> Element {
    let MultiChainSwapProps {
        chains,
        tokens,
        default_from_id,
        default_to_id,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let chains = chains.unwrap_or_default();
    let tokens = tokens.unwrap_or_default();

    let from_id = use_signal(|| default_from_id.clone());
    let to_id = use_signal(|| default_to_id.clone());
    let amount = use_signal(|| "1".to_string());
    let flip_rotation = use_signal(|| 0.0f64);
    let mut quoting = use_signal(|| false);
    let picking = use_signal(|| None::<TokenSide>);
    let show_dest = use_signal(|| false);
    let dest_address = use_signal(String::new);
    let mut quote_generation = use_signal(|| 0u32);

    let find_token = |id: &str| {
        tokens
            .iter()
            .find(|token| token.id == id)
            .cloned()
            .unwrap_or_else(|| Token::new(id, id, id, ""))
    };
    let find_chain = |id: &str| {
        chains
            .iter()
            .find(|chain| chain.id == id)
            .cloned()
            .unwrap_or_else(|| Chain::new(id, id, id, "bg-muted text-muted-foreground"))
    };

    let from = find_token(&from_id());
    let to = find_token(&to_id());
    let from_chain = find_chain(&from.chain_id);
    let to_chain = find_chain(&to.chain_id);

    let numeric_amount = amount().parse::<f64>().unwrap_or(0.0);
    let rate = match (from.usd, to.usd) {
        (Some(from_usd), Some(to_usd)) if to_usd != 0.0 => from_usd / to_usd,
        _ => 1.0,
    };
    let to_amount = numeric_amount * rate;

    // A 450ms quoting window after any change to the order.
    let from_key = from_id();
    let to_key = to_id();
    use_effect(use_reactive(
        (&numeric_amount, &from_key, &to_key),
        move |(numeric, _from, _to)| {
            if numeric == 0.0 {
                return;
            }
            quoting.set(true);
            // `peek`, not a tracked read: an effect that reads the signal it
            // writes re-triggers itself forever and the renderer dies.
            let run = *quote_generation.peek() + 1;
            quote_generation.set(run);
            spawn(async move {
                let _ = json::<bool>(
                    "await new Promise((resolve) => setTimeout(resolve, 450));\nreturn true;",
                )
                .await;
                if *quote_generation.peek() == run {
                    quoting.set(false);
                }
            });
        },
    ));

    let mut flip = {
        let mut from_id = from_id;
        let mut to_id = to_id;
        let mut flip_rotation = flip_rotation;
        let from_now = from_id();
        let to_now = to_id();
        move || {
            flip_rotation.set(flip_rotation() + 180.0);
            from_id.set(to_now.clone());
            to_id.set(from_now.clone());
        }
    };
    let mut pick_token = {
        let mut from_id = from_id;
        let mut to_id = to_id;
        let mut picking = picking;
        move |id: String| {
            let Some(side) = picking() else {
                return;
            };
            match side {
                TokenSide::From => {
                    if id == to_id() {
                        to_id.set(from_id());
                    }
                    from_id.set(id);
                }
                TokenSide::To => {
                    if id == from_id() {
                        from_id.set(to_id());
                    }
                    to_id.set(id);
                }
            }
            picking.set(None);
        }
    };

    let root_class = cn!(
        "relative isolate w-full max-w-[420px] overflow-hidden rounded-3xl border border-border/20 bg-card",
        class,
    );

    rsx! {
        style { dangerous_inner_html: ANIMATIONS }
        div { class: root_class,
            div { class: "flex h-12 items-center justify-between border-b border-border/50 px-3",
                span { class: "px-2 text-sm font-semibold tracking-tight text-foreground", "Swap" }
                button {
                    r#type: "button",
                    aria_label: "Settings",
                    class: "inline-flex h-8 w-8 items-center justify-center rounded-full text-muted-foreground transition-transform hover:bg-muted/60 hover:text-foreground active:scale-[0.97]",
                    Settings { class: Some("h-4 w-4".to_string()) }
                }
            }
            div { class: "flex flex-col gap-1.5 p-4",
                SwapField {
                    side: TokenSide::From,
                    token: from.clone(),
                    chain: from_chain.clone(),
                    amount: amount(),
                    on_amount: Some(EventHandler::new({
                        let mut amount = amount;
                        move |next: String| amount.set(next)
                    })),
                    editable: true,
                    quoting: false,
                    on_open_picker: EventHandler::new({
                        let mut picking = picking;
                        move |_| picking.set(Some(TokenSide::From))
                    }),
                }
                FlipButton {
                    rotation: flip_rotation(),
                    reduce: reduce_value,
                    on_click: EventHandler::new(move |_| flip()),
                }
                SwapField {
                    side: TokenSide::To,
                    token: to.clone(),
                    chain: to_chain.clone(),
                    amount: if to_amount > 0.0 { format_amount(to_amount, 6) } else { String::new() },
                    editable: false,
                    quoting: quoting(),
                    on_open_picker: EventHandler::new({
                        let mut picking = picking;
                        move |_| picking.set(Some(TokenSide::To))
                    }),
                }
                SwapQuoteRow {
                    from: from.clone(),
                    to: to.clone(),
                    rate,
                    fee: 0.42,
                    slippage: 0.5,
                    eta: "≈ 24s".to_string(),
                    quoting: quoting(),
                }
                DestinationRow {
                    show: show_dest(),
                    address: dest_address(),
                    reduce: reduce_value,
                    on_toggle: EventHandler::new({
                        let mut show_dest = show_dest;
                        let mut dest_address = dest_address;
                        move |_| {
                            if show_dest() {
                                dest_address.set(String::new());
                            }
                            show_dest.set(!show_dest());
                        }
                    }),
                    on_address: EventHandler::new({
                        let mut dest_address = dest_address;
                        move |next: String| dest_address.set(next)
                    }),
                }
                SwapActionButton {
                    from: from.clone(),
                    to: to.clone(),
                    amount: numeric_amount,
                    dest_address: dest_address(),
                }
            }
            if let Some(side) = picking() {
                TokenPicker {
                    open: true,
                    side,
                    chains: chains.clone(),
                    tokens: tokens.clone(),
                    selected_id: if side == TokenSide::From { from_id() } else { to_id() },
                    reduce: reduce_value,
                    on_pick: EventHandler::new(move |id: String| pick_token(id)),
                    on_close: EventHandler::new({
                        let mut picking = picking;
                        move |_| picking.set(None)
                    }),
                }
            }
        }
    }
}

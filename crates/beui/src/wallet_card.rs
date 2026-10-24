//! `motion/wallet-card/` — the composed wallet overview: an account switcher
//! that morphs open, a search icon that becomes a bar, a rolling balance with a
//! transient change pill, and the Send / Deposit / Swap / Buy row.
//!
//! The source splits this over eight files; the port keeps it in one, with the
//! same pieces. Everything that fixes behaviour is the source's: the
//! `{duration 0.5, bounce 0.22}` morph (through `spring_from_visual_duration`,
//! so motion's own conversion is reused), the `0.5 / bounce 0` glide for the
//! content that travels across the row, the extra width reserved in flow so the
//! header (and the card) never shift, the deferred `armed` flag that stops the
//! panel flashing a phantom hover under the cursor, the list stagger
//! (`0.12s` then `0.035s` per row, `opacity 0 / y -6 / blur(3px)`), the
//! `ActionSwapText` cascade that swaps the balance for asterisks on one
//! baseline, the change pill that persists until the balance moves again, and
//! `truncateAddress`.
//!
//! Differences, both deliberate: the trigger → panel handoff uses
//! `beui-core::flip` rather than motion's `layoutId` (same idea, and the panel
//! grows out of the trigger's last box), and the list's enter stagger is a
//! per-row CSS animation, so rows arrive with the source's timing but leave
//! without an exit animation. The avatar helper still builds the DiceBear URL;
//! the gallery passes initials so the page stays offline.

use crate::action_swap::{ActionSwapAnimation, ActionSwapIcon, ActionSwapText};
use crate::icons::{
    ArrowDownToLine, ArrowUp, Bell, Check, ChevronDown, Copy, CreditCard, Eye, EyeOff, HistoryIcon,
    Repeat, Search, TrendingDown, TrendingUp,
};
use crate::prelude::*;
use crate::spring::spring_from_visual_duration;
use beui_core::flip::{use_flip, FlipOptions};
use beui_core::hooks::{use_dismiss, DismissOptions};
use beui_core::platform::{js_string, json, next_scope_id};
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle};

const HEAD: &str = "flex items-center gap-2 px-2 py-1.5 text-left";
const ITEM_STAGGER: f64 = 35.0;
const ITEM_DELAY: f64 = 120.0;

const ENTER: &str = "\
@keyframes beui-wallet-item{\
0%{opacity:0;transform:translateY(-6px);filter:blur(3px)}\
100%{opacity:1;transform:translateY(0);filter:blur(0)}}\
@keyframes beui-wallet-pill{\
0%{opacity:0;transform:translateY(6px) scale(0.9)}\
100%{opacity:1;transform:translateY(0) scale(1)}}";

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// The morph the trigger and panel share.
fn morph_config(reduce: bool) -> AnimationConfig {
    if reduce {
        beui_core::ease::tween_ease_out(0)
    } else {
        AnimationConfig::spring(spring_from_visual_duration(0.5, 0.22))
    }
}

/// `diceBearGlassUrl`.
pub fn dice_bear_glass_url(seed: &str) -> String {
    let encoded: String = seed
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.!~*'()".contains(character) {
                character.to_string()
            } else {
                character
                    .to_string()
                    .bytes()
                    .map(|byte| format!("%{byte:02X}"))
                    .collect()
            }
        })
        .collect();
    format!("https://api.dicebear.com/9.x/glass/svg?seed={encoded}")
}

/// `truncateAddress`.
pub fn truncate_address(address: &str) -> String {
    let chars: Vec<char> = address.chars().collect();
    if chars.len() > 12 {
        let head: String = chars[..6].iter().collect();
        let tail: String = chars[chars.len() - 4..].iter().collect();
        format!("{head}…{tail}")
    } else {
        address.to_string()
    }
}

/// One wallet account.
#[derive(Clone, PartialEq)]
pub struct WalletAccount {
    pub id: String,
    pub name: String,
    pub address: String,
    /// Rendered instead of the DiceBear avatar.
    pub avatar: Option<Element>,
}

impl WalletAccount {
    /// An account with a name and an address.
    pub fn new(id: impl Into<String>, name: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            address: address.into(),
            avatar: None,
        }
    }

    /// A custom avatar.
    pub fn avatar(mut self, avatar: Element) -> Self {
        self.avatar = Some(avatar);
        self
    }
}

/// Props for [`WalletCard`].
#[derive(Props, Clone, PartialEq)]
pub struct WalletCardProps {
    pub accounts: Vec<WalletAccount>,
    /// Controlled account.
    #[props(default)]
    pub account_id: Option<String>,
    #[props(default)]
    pub default_account_id: Option<String>,
    #[props(default)]
    pub on_account_change: Option<EventHandler<String>>,
    pub balance: f64,
    #[props(default = "$".to_string())]
    pub balance_prefix: String,
    /// Initial change shown in the pill before any live change.
    #[props(default)]
    pub default_change: Option<f64>,
    #[props(default)]
    pub default_balance_hidden: bool,
    #[props(default)]
    pub on_send: Option<EventHandler<()>>,
    #[props(default)]
    pub on_deposit: Option<EventHandler<()>>,
    #[props(default)]
    pub on_swap: Option<EventHandler<()>>,
    #[props(default)]
    pub on_buy: Option<EventHandler<()>>,
    #[props(default)]
    pub search_placeholder: Option<String>,
    #[props(default)]
    pub search_recent: Vec<String>,
    #[props(default)]
    pub on_search_change: Option<EventHandler<String>>,
    #[props(default)]
    pub on_search_submit: Option<EventHandler<String>>,
    #[props(default)]
    pub has_notifications: bool,
    #[props(default)]
    pub on_notifications: Option<EventHandler<()>>,
    #[props(default)]
    pub class: Option<String>,
}

/// A wallet overview card.
#[component]
pub fn WalletCard(props: WalletCardProps) -> Element {
    let WalletCardProps {
        accounts,
        account_id,
        default_account_id,
        on_account_change,
        balance,
        balance_prefix,
        default_change,
        default_balance_hidden,
        on_send,
        on_deposit,
        on_swap,
        on_buy,
        search_placeholder,
        search_recent,
        on_search_change,
        on_search_submit,
        has_notifications,
        on_notifications,
        class,
    } = props;

    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let controlled = account_id.is_some();
    let mut internal_account = use_signal(|| {
        default_account_id
            .clone()
            .or_else(|| accounts.first().map(|account| account.id.clone()))
    });
    let active_id = account_id.unwrap_or_else(|| internal_account().unwrap_or_default());
    let mut balance_hidden = use_signal(|| default_balance_hidden);

    let active = accounts
        .iter()
        .find(|account| account.id == active_id)
        .cloned()
        .or_else(|| accounts.first().cloned());
    let shown_balance = format!("{}{}", balance_prefix, format_balance(balance, 2));
    let masked = "*".repeat(7);
    let root_class = cn!(
        "relative w-full max-w-xs overflow-hidden rounded-4xl border border-border p-6",
        class,
    );

    rsx! {
        style { dangerous_inner_html: ENTER }
        div { class: root_class,
            div { class: "relative flex items-center justify-between gap-2",
                AccountSwitcher {
                    accounts: accounts.clone(),
                    active: active.clone(),
                    on_select: EventHandler::new(move |id: String| {
                        if !controlled {
                            internal_account.set(Some(id.clone()));
                        }
                        if let Some(handler) = on_account_change {
                            handler.call(id);
                        }
                    }),
                }
                div { class: "flex shrink-0 items-center gap-1",
                    SearchBar {
                        placeholder: search_placeholder.clone().unwrap_or_else(|| "Search".to_string()),
                        recent: search_recent.clone(),
                        on_change: on_search_change,
                        on_submit: on_search_submit,
                    }
                    button {
                        r#type: "button",
                        aria_label: "Notifications",
                        class: "relative inline-flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground",
                        onclick: move |_| {
                            if let Some(handler) = on_notifications {
                                handler.call(());
                            }
                        },
                        Bell { class: Some("h-4 w-4".to_string()) }
                        if has_notifications {
                            span { class: "absolute right-1.5 top-1.5 flex h-2 w-2",
                                span { class: "absolute inline-flex h-full w-full animate-ping rounded-full bg-primary opacity-60" }
                                span { class: "relative inline-flex h-2 w-2 rounded-full bg-primary" }
                            }
                        }
                    }
                }
            }

            div { class: "mt-8 flex flex-col items-center text-center",
                div { class: "flex items-center gap-1.5",
                    p { class: "text-xs text-muted-foreground", "Balance" }
                    button {
                        r#type: "button",
                        aria_label: if balance_hidden() { "Show balance" } else { "Hide balance" },
                        aria_pressed: if balance_hidden() { "true" } else { "false" },
                        class: "text-muted-foreground outline-none transition-colors hover:text-foreground",
                        onclick: move |_| balance_hidden.set(!balance_hidden()),
                        if balance_hidden() {
                            EyeOff { class: Some("h-3.5 w-3.5".to_string()) }
                        } else {
                            Eye { class: Some("h-3.5 w-3.5".to_string()) }
                        }
                    }
                }
                // One cascade swap keeps the number and the mask on one baseline.
                ActionSwapText {
                    value: if balance_hidden() { "hidden".to_string() } else { shown_balance.clone() },
                    label: if balance_hidden() { masked.clone() } else { shown_balance.clone() },
                    animation: ActionSwapAnimation::Cascade,
                    class: Some("text-3xl font-semibold text-foreground".to_string()),
                }
                if balance_hidden() {
                    div { class: "mt-2 flex h-7 items-center justify-center",
                        span { class: "translate-y-[3px] text-sm font-semibold leading-none tracking-[0.3em] text-muted-foreground",
                            "*****"
                        }
                    }
                } else {
                    BalanceDelta { balance, initial_change: default_change, reduce: reduce_value }
                }
            }

            div { class: "mt-8",
                WalletActions { on_send, on_deposit, on_swap, on_buy }
            }
        }
    }
}

/// `1234.5` → `1,234.50`.
fn format_balance(value: f64, decimals: usize) -> String {
    let negative = value < 0.0;
    let scale = 10f64.powi(decimals as i32);
    let scaled = (value.abs() * scale).round() as u64;
    let whole = scaled / scale as u64;
    let fraction = scaled % scale as u64;
    let text = whole.to_string();
    let mut grouped = String::new();
    for (index, character) in text.chars().enumerate() {
        if index > 0 && (text.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    let fraction = if decimals > 0 {
        format!(".{fraction:0width$}", width = decimals)
    } else {
        String::new()
    };
    format!("{}{grouped}{fraction}", if negative { "-" } else { "" })
}

/// The account trigger that morphs into the account panel.
#[component]
fn AccountSwitcher(
    accounts: Vec<WalletAccount>,
    active: Option<WalletAccount>,
    on_select: EventHandler<String>,
) -> Element {
    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let scope = use_hook(next_scope_id);
    let layout_id = format!("wallet-account-{scope}");
    let mut open = use_signal(|| false);
    let armed = use_signal(|| false);

    let close = EventHandler::new({
        let mut open = open;
        move |_| open.set(false)
    });
    let dismiss = use_dismiss(open, close, DismissOptions::default());

    // Hover arms only after the morph settles, so the panel cannot flash a
    // phantom hover under the cursor.
    let arm = {
        let mut armed = armed;
        move || {
            armed.set(false);
            spawn(async move {
                let _ = json::<bool>(&format!(
                    "await new Promise((resolve) => setTimeout(resolve, {}));\nreturn true;",
                    if reduce_value { 0 } else { 280 }
                ))
                .await;
                armed.set(true);
            });
        }
    };
    let mut open_flag = use_signal(|| false);
    use_effect(move || open_flag.set(open()));

    let trigger_flip = use_flip(
        open(),
        FlipOptions::default()
            .layout_id(layout_id.clone())
            .border_radius(16.0)
            .transition(morph_config(reduce_value)),
    );
    let panel_flip = use_flip(
        open(),
        FlipOptions::default()
            .layout_id(layout_id)
            .border_radius(16.0)
            .transition(morph_config(reduce_value)),
    );

    let k_trigger = "trigger";
    let k_panel = "panel";

    rsx! {
        div { "data-beui-scope": dismiss.attr(), class: "min-w-0",
            // The in-flow sizer reserves the widest trigger footprint.
            div { aria_hidden: "true", class: cn!(HEAD, "pointer-events-none opacity-0"),
                AccountAvatar { account: active.clone().or_else(|| accounts.first().cloned()) }
                span { class: "grid",
                    for account in accounts.iter() {
                        span { key: "{account.id}", class: "col-start-1 row-start-1 whitespace-nowrap text-sm font-medium",
                            "{account.name}"
                        }
                    }
                }
                ChevronDown { class: Some("h-4 w-4".to_string()) }
            }

            if !open() {
                button {
                    key: "{k_trigger}",
                    r#type: "button",
                    "data-beui-flip": trigger_flip.scope(),
                    aria_haspopup: "listbox",
                    aria_expanded: "false",
                    class: cn!(HEAD, "absolute -left-2 top-0 z-20 max-w-full outline-none transition-colors hover:bg-muted/60"),
                    style: trigger_flip.style_with("border-radius:16px"),
                    onclick: {
                        let mut open = open;
                        let mut arm = arm;
                        move |_| {
                            open.set(true);
                            arm();
                        }
                    },
                    if let Some(account) = active.clone() {
                        AccountAvatar { account: Some(account.clone()) }
                        span { class: "truncate text-sm font-medium text-foreground", "{account.name}" }
                        span { class: "text-muted-foreground",
                            ChevronDown { class: Some("h-4 w-4".to_string()) }
                        }
                    }
                }
            }

            if open() {
                div {
                    key: "{k_panel}",
                    "data-beui-flip": panel_flip.scope(),
                    role: "listbox",
                    class: "absolute -left-2 -right-2 top-0 z-30 overflow-hidden border border-border/30 bg-background backdrop-blur-md",
                    style: panel_flip.style_with("border-radius:16px"),
                    button {
                        r#type: "button",
                        class: cn!(HEAD, "w-full outline-none"),
                        onclick: move |_| open.set(false),
                        AccountAvatar { account: active.clone() }
                        span { class: "min-w-0 flex-1 truncate text-sm font-medium text-foreground",
                            {active.as_ref().map(|account| account.name.clone()).unwrap_or_else(|| "Select account".to_string())}
                        }
                        span { class: "text-muted-foreground", style: "transform:rotate(180deg)",
                            ChevronDown { class: Some("h-4 w-4".to_string()) }
                        }
                    }
                    ul {
                        class: cn!(
                            "max-h-64 overflow-y-auto p-1.5",
                            if armed() { "" } else { "pointer-events-none" },
                        ),
                        for (index, account) in accounts.iter().enumerate() {
                            {
                                let selected = active.as_ref().is_some_and(|active| active.id == account.id);
                                let item_style = if reduce_value {
                                    String::new()
                                } else {
                                    format!(
                                        "animation:beui-wallet-item 220ms ease-out {}ms both",
                                        ITEM_DELAY + index as f64 * ITEM_STAGGER
                                    )
                                };
                                rsx! {
                                    li {
                                        key: "{account.id}",
                                        class: cn!(
                                            "flex items-center rounded-xl pr-1 text-sm transition-colors",
                                            if selected {
                                                "bg-muted text-foreground"
                                            } else {
                                                "text-muted-foreground"
                                            },
                                        ),
                                        style: item_style,
                                        button {
                                            r#type: "button",
                                            role: "option",
                                            aria_selected: if selected { "true" } else { "false" },
                                            class: "flex min-w-0 flex-1 items-center gap-2.5 rounded-xl px-2 py-2 text-left outline-none",
                                            onclick: {
                                                let id = account.id.clone();
                                                move |_| {
                                                    on_select.call(id.clone());
                                                    open.set(false);
                                                }
                                            },
                                            AccountAvatar { account: Some(account.clone()) }
                                            span { class: "flex min-w-0 flex-1 flex-col",
                                                span { class: "truncate font-medium", "{account.name}" }
                                                span { class: "truncate text-xs text-muted-foreground",
                                                    "{truncate_address(&account.address)}"
                                                }
                                            }
                                            if selected {
                                                Check { class: Some("h-4 w-4 shrink-0 text-foreground".to_string()) }
                                            }
                                        }
                                        CopyButton { value: account.address.clone() }
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

/// The account avatar slot: a custom avatar, then the DiceBear SVG, then the
/// name's initials.
#[component]
fn AccountAvatar(#[props(default)] account: Option<WalletAccount>) -> Element {
    if let Some(avatar) = account.as_ref().and_then(|account| account.avatar.clone()) {
        return rsx! { {avatar} };
    }
    if let Some(account) = account {
        let seed = if account.id.is_empty() {
            account.address.clone()
        } else {
            account.id.clone()
        };
        return rsx! {
            img {
                src: "{dice_bear_glass_url(&seed)}",
                alt: "{account.name}",
                class: "h-7 w-7 shrink-0 rounded-full bg-muted",
            }
        };
    }
    rsx! {
        span { class: "h-7 w-7 shrink-0 rounded-full bg-muted" }
    }
}

/// Copies the address and swaps the icon for a check.
#[component]
fn CopyButton(value: String, #[props(default)] class: Option<String>) -> Element {
    let mut copied = use_signal(|| false);
    let button_class = cn!(
        "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground",
        class,
    );

    rsx! {
        button {
            r#type: "button",
            aria_label: if copied() { "Copied" } else { "Copy address" },
            class: button_class,
            onclick: {
                let value = value.clone();
                move |event: Event<MouseData>| {
                    event.stop_propagation();
                    let value = value.clone();
                    spawn(async move {
                        let script = format!(
                            "try {{ await navigator.clipboard.writeText({}); }} catch (e) {{}}\nreturn true;",
                            js_string(&value)
                        );
                        let _ = json::<bool>(&script).await;
                    });
                    copied.set(true);
                    spawn(async move {
                        let _ = json::<bool>(
                            "await new Promise((resolve) => setTimeout(resolve, 1400));\nreturn true;",
                        )
                        .await;
                        copied.set(false);
                    });
                }
            },
            ActionSwapIcon {
                value: if copied() { "check".to_string() } else { "copy".to_string() },
                animation: ActionSwapAnimation::Cascade,
                class: Some("h-3.5 w-3.5".to_string()),
                if copied() {
                    Check { class: Some("h-3.5 w-3.5 text-emerald-500".to_string()) }
                } else {
                    Copy { class: Some("h-3.5 w-3.5".to_string()) }
                }
            }
        }
    }
}

/// The search icon that becomes a bar, with its recent searches below.
#[component]
fn SearchBar(
    placeholder: String,
    #[props(default)] recent: Vec<String>,
    #[props(default)] on_change: Option<EventHandler<String>>,
    #[props(default)] on_submit: Option<EventHandler<String>>,
) -> Element {
    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let scope = use_hook(next_scope_id);
    let layout_id = format!("wallet-search-{scope}");
    let open = use_signal(|| false);
    let armed = use_signal(|| false);
    let value = use_signal(String::new);

    let close = EventHandler::new({
        let mut open = open;
        move |_| open.set(false)
    });
    let dismiss = use_dismiss(open, close, DismissOptions::default());

    let arm = {
        let mut armed = armed;
        move || {
            armed.set(false);
            spawn(async move {
                let _ = json::<bool>(&format!(
                    "await new Promise((resolve) => setTimeout(resolve, {}));\nreturn true;",
                    if reduce_value { 0 } else { 260 }
                ))
                .await;
                armed.set(true);
            });
        }
    };
    let mut open_flag = use_signal(|| false);
    use_effect(move || open_flag.set(open()));

    let query = value().trim().to_lowercase();
    let filtered: Vec<String> = if query.is_empty() {
        recent.clone()
    } else {
        recent
            .iter()
            .filter(|term| term.to_lowercase().contains(&query))
            .cloned()
            .collect()
    };

    let trigger_flip = use_flip(
        open(),
        FlipOptions::default()
            .layout_id(layout_id.clone())
            .border_radius(12.0)
            .transition(morph_config(reduce_value)),
    );
    let panel_flip = use_flip(
        open(),
        FlipOptions::default()
            .layout_id(layout_id)
            .border_radius(16.0)
            .transition(morph_config(reduce_value)),
    );

    let k_icon = "icon";
    let k_panel = "panel";

    rsx! {
        div { "data-beui-scope": dismiss.attr(), class: "shrink-0",
            // Reserve the icon's width while open, so the row cannot shift.
            if open() {
                div { aria_hidden: "true", class: "h-8 w-8" }
            }

            if !open() {
                button {
                    key: "{k_icon}",
                    r#type: "button",
                    "data-beui-flip": trigger_flip.scope(),
                    aria_label: "Search",
                    class: "inline-flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground",
                    style: trigger_flip.style_with("border-radius:12px"),
                    onclick: {
                        let mut open = open;
                        let mut arm = arm;
                        move |_| {
                            open.set(true);
                            arm();
                        }
                    },
                    Search { class: Some("h-4 w-4".to_string()) }
                }
            }

            if open() {
                div {
                    key: "{k_panel}",
                    "data-beui-flip": panel_flip.scope(),
                    class: "absolute -left-2 -right-2 top-0 z-40 overflow-hidden border border-border/30 bg-background backdrop-blur-md",
                    style: panel_flip.style_with("border-radius:16px"),
                    div { class: "flex h-9 items-center gap-2 px-3",
                        span { class: "shrink-0",
                            Search { class: Some("h-4 w-4 text-muted-foreground".to_string()) }
                        }
                        input {
                            "data-beui-wallet-search": "{scope}",
                            value: "{value()}",
                            placeholder,
                            class: "min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground",
                            oninput: {
                                let mut value = value;
                                move |event: Event<FormData>| {
                                    value.set(event.value());
                                    if let Some(handler) = on_change {
                                        handler.call(event.value());
                                    }
                                }
                            },
                            onkeydown: {
                                let mut open = open;
                                move |event: Event<KeyboardData>| {
                                    match event.key() {
                                        Key::Enter => {
                                            let trimmed = value().trim().to_string();
                                            if !trimmed.is_empty() {
                                                if let Some(handler) = on_submit {
                                                    handler.call(trimmed);
                                                }
                                                open.set(false);
                                            }
                                        }
                                        Key::Escape => open.set(false),
                                        _ => {}
                                    }
                                }
                            },
                            onmounted: {
                                let selector = format!("[data-beui-wallet-search=\"{scope}\"]");
                                move |_| {
                                    let selector = selector.clone();
                                    spawn(async move {
                                        let script = format!(
                                            "await new Promise((resolve) => requestAnimationFrame(resolve));\n\
                                             const el = document.querySelector({});\n\
                                             if (el) el.focus();\n\
                                             return true;",
                                            js_string(&selector)
                                        );
                                        let _ = json::<bool>(&script).await;
                                    });
                                }
                            },
                        }
                    }
                    div { class: "h-px bg-border/40" }
                    if !filtered.is_empty() {
                        ul {
                            class: cn!(
                                "max-h-56 overflow-y-auto p-1.5",
                                if armed() { "" } else { "pointer-events-none" },
                            ),
                            for (index, term) in filtered.iter().enumerate() {
                                li {
                                    key: "{term}",
                                    style: if reduce_value {
                                        String::new()
                                    } else {
                                        format!(
                                            "animation:beui-wallet-item 220ms ease-out {}ms both",
                                            ITEM_DELAY + index as f64 * ITEM_STAGGER
                                        )
                                    },
                                    button {
                                        r#type: "button",
                                        class: cn!(
                                            "flex w-full items-center gap-2.5 rounded-xl px-2.5 py-2 text-left text-sm text-muted-foreground outline-none transition-colors",
                                            if armed() { "hover:bg-muted hover:text-foreground" } else { "" },
                                        ),
                                        onclick: {
                                            let mut open = open;
                                            let mut value = value;
                                            let term = term.clone();
                                            move |_| {
                                                value.set(term.clone());
                                                if let Some(handler) = on_change {
                                                    handler.call(term.clone());
                                                }
                                                if let Some(handler) = on_submit {
                                                    handler.call(term.clone());
                                                }
                                                open.set(false);
                                            }
                                        },
                                        HistoryIcon { class: Some("h-4 w-4 shrink-0".to_string()) }
                                        span { class: "min-w-0 flex-1 truncate", "{term}" }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "flex flex-col items-center gap-1 px-4 py-8 text-center",
                            Search { class: Some("h-5 w-5 text-muted-foreground/60".to_string()) }
                            p { class: "text-sm text-muted-foreground",
                                if query.is_empty() { "No recent searches" } else { "No matches" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The transient balance change pill.
#[component]
fn BalanceDelta(balance: f64, initial_change: Option<f64>, reduce: bool) -> Element {
    // `peek()`/`peek()` throughout: an effect that both reads and writes the
    // same signal never settles, and on mount that pegs the renderer. The only
    // tracked input is the `balance` prop, declared through `use_reactive`.
    let mut previous = use_signal(|| balance);
    let mut delta = use_signal(|| initial_change);
    let mut generation = use_signal(|| 0u32);

    use_effect(use_reactive((&balance,), move |(balance,)| {
        let diff = balance - *previous.peek();
        if diff == 0.0 {
            return;
        }
        previous.set(balance);
        let next = *generation.peek() + 1;
        generation.set(next);
        delta.set(Some(diff));
    }));

    let amount = *delta.peek();
    let up = amount.map(|amount| amount > 0.0).unwrap_or(false);
    let sign = if up { "+" } else { "-" };
    let pill_key = format!("{}-{}", *generation.peek(), amount.unwrap_or_default());

    rsx! {
        div { class: "mt-2 flex h-7 items-center justify-center",
            if let Some(amount) = amount {
                span {
                    key: "{pill_key}",
                    class: cn!(
                        "inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs font-semibold tabular-nums",
                        if up {
                            "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400"
                        } else {
                            "bg-red-500/15 text-red-600 dark:text-red-400"
                        },
                    ),
                    style: if reduce {
                        String::new()
                    } else {
                        "animation:beui-wallet-pill 200ms ease-out both".to_string()
                    },
                    if up {
                        TrendingUp { class: Some("h-3.5 w-3.5".to_string()) }
                    } else {
                        TrendingDown { class: Some("h-3.5 w-3.5".to_string()) }
                    }
                    "{sign}${format_balance(amount.abs(), 2)}"
                }
            }
        }
    }
}

/// The four primary actions.
#[component]
fn WalletActions(
    #[props(default)] on_send: Option<EventHandler<()>>,
    #[props(default)] on_deposit: Option<EventHandler<()>>,
    #[props(default)] on_swap: Option<EventHandler<()>>,
    #[props(default)] on_buy: Option<EventHandler<()>>,
) -> Element {
    let handlers = [on_send, on_deposit, on_swap, on_buy];
    let labels = ["Send", "Deposit", "Swap", "Buy"];

    rsx! {
        div { class: "flex items-start justify-between gap-2",
            for (index, label) in labels.iter().enumerate() {
                WalletAction { key: "{label}", index, label: (*label).to_string(), handler: handlers[index] }
            }
        }
    }
}

/// One action button, with the spring press.
#[component]
fn WalletAction(index: usize, label: String, handler: Option<EventHandler<()>>) -> Element {
    let reduce = use_reduced_motion();
    let reduce_value = reduce();
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");

    rsx! {
        button {
            r#type: "button",
            class: "flex flex-1 flex-col items-center gap-2 outline-none",
            style: if reduce_value { String::new() } else { format!("transform:scale({})", current(scale)) },
            onclick: move |_| {
                if let Some(handler) = handler {
                    handler.call(());
                }
            },
            onpointerdown: move |_| {
                if !reduce_value {
                    let _ = scale.animate_to(0.94, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
            },
            onpointercancel: move |_| {
                let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
            },
            span { class: "flex h-12 w-12 items-center justify-center rounded-full bg-muted text-foreground",
                {match index {
                    0 => rsx! { ArrowUp { class: Some("h-5 w-5".to_string()) } },
                    1 => rsx! { ArrowDownToLine { class: Some("h-5 w-5".to_string()) } },
                    2 => rsx! { Repeat { class: Some("h-5 w-5".to_string()) } },
                    _ => rsx! { CreditCard { class: Some("h-5 w-5".to_string()) } },
                }}
            }
            span { class: "text-xs font-medium text-muted-foreground", "{label}" }
        }
    }
}

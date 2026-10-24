//! `motion/not-found.tsx` — the 404 block, in five variants.
//!
//! The source ships one file per variant over a shared stage and dual CTA;
//! per rule 9 the variant is an enum on one [`NotFound`] rather than five
//! exports. Everything is the source's: the stacked cards' `rest`/`hover`
//! triples on `SPRING_PANEL`, the glitch's three scrambled layers (the two
//! chromatic ghosts are pure Tailwind `group-hover` / `motion-reduce`), the
//! spotlight's `radial-gradient(220px circle at x% y%, …)` mask following the
//! pointer, the terminal's three staggered `TextReveal` lines plus the blinking
//! cursor, and the magnetic per-character 0.6 strength with the `-ml-2` overlap.

use crate::magnetic::Magnetic;
use crate::prelude::*;
use crate::rect::use_element_rect;
use crate::text::TextTag;
use crate::text_reveal::{SplitMode, TextReveal};
use crate::text_scramble::TextScramble;
use dioxus::prelude::ReadableExt;
use dioxus_motion::prelude::{use_motion, AnimationConfig, AnimationManager, MotionHandle, Spring};

fn current(handle: MotionHandle<f32>) -> f32 {
    *handle.current().read()
}

/// `NOT_FOUND_DEFAULTS`.
const DEFAULT_CODE: &str = "404";
const DEFAULT_TITLE: &str = "Page not found";
const DEFAULT_DESCRIPTION: &str = "The page you are looking for moved, vanished, or never existed.";
const DEFAULT_HOME_HREF: &str = "/";
const DEFAULT_HOME_LABEL: &str = "Back home";
const DEFAULT_BROWSE_HREF: &str = "/components/motion";
const DEFAULT_BROWSE_LABEL: &str = "Browse components";

const GLYPHS: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ0123456789#%&@$?/\\";

/// How the 404 is staged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotFoundVariant {
    /// Three stacked cards that fan out on hover.
    #[default]
    Stacked,
    /// A terminal window typing the failed `cd`.
    Terminal,
    /// The code, scrambled on mount, with chromatic ghosts on hover.
    Glitch,
    /// The code dimmed behind a pointer-following spotlight.
    Spotlight,
    /// The code, each glyph leaning toward the cursor.
    Magnetic,
}

/// Props for [`NotFound`].
#[derive(Props, Clone, PartialEq)]
pub struct NotFoundProps {
    #[props(default)]
    pub variant: NotFoundVariant,
    /// The big status code. Defaults to `"404"`.
    #[props(default)]
    pub code: Option<String>,
    #[props(default)]
    pub title: Option<String>,
    #[props(default)]
    pub description: Option<String>,
    #[props(default)]
    pub home_href: Option<String>,
    #[props(default)]
    pub home_label: Option<String>,
    #[props(default)]
    pub browse_href: Option<String>,
    #[props(default)]
    pub browse_label: Option<String>,
    #[props(default)]
    pub class: Option<String>,
}

impl NotFoundProps {
    fn code(&self) -> String {
        self.code
            .clone()
            .unwrap_or_else(|| DEFAULT_CODE.to_string())
    }

    fn title(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| DEFAULT_TITLE.to_string())
    }

    fn description(&self) -> String {
        self.description
            .clone()
            .unwrap_or_else(|| DEFAULT_DESCRIPTION.to_string())
    }
}

/// A 404 page.
#[component]
pub fn NotFound(props: NotFoundProps) -> Element {
    let code = props.code();
    let title = props.title();
    let description = props.description();
    let class = props.class.clone();
    let actions = (
        props.home_href.clone(),
        props.home_label.clone(),
        props.browse_href.clone(),
        props.browse_label.clone(),
    );

    match props.variant {
        NotFoundVariant::Stacked => {
            rsx! { StackedView { code, title, description, class, actions } }
        }
        NotFoundVariant::Terminal => {
            rsx! { TerminalView { code, title, description, class, actions } }
        }
        NotFoundVariant::Glitch => {
            rsx! { GlitchView { code, title, description, class, actions } }
        }
        NotFoundVariant::Spotlight => {
            rsx! { SpotlightView { code, title, description, class, actions } }
        }
        NotFoundVariant::Magnetic => {
            rsx! { MagneticView { code, title, description, class, actions } }
        }
    }
}

type Actions = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Props for [`NotFoundStage`].
#[derive(Props, Clone, PartialEq)]
pub struct NotFoundStageProps {
    #[props(default)]
    pub class: Option<String>,
    pub children: Element,
}

/// Centers a variant and gives it a consistent minimum stage height.
#[component]
pub fn NotFoundStage(props: NotFoundStageProps) -> Element {
    rsx! {
        div {
            class: cn!(
                "flex min-h-[420px] w-full flex-col items-center justify-center gap-8 px-4 text-center",
                props.class,
            ),
            {props.children}
        }
    }
}

/// Props for [`NotFoundActions`].
#[derive(Props, Clone, PartialEq)]
pub struct NotFoundActionsProps {
    #[props(default)]
    pub home_href: Option<String>,
    #[props(default)]
    pub home_label: Option<String>,
    #[props(default)]
    pub browse_href: Option<String>,
    #[props(default)]
    pub browse_label: Option<String>,
    #[props(default)]
    pub class: Option<String>,
}

/// The shared dual CTA.
#[component]
pub fn NotFoundActions(props: NotFoundActionsProps) -> Element {
    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let interactive = !reduce() && can_hover();
    let home_href = props
        .home_href
        .clone()
        .unwrap_or_else(|| DEFAULT_HOME_HREF.to_string());
    let home_label = props
        .home_label
        .clone()
        .unwrap_or_else(|| DEFAULT_HOME_LABEL.to_string());
    let browse_href = props
        .browse_href
        .clone()
        .unwrap_or_else(|| DEFAULT_BROWSE_HREF.to_string());
    let browse_label = props
        .browse_label
        .clone()
        .unwrap_or_else(|| DEFAULT_BROWSE_LABEL.to_string());

    rsx! {
        div { class: cn!("flex flex-wrap items-center justify-center gap-3", props.class),
            ScaleAnchor {
                href: home_href,
                interactive,
                class: "inline-flex h-11 select-none items-center justify-center rounded-full bg-primary px-6 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90".to_string(),
                "{home_label}"
            }
            ScaleAnchor {
                href: browse_href,
                interactive,
                class: "inline-flex h-11 select-none items-center justify-center rounded-full border border-border bg-card px-6 text-sm font-medium text-foreground transition-colors hover:bg-muted/60".to_string(),
                "{browse_label}"
            }
        }
    }
}

/// The CTA itself: `whileTap` 0.96 / `whileHover` 1.02 on `SPRING_PRESS`.
#[component]
fn ScaleAnchor(href: String, interactive: bool, class: String, children: Element) -> Element {
    let mut scale = use_motion(1.0f32).expect("1.0 is finite");
    let mut hovering = use_signal(|| false);

    rsx! {
        a {
            href,
            class,
            style: if interactive {
                format!("transform:scale({})", current(scale))
            } else {
                String::new()
            },
            onpointerenter: move |_| {
                if interactive {
                    hovering.set(true);
                    let _ = scale.animate_to(1.02, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointerleave: move |_| {
                if interactive {
                    hovering.set(false);
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointerdown: move |_| {
                if interactive {
                    let _ = scale.animate_to(0.96, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointerup: move |_| {
                if interactive {
                    let _ = scale.animate_to(if hovering() { 1.02 } else { 1.0 }, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            onpointercancel: move |_| {
                if interactive {
                    let _ = scale.animate_to(1.0, AnimationConfig::spring(SPRING_PRESS));
                }
            },
            {children}
        }
    }
}

/// The shared "title + description" block under every variant.
#[component]
fn NotFoundCopy(title: String, description: String) -> Element {
    rsx! {
        div { class: "flex flex-col items-center gap-2",
            p { class: "text-lg font-semibold text-foreground", "{title}" }
            p { class: "max-w-sm text-sm text-muted-foreground", "{description}" }
        }
    }
}

#[component]
fn StackedView(
    code: String,
    title: String,
    description: String,
    class: Option<String>,
    actions: Actions,
) -> Element {
    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let interactive = !reduce() && can_hover();
    let mut hover = use_motion(0.0f32).expect("0.0 is finite");
    let mut set_hover = move |on: bool| {
        if interactive {
            let _ = hover.animate_to(
                if on { 1.0 } else { 0.0 },
                AnimationConfig::spring(SPRING_PANEL),
            );
        }
    };
    let p = f64::from(current(hover).clamp(0.0, 1.0));

    let card = "absolute inset-0 rounded-3xl border border-border bg-card shadow-sm";
    let left_style = format!(
        "transform:translate({}px, {}px) rotate({}deg)",
        -28.0 * p,
        8.0 * p,
        -9.0 * p
    );
    let right_style = format!(
        "transform:translate({}px, {}px) rotate({}deg)",
        28.0 * p,
        8.0 * p,
        9.0 * p
    );
    let front_style = format!("transform:translateY({}px)", -6.0 * p);

    rsx! {
        NotFoundStage { class,
            div {
                class: "relative h-44 w-64",
                onpointerenter: move |_| set_hover(true),
                onpointerleave: move |_| set_hover(false),
                div { aria_hidden: "true", class: card, style: left_style }
                div { aria_hidden: "true", class: card, style: right_style }
                div {
                    class: "absolute inset-0 flex flex-col items-center justify-center gap-1 rounded-3xl border border-border bg-card shadow-md",
                    style: front_style,
                    h1 {
                        "data-not-found-code": "",
                        class: "select-none font-bold leading-none tracking-tighter text-foreground [font-size:clamp(3.5rem,9vw,5rem)]",
                        "{code}"
                    }
                    span { class: "text-xs font-medium uppercase tracking-wide text-muted-foreground",
                        "out of the deck"
                    }
                }
            }
            NotFoundCopy { title, description }
            NotFoundActions {
                home_href: actions.0.clone(),
                home_label: actions.1.clone(),
                browse_href: actions.2.clone(),
                browse_label: actions.3.clone(),
            }
        }
    }
}

#[component]
fn TerminalView(
    code: String,
    title: String,
    description: String,
    class: Option<String>,
    actions: Actions,
) -> Element {
    let type_spring = Spring {
        stiffness: 320.0,
        damping: 30.0,
        mass: 0.6,
    };
    let cursor = "ml-1 inline-block h-[1.1em] w-[0.55ch] translate-y-[0.12em] bg-white/80 motion-safe:animate-pulse";

    rsx! {
        NotFoundStage { class,
            div { class: "w-full max-w-md overflow-hidden rounded-xl border border-border bg-neutral-950 text-left shadow-lg",
                div { class: "flex items-center gap-1.5 border-b border-white/10 px-4 py-3",
                    span { class: "h-3 w-3 rounded-full bg-[#ff5f57]" }
                    span { class: "h-3 w-3 rounded-full bg-[#febc2e]" }
                    span { class: "h-3 w-3 rounded-full bg-[#28c840]" }
                    span { class: "ml-2 text-xs text-white/40", "~/beui" }
                }
                div { class: "space-y-1.5 p-4 font-mono text-sm leading-relaxed",
                    TextReveal {
                        text: vec!["$ cd /page".to_string()],
                        r#as: TextTag::P,
                        split: SplitMode::Char,
                        stagger: 0.018,
                        blur: 6.0,
                        y_offset: "0".to_string(),
                        spring: Some(type_spring),
                        class: Some("text-white/80".to_string()),
                    }
                    TextReveal {
                        text: vec!["cd: no such file or directory: /page".to_string()],
                        r#as: TextTag::P,
                        split: SplitMode::Char,
                        stagger: 0.012,
                        delay: 0.45,
                        blur: 6.0,
                        y_offset: "0".to_string(),
                        spring: Some(type_spring),
                        class: Some("text-[#ff5f57]".to_string()),
                    }
                    p { class: "flex items-center text-white/80",
                        TextReveal {
                            text: vec![format!("$ status {code}")],
                            r#as: TextTag::Span,
                            split: SplitMode::Char,
                            stagger: 0.018,
                            delay: 1.1,
                            blur: 6.0,
                            y_offset: "0".to_string(),
                            spring: Some(type_spring),
                        }
                        span { class: cursor }
                    }
                }
            }
            NotFoundCopy { title, description }
            NotFoundActions {
                home_href: actions.0.clone(),
                home_label: actions.1.clone(),
                browse_href: actions.2.clone(),
                browse_label: actions.3.clone(),
            }
        }
    }
}

#[component]
fn GlitchView(
    code: String,
    title: String,
    description: String,
    class: Option<String>,
    actions: Actions,
) -> Element {
    // The chromatic ghosts are pure CSS: `group-hover` nudges and fades them,
    // `motion-reduce:hidden` drops them entirely.
    let ghost = "pointer-events-none absolute inset-0 opacity-0 mix-blend-screen transition-[transform,opacity] duration-150 ease-out motion-reduce:hidden";

    rsx! {
        NotFoundStage { class,
            div { class: "group relative select-none font-mono font-bold leading-none tracking-tighter text-foreground [font-size:clamp(5rem,18vw,11rem)]",
                span {
                    aria_hidden: "true",
                    class: "{ghost} text-[#ff0040] group-hover:translate-x-[3px] group-hover:opacity-70",
                    TextScramble {
                        text: code.clone(),
                        duration: Some(700.0),
                        glyphs: GLYPHS.to_string(),
                        auto_play: true,
                    }
                }
                span {
                    aria_hidden: "true",
                    class: "{ghost} text-[#00e5ff] group-hover:-translate-x-[3px] group-hover:opacity-70",
                    TextScramble {
                        text: code.clone(),
                        duration: Some(700.0),
                        glyphs: GLYPHS.to_string(),
                        auto_play: true,
                    }
                }
                h1 { "data-not-found-code": "", class: "relative",
                    TextScramble {
                        text: code.clone(),
                        duration: Some(700.0),
                        glyphs: GLYPHS.to_string(),
                        auto_play: true,
                    }
                }
            }
            NotFoundCopy { title, description }
            NotFoundActions {
                home_href: actions.0.clone(),
                home_label: actions.1.clone(),
                browse_href: actions.2.clone(),
                browse_label: actions.3.clone(),
            }
        }
    }
}

#[component]
fn SpotlightView(
    code: String,
    title: String,
    description: String,
    class: Option<String>,
    actions: Actions,
) -> Element {
    let reduce = use_reduced_motion();
    let can_hover = use_hover_capable();
    let enabled = !reduce() && can_hover();
    let rect = use_element_rect();
    let mut position = use_signal(|| (50.0f64, 50.0f64));
    let (mx, my) = position();
    let mask = format!(
        "-webkit-mask-image:radial-gradient(220px circle at {mx:.2}% {my:.2}%, #000 25%, transparent 72%);mask-image:radial-gradient(220px circle at {mx:.2}% {my:.2}%, #000 25%, transparent 72%)"
    );
    let big =
        "select-none font-bold leading-none tracking-tighter [font-size:clamp(5rem,16vw,10rem)]";

    rsx! {
        NotFoundStage { class,
            div {
                "data-beui-rect": rect.attr(),
                class: "relative isolate flex aspect-[16/9] w-full max-w-xl items-center justify-center overflow-hidden rounded-3xl border border-border bg-neutral-950",
                onmounted: move |event| rect.measure(event.data()),
                onpointermove: move |event: Event<PointerData>| {
                    if !enabled {
                        return;
                    }
                    let Some(stage_rect) = rect.get() else {
                        return;
                    };
                    if stage_rect.width <= 0.0 || stage_rect.height <= 0.0 {
                        return;
                    }
                    let point = event.client_coordinates();
                    position.set((
                        ((point.x - stage_rect.left) / stage_rect.width * 100.0).clamp(0.0, 100.0),
                        ((point.y - stage_rect.top) / stage_rect.height * 100.0).clamp(0.0, 100.0),
                    ));
                },
                // Dim base layer.
                span { aria_hidden: "true", class: "{big} text-white/10", "{code}" }
                // Bright layer, revealed only under the spotlight.
                h1 {
                    "data-not-found-code": "",
                    aria_label: "{code}",
                    class: cn!("absolute {big}", if enabled { "text-white" } else { "text-white/90" }),
                    style: if enabled { mask } else { String::new() },
                    span { aria_hidden: "true", "{code}" }
                }
            }
            NotFoundCopy { title, description }
            NotFoundActions {
                home_href: actions.0.clone(),
                home_label: actions.1.clone(),
                browse_href: actions.2.clone(),
                browse_label: actions.3.clone(),
            }
        }
    }
}

#[component]
fn MagneticView(
    code: String,
    title: String,
    description: String,
    class: Option<String>,
    actions: Actions,
) -> Element {
    let chars: Vec<(usize, String)> = code
        .chars()
        .enumerate()
        .map(|(index, ch)| (index, ch.to_string()))
        .collect();

    rsx! {
        NotFoundStage { class,
            h1 {
                "data-not-found-code": "",
                aria_label: "{code}",
                class: "flex select-none items-center justify-center font-bold leading-none tracking-tighter text-foreground [font-size:clamp(5rem,18vw,12rem)]",
                for (index, ch) in chars {
                    Magnetic {
                        key: "{index}",
                        strength: 0.6,
                        class: cn!(if index > 0 { "-ml-2" } else { "" }),
                        span { aria_hidden: "true", class: "inline-block px-1 tabular-nums", "{ch}" }
                    }
                }
            }
            NotFoundCopy { title, description }
            NotFoundActions {
                home_href: actions.0.clone(),
                home_label: actions.1.clone(),
                browse_href: actions.2.clone(),
                browse_label: actions.3.clone(),
            }
        }
    }
}

//! The gallery app shell.
//!
//! One page per component, same copy and structure as beUI's previews, plus two
//! not-a-component pages — `foundation` (tokens, palettes, fonts, utilities) and
//! `hooks` (the shared input primitives) — because those have no beUI preview to
//! mirror but still need somewhere to be observed and diffed.
//!
//! The slug in [`PAGES`] is the URL path segment (`dioxus-router`), and
//! [`nav`] renders the only navigation there is. The beUI site replica that used
//! to wrap these pages — landing, index, category pages, detail pages, docs,
//! site header, right rail, `Customize` drawer — is gone by design: this is a
//! component browser, not a copy of beui.dev.

use beui::prelude::*;

mod nav;
mod pages;
mod registry;

fn main() {
    dioxus::launch(App);
}

/// Every page that exists, in nav order: `(slug, label)`. This is the truth the
/// navigation is built from, so it must only ever name a page `render` can
/// serve.
pub(crate) const PAGES: &[(&str, &str)] = &[
    ("foundation", "Foundation"),
    ("button", "Button"),
    ("input", "Input"),
    ("checkbox", "Checkbox"),
    ("switch", "Switch"),
    ("radio", "Radio"),
    ("card", "Card"),
    ("loader", "Loader"),
    ("animated-badge", "Badge"),
    ("range-slider", "Slider"),
    ("color-selector", "Colors"),
    ("color-picker", "Picker"),
    ("otp-input", "OTP"),
    ("adaptive-stepper", "Stepper"),
    ("wheel-picker", "Wheel"),
    ("date-range-picker", "Dates"),
    ("hooks", "Hooks"),
    ("presence", "Presence"),
    ("flip", "Layout"),
    ("drag", "Drag"),
    ("portal", "Portal"),
    ("agents", "Agents"),
    ("conversation", "Conversation"),
    ("activity", "Activity"),
    ("approval", "Approval"),
    ("generation", "Generation"),
    ("sidebar", "Sidebar"),
    ("charts", "Charts"),
    ("alert", "Alert"),
    ("collapsible", "Collapsible"),
    ("tabs", "Tabs"),
    ("drawer", "Drawer"),
    ("bottom-sheet", "Bottom sheet"),
    ("animated-toast-stack", "Toast stack"),
    ("dock", "Dock"),
    ("select", "Select"),
    ("bouncy-accordion", "Accordion"),
    ("popover", "Popover"),
    ("morph-popover", "Morph popover"),
    ("breadcrumb", "Breadcrumb"),
    ("morph-select", "Morph select"),
    ("expandable-tabs", "Expandable tabs"),
    ("command-palette", "Command palette"),
    ("morphing-modal", "Morphing modal"),
    ("center-morph-modal", "Center morph"),
    ("context-menu", "Context menu"),
    ("combobox", "Combobox"),
    ("multi-select", "Multi select"),
    ("tooltip", "Tooltip"),
    ("morphing-tabs", "Morphing tabs"),
    ("animated-sidebar", "Animated sidebar"),
    ("text-shimmer", "TextShimmer"),
    ("marquee", "Marquee"),
    ("magnetic", "Magnetic"),
    ("tilt-card", "TiltCard"),
    ("scroll-reveal", "ScrollReveal"),
    ("scroll-progress", "ScrollProgress"),
    ("parallax", "Parallax"),
    ("animated-number", "AnimatedNumber"),
    ("arc-picker", "ArcPicker"),
    ("attachment-upload", "AttachmentUpload"),
    ("text-reveal", "TextReveal"),
    ("text-scramble", "TextScramble"),
    ("digit-swap", "DigitSwap"),
    ("number-ticker", "NumberTicker"),
    ("text-cascade", "TextCascade"),
    ("action-swap", "ActionSwap"),
    ("theme-toggle", "ThemeToggle"),
    ("shared-layout-bg", "SharedLayoutBg"),
    ("chromatic-text-reveal", "ChromaticTextReveal"),
    ("preview-rail", "PreviewRail"),
    ("aspect-ratio", "AspectRatio"),
    ("notification-stack", "NotificationStack"),
    ("bounce-sidebar", "BounceSidebar"),
    ("bloom-menu", "BloomMenu"),
    ("expanding-arrow-button", "ExpandingArrowButton"),
    ("slide-action-button", "SlideActionButton"),
    ("hold-action-button", "HoldActionButton"),
    ("expandable-control", "ExpandableControl"),
    ("overflow-actions", "OverflowActions"),
    ("smooth-scroll", "ScrollTo"),
    ("card-folder", "CardFolder"),
    ("dynamic-island", "DynamicIsland"),
    ("file-tree", "FileTree"),
    ("expandable-action-bar", "ExpandableActionBar"),
    ("swipeable-list", "SwipeableList"),
    ("project-folder", "ProjectFolder"),
    ("pull-to-refresh", "PullToRefresh"),
    ("infinite-masonry", "Visual index"),
    ("not-found-stacked", "NotFound"),
    ("not-found-terminal", "NotFound"),
    ("not-found-glitch", "NotFound"),
    ("not-found-spotlight", "NotFound"),
    ("not-found-magnetic", "NotFound"),
    ("range-slider-wave", "Gain"),
    ("prediction-market-card", "Boston Celtics"),
    ("sortable-list", "In the studio"),
    ("sortable-stack", "In the studio"),
    ("liquid", "Overview"),
    ("cylinder-carousel", "CylinderCarousel"),
    ("feedback-widget", "FeedbackWidget"),
    ("signup-form", "Create your account"),
    ("morphing-search-chips", "MorphingSearchChips"),
    ("prediction-market", "PredictionMarket"),
    ("morphing-search", "MorphingSearch"),
    ("file-upload", "Drop files here"),
    ("knockout-bracket", "KnockoutBracket"),
    ("wallet-card", "WalletCard"),
    ("swap", "MultiChainSwap"),
    ("knockout-wheel", "KnockoutWheel"),
    ("table", "Table"),
    ("table-editable", "Table editable"),
    ("table-async", "Table async"),
    ("availability-scheduler", "AvailabilityScheduler"),
    ("morphing-lightbox", "MorphingLightbox"),
];

pub fn render(slug: &str) -> Element {
    match slug {
        "foundation" => rsx! { pages::foundation::Foundation {} },
        "button" => rsx! { pages::button::ButtonPage {} },
        "input" => rsx! { pages::input::InputPage {} },
        "checkbox" => rsx! { pages::checkbox::CheckboxPage {} },
        "switch" => rsx! { pages::switch::SwitchPage {} },
        "radio" => rsx! { pages::radio::RadioPage {} },
        "card" => rsx! { pages::card::CardPage {} },
        "loader" => rsx! { pages::loader::LoaderPage {} },
        "animated-badge" => rsx! { pages::animated_badge::AnimatedBadgePage {} },
        "range-slider" => rsx! { pages::range_slider::RangeSliderPage {} },
        "color-selector" => rsx! { pages::color_selector::ColorSelectorPage {} },
        "color-picker" => rsx! { pages::color_picker::ColorPickerPage {} },
        "otp-input" => rsx! { pages::otp_input::OTPInputPage {} },
        "adaptive-stepper" => rsx! { pages::adaptive_stepper::AdaptiveStepperPage {} },
        "wheel-picker" => rsx! { pages::wheel_picker::WheelPickerPage {} },
        "date-range-picker" => rsx! { pages::date_range_picker::DateRangePickerPage {} },
        "hooks" => rsx! { pages::hooks::Hooks {} },
        "presence" => rsx! { pages::presence::Presence {} },
        "flip" => rsx! { pages::flip::Flip {} },
        "drag" => rsx! { pages::drag::Drag {} },
        "portal" => rsx! { pages::portal::Portal {} },
        "agents" => rsx! { pages::agents::Agents {} },
        "conversation" => rsx! { pages::conversation::Conversation {} },
        "activity" => rsx! { pages::activity::Activity {} },
        "approval" => rsx! { pages::approval::Approval {} },
        "generation" => rsx! { pages::generation::Generation {} },
        "sidebar" => rsx! { pages::sidebar::Sidebar {} },
        "charts" => rsx! { pages::charts::Charts {} },
        "alert" => rsx! { pages::alert::AlertPage {} },
        "collapsible" => rsx! { pages::collapsible::CollapsiblePage {} },
        "tabs" => rsx! { pages::tabs::TabsPage {} },
        "drawer" => rsx! { pages::drawer::DrawerPage {} },
        "bottom-sheet" => rsx! { pages::bottom_sheet::BottomSheetPage {} },
        "animated-toast-stack" => rsx! { pages::animated_toast_stack::AnimatedToastStackPage {} },
        "dock" => rsx! { pages::dock::DockPage {} },
        "select" => rsx! { pages::select::SelectPage {} },
        "bouncy-accordion" => rsx! { pages::bouncy_accordion::BouncyAccordionPage {} },
        "popover" => rsx! { pages::popover::PopoverPage {} },
        "morph-popover" => rsx! { pages::morph_popover::MorphPopoverPage {} },
        "breadcrumb" => rsx! { pages::breadcrumb::BreadcrumbDemo {} },
        "morph-select" => rsx! { pages::morph_select::MorphSelectPage {} },
        "expandable-tabs" => rsx! { pages::expandable_tabs::ExpandableTabsPage {} },
        "command-palette" => rsx! { pages::command_palette::CommandPalettePage {} },
        "morphing-modal" => rsx! { pages::morphing_modal::MorphingModalPage {} },
        "center-morph-modal" => rsx! { pages::center_morph_modal::CenterMorphModalPage {} },
        "context-menu" => rsx! { pages::context_menu::ContextMenuPage {} },
        "combobox" => rsx! { pages::combobox::ComboboxPage {} },
        "multi-select" => rsx! { pages::multi_select::MultiSelectPage {} },
        "tooltip" => rsx! { pages::tooltip::TooltipPage {} },
        "morphing-tabs" => rsx! { pages::morphing_tabs::MorphingTabsPage {} },
        "animated-sidebar" => rsx! { pages::animated_sidebar::AnimatedSidebarPage {} },
        "text-shimmer" => rsx! { pages::text_shimmer::TextShimmerPage {} },
        "marquee" => rsx! { pages::marquee::MarqueePage {} },
        "magnetic" => rsx! { pages::magnetic::MagneticPage {} },
        "tilt-card" => rsx! { pages::tilt_card::TiltCardPage {} },
        "scroll-reveal" => rsx! { pages::scroll_reveal::ScrollRevealPage {} },
        "scroll-progress" => rsx! { pages::scroll_progress::ScrollProgressPage {} },
        "parallax" => rsx! { pages::parallax::ParallaxPage {} },
        "animated-number" => rsx! { pages::animated_number::AnimatedNumberPage {} },
        "arc-picker" => rsx! { pages::arc_picker::ArcPickerPage {} },
        "attachment-upload" => rsx! { pages::attachment_upload::AttachmentUploadPage {} },
        "text-reveal" => rsx! { pages::text_reveal::TextRevealPage {} },
        "text-scramble" => rsx! { pages::text_scramble::TextScramblePage {} },
        "digit-swap" => rsx! { pages::digit_swap::DigitSwapPage {} },
        "number-ticker" => rsx! { pages::number_ticker::NumberTickerPage {} },
        "text-cascade" => rsx! { pages::text_cascade::TextCascadePage {} },
        "action-swap" => rsx! { pages::action_swap::ActionSwapPage {} },
        "theme-toggle" => rsx! { pages::theme_toggle::ThemeTogglePage {} },
        "shared-layout-bg" => rsx! { pages::shared_layout_bg::SharedLayoutBgPage {} },
        "chromatic-text-reveal" => {
            rsx! { pages::chromatic_text_reveal::ChromaticTextRevealPage {} }
        }
        "preview-rail" => rsx! { pages::preview_rail::PreviewRailPage {} },
        "aspect-ratio" => rsx! { pages::aspect_ratio::AspectRatioPage {} },
        "notification-stack" => rsx! { pages::notification_stack::NotificationStackPage {} },
        "bounce-sidebar" => rsx! { pages::bounce_sidebar::BounceSidebarPage {} },
        "bloom-menu" => rsx! { pages::bloom_menu::BloomMenuPage {} },
        "expanding-arrow-button" => {
            rsx! { pages::expanding_arrow_button::ExpandingArrowButtonPage {} }
        }
        "slide-action-button" => rsx! { pages::slide_action_button::SlideActionButtonPage {} },
        "hold-action-button" => rsx! { pages::hold_action_button::HoldActionButtonPage {} },
        "expandable-control" => rsx! { pages::expandable_control::ExpandableControlPage {} },
        "overflow-actions" => rsx! { pages::overflow_actions::OverflowActionsPage {} },
        "smooth-scroll" => rsx! { pages::scroll_to::ScrollToPage {} },
        "card-folder" => rsx! { pages::card_folder::CardFolderPage {} },
        "dynamic-island" => rsx! { pages::dynamic_island::DynamicIslandPage {} },
        "file-tree" => rsx! { pages::file_tree::FileTreePage {} },
        "expandable-action-bar" => {
            rsx! { pages::expandable_action_bar::ExpandableActionBarPage {} }
        }
        "swipeable-list" => rsx! { pages::swipeable_list::SwipeableListPage {} },
        "project-folder" => rsx! { pages::project_folder::ProjectFolderPage {} },
        "pull-to-refresh" => rsx! { pages::pull_to_refresh::PullToRefreshPage {} },
        "infinite-masonry" => rsx! { pages::infinite_masonry::InfiniteMasonryPage {} },
        "not-found-stacked" => rsx! { pages::not_found::NotFoundStackedPage {} },
        "not-found-terminal" => rsx! { pages::not_found::NotFoundTerminalPage {} },
        "not-found-glitch" => rsx! { pages::not_found::NotFoundGlitchPage {} },
        "not-found-spotlight" => rsx! { pages::not_found::NotFoundSpotlightPage {} },
        "not-found-magnetic" => rsx! { pages::not_found::NotFoundMagneticPage {} },
        "range-slider-wave" => rsx! { pages::range_slider_wave::RangeSliderWavePage {} },
        "prediction-market-card" => {
            rsx! { pages::prediction_market_card::PredictionMarketCardPage {} }
        }
        "sortable-list" => rsx! { pages::sortable_list::SortableListPage {} },
        "sortable-stack" => rsx! { pages::sortable_list::SortableListPage {} },
        "liquid" => rsx! { pages::liquid::LiquidPage {} },
        "cylinder-carousel" => rsx! { pages::cylinder_carousel::CylinderCarouselPage {} },
        "feedback-widget" => rsx! { pages::feedback_widget::FeedbackWidgetPage {} },
        "signup-form" => rsx! { pages::signup_form::SignUpFormPage {} },
        "morphing-search-chips" => {
            rsx! { pages::morphing_search_chips::MorphingSearchChipsPage {} }
        }
        "prediction-market" => rsx! { pages::prediction_market::PredictionMarketPage {} },
        "morphing-search" => rsx! { pages::morphing_search::MorphingSearchPage {} },
        "file-upload" => rsx! { pages::file_upload::FileUploadPage {} },
        "knockout-bracket" => rsx! { pages::knockout_bracket::KnockoutBracketPage {} },
        "wallet-card" => rsx! { pages::wallet_card::WalletCardPage {} },
        "swap" => rsx! { pages::swap::SwapPage {} },
        "knockout-wheel" => rsx! { pages::knockout_wheel::KnockoutWheelPage {} },
        "table" => rsx! { pages::table::TablePage {} },
        "table-editable" => rsx! { pages::table::TableEditablePage {} },
        "table-async" => rsx! { pages::table::TableAsyncPage {} },
        "availability-scheduler" => {
            rsx! { pages::availability_scheduler::AvailabilitySchedulerPage {} }
        }
        "morphing-lightbox" => rsx! { pages::morphing_lightbox::MorphingLightboxPage {} },
        other => rsx! {
            div { class: "p-6 text-sm text-muted-foreground", "Unknown page: {other}" }
        },
    }
}

/// The shell's default page — what `/` shows, and the slug the pre-router
/// signal also started on.
const DEFAULT_PAGE: &str = "foundation";

/// One URL per gallery page.
///
/// The `[PAGES]` slug is the path segment, so `/component-alert`,
/// `/cat-motion`, `/site-home` and `/docs-theme` are all addressable, linkable
/// and survive a refresh. This mirrors the gallery's own slug vocabulary rather
/// than the source site's `/components/<category>/<slug>` scheme.
///
/// `/:..route` is the catch-all: a multi-segment path has no gallery page, so it
/// renders the same `Unknown page: …` notice `render` uses.
#[derive(Clone, Debug, PartialEq, Routable)]
pub(crate) enum Route {
    #[route("/")]
    Home {},
    #[route("/:slug")]
    Page { slug: String },
    #[route("/:..route")]
    Unknown { route: Vec<String> },
}

#[component]
fn App() -> Element {
    // `next-themes` (`attribute="class"`, `defaultTheme="dark"`, `enableSystem`)
    // and the color theme. These sit above the router so a navigation does not
    // re-create them.
    let _dark = beui_theme::use_dark_mode_provider();
    let _color = beui_theme::use_color_theme_provider();

    // The `@font-face` block and the eleven `data-theme` palettes, injected once
    // into `<head>` — the same stylesheets beUI injects at runtime.
    use_effect(beui_theme::install_theme);

    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        Shell { slug: DEFAULT_PAGE.to_string() }
    }
}

#[component]
fn Page(slug: String) -> Element {
    rsx! {
        Shell { slug }
    }
}

#[component]
fn Unknown(route: Vec<String>) -> Element {
    rsx! {
        Shell { slug: route.join("/") }
    }
}

/// The gallery shell: the left navigation and the page body. `slug` comes from
/// the URL.
#[component]
fn Shell(slug: String) -> Element {
    rsx! {
        div { class: "flex min-h-dvh flex-col bg-background font-sans text-foreground md:flex-row",
            nav::NavSidebar { active: slug.clone() }
            // `min-w-0` keeps a wide page from pushing the sidebar off the screen.
            main { class: "min-w-0 flex-1",
                div { "data-page": "{slug}", {render(&slug)} }
            }
        }
    }
}

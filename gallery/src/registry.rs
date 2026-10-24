//! The website registry — categories and their components, in source order.
//!
//! Generated from `../ui-components/lib/registry.ts` by
//! `scripts/gen-registry.mjs`; regenerate rather than edit. Only the fields
//! the navigation needs are carried over — descriptions, file paths, keywords
//! and guides stay in the source.

/// One component, as the navigation names it.
#[derive(Clone, Copy, PartialEq)]
pub struct SiteComponent {
    pub slug: &'static str,
    pub name: &'static str,
}

/// One registry category.
pub struct SiteCategory {
    pub name: &'static str,
    pub components: &'static [SiteComponent],
}

/// `registry` from `lib/registry.ts`, verbatim order and copy.
pub static REGISTRY: &[SiteCategory] = &[
    SiteCategory {
        name: "Components",
        components: &[
            SiteComponent {
                slug: "alert",
                name: "Animated Alert",
            },
            SiteComponent {
                slug: "collapsible",
                name: "Animated Collapsible",
            },
            SiteComponent {
                slug: "aspect-ratio",
                name: "Aspect Ratio",
            },
            SiteComponent {
                slug: "arc-picker",
                name: "Arc Picker",
            },
            SiteComponent {
                slug: "sortable-stack",
                name: "Sortable List",
            },
            SiteComponent {
                slug: "image-viewer",
                name: "Image Viewer",
            },
            SiteComponent {
                slug: "date-range-picker",
                name: "Date Range Picker",
            },
            SiteComponent {
                slug: "card",
                name: "Card",
            },
            SiteComponent {
                slug: "color-picker",
                name: "Color Picker",
            },
            SiteComponent {
                slug: "color-selector",
                name: "Animated Color Selector",
            },
            SiteComponent {
                slug: "breadcrumb",
                name: "Animated Breadcrumb",
            },
            SiteComponent {
                slug: "tilt-card",
                name: "Tilt Card",
            },
            SiteComponent {
                slug: "button",
                name: "Button",
            },
            SiteComponent {
                slug: "expanding-arrow-button",
                name: "Animated CTA Buttons",
            },
            SiteComponent {
                slug: "expandable-control",
                name: "Expandable Control",
            },
            SiteComponent {
                slug: "adaptive-stepper",
                name: "Adaptive Stepper",
            },
            SiteComponent {
                slug: "marquee",
                name: "Marquee",
            },
            SiteComponent {
                slug: "tabs",
                name: "Tabs",
            },
            SiteComponent {
                slug: "switch",
                name: "Switch",
            },
            SiteComponent {
                slug: "input",
                name: "Input",
            },
            SiteComponent {
                slug: "select",
                name: "Select",
            },
            SiteComponent {
                slug: "combobox",
                name: "Combobox",
            },
            SiteComponent {
                slug: "multi-select",
                name: "Multi Select",
            },
            SiteComponent {
                slug: "checkbox",
                name: "Checkbox",
            },
            SiteComponent {
                slug: "radio",
                name: "Radio Group",
            },
            SiteComponent {
                slug: "bottom-sheet",
                name: "Bottom Sheet",
            },
            SiteComponent {
                slug: "pull-to-refresh",
                name: "Pull to Refresh",
            },
            SiteComponent {
                slug: "shared-layout-bg",
                name: "Shared Layout Background",
            },
            SiteComponent {
                slug: "bounce-sidebar",
                name: "Bounce Sidebar",
            },
            SiteComponent {
                slug: "animated-sidebar",
                name: "Animated Sidebar",
            },
            SiteComponent {
                slug: "file-tree",
                name: "File Tree",
            },
            SiteComponent {
                slug: "preview-rail",
                name: "Preview Rail",
            },
            SiteComponent {
                slug: "dock",
                name: "Dock",
            },
            SiteComponent {
                slug: "tooltip",
                name: "Tooltip",
            },
            SiteComponent {
                slug: "context-menu",
                name: "Animated Context Menu",
            },
            SiteComponent {
                slug: "popover",
                name: "Popover",
            },
            SiteComponent {
                slug: "morphing-modal",
                name: "Morphing Modal",
            },
            SiteComponent {
                slug: "center-morph-modal",
                name: "Center Morph Modal",
            },
            SiteComponent {
                slug: "text-animation",
                name: "Text Animation",
            },
            SiteComponent {
                slug: "number",
                name: "Number Animation",
            },
            SiteComponent {
                slug: "animated-badge",
                name: "Animated Badge",
            },
            SiteComponent {
                slug: "action-swap",
                name: "Action Swap",
            },
            SiteComponent {
                slug: "animated-toast-stack",
                name: "Animated Toast Stack",
            },
            SiteComponent {
                slug: "theme-toggle",
                name: "Theme Toggle",
            },
            SiteComponent {
                slug: "bouncy-accordion",
                name: "Bouncy Accordion",
            },
            SiteComponent {
                slug: "drawer",
                name: "Drawer",
            },
            SiteComponent {
                slug: "scroll-animation",
                name: "Scroll Animation",
            },
            SiteComponent {
                slug: "range-slider",
                name: "Range Slider",
            },
            SiteComponent {
                slug: "wheel-picker",
                name: "Wheel Picker",
            },
            SiteComponent {
                slug: "table",
                name: "Table",
            },
            SiteComponent {
                slug: "shader-background",
                name: "Shader Background",
            },
            SiteComponent {
                slug: "cylinder-carousel",
                name: "Cylinder Carousel",
            },
            SiteComponent {
                slug: "loader",
                name: "Loader",
            },
        ],
    },
    SiteCategory {
        name: "Charts",
        components: &[
            SiteComponent {
                slug: "treemap",
                name: "Treemap",
            },
            SiteComponent {
                slug: "candlestick-chart",
                name: "Candlestick Chart",
            },
            SiteComponent {
                slug: "donut-chart",
                name: "Donut Chart",
            },
            SiteComponent {
                slug: "volume-profile",
                name: "Volume Profile",
            },
            SiteComponent {
                slug: "status-bar",
                name: "Status Bar",
            },
            SiteComponent {
                slug: "composition-chart",
                name: "Composition Chart",
            },
            SiteComponent {
                slug: "funnel-chart",
                name: "Funnel Chart",
            },
            SiteComponent {
                slug: "liquidity-heatmap",
                name: "Liquidity Heatmap",
            },
            SiteComponent {
                slug: "bump-chart",
                name: "Bump Chart",
            },
            SiteComponent {
                slug: "order-book",
                name: "Order Book",
            },
            SiteComponent {
                slug: "heat-calendar",
                name: "Heat Calendar",
            },
            SiteComponent {
                slug: "returns-calendar",
                name: "Returns Calendar",
            },
            SiteComponent {
                slug: "price-target-fan",
                name: "Price Target Fan",
            },
        ],
    },
    SiteCategory {
        name: "AI Agents",
        components: &[
            SiteComponent {
                slug: "voice-orb",
                name: "Voice Orb",
            },
            SiteComponent {
                slug: "message-bubble",
                name: "Message Bubble",
            },
            SiteComponent {
                slug: "message",
                name: "Message",
            },
            SiteComponent {
                slug: "message-scroller",
                name: "Message Scroller",
            },
            SiteComponent {
                slug: "prompt-input",
                name: "Prompt Input",
            },
            SiteComponent {
                slug: "todo-list",
                name: "Todo List",
            },
            SiteComponent {
                slug: "code-block",
                name: "Code Block",
            },
            SiteComponent {
                slug: "approval-card",
                name: "Approval Card",
            },
            SiteComponent {
                slug: "file-diff",
                name: "File Diff",
            },
            SiteComponent {
                slug: "tool-result",
                name: "Tool Result",
            },
            SiteComponent {
                slug: "streaming-response",
                name: "Streaming Response",
            },
            SiteComponent {
                slug: "image-generation",
                name: "Image Generation",
            },
            SiteComponent {
                slug: "tool-approval",
                name: "Tool Approval",
            },
            SiteComponent {
                slug: "citations",
                name: "Citations",
            },
            SiteComponent {
                slug: "agent-activity",
                name: "Agent Activity",
            },
            SiteComponent {
                slug: "loading-states",
                name: "Agent Loading States",
            },
            SiteComponent {
                slug: "ai-sidebar",
                name: "AI Sidebar",
            },
            SiteComponent {
                slug: "chat-app",
                name: "Chat App",
            },
        ],
    },
    SiteCategory {
        name: "Blocks",
        components: &[
            SiteComponent {
                slug: "infinite-masonry",
                name: "Infinite Masonry",
            },
            SiteComponent {
                slug: "notification-stack",
                name: "Notification Stack",
            },
            SiteComponent {
                slug: "project-folder",
                name: "Project Folder",
            },
            SiteComponent {
                slug: "card-folder",
                name: "Card Folder",
            },
            SiteComponent {
                slug: "knockout-bracket",
                name: "Fixtures",
            },
            SiteComponent {
                slug: "availability-scheduler",
                name: "Availability Scheduler",
            },
            SiteComponent {
                slug: "swap",
                name: "Multi-chain Swap",
            },
            SiteComponent {
                slug: "dynamic-island",
                name: "Dynamic Island",
            },
            SiteComponent {
                slug: "command-palette",
                name: "Command Palette",
            },
            SiteComponent {
                slug: "morphing-search",
                name: "Morphing Search",
            },
            SiteComponent {
                slug: "expandable-action-bar",
                name: "Expandable Action Bar",
            },
            SiteComponent {
                slug: "overflow-actions",
                name: "Overflow Actions",
            },
            SiteComponent {
                slug: "expandable-tabs",
                name: "Expandable Tabs",
            },
            SiteComponent {
                slug: "morphing-tabs",
                name: "Morphing Tabs",
            },
            SiteComponent {
                slug: "swipeable-list",
                name: "Swipeable List",
            },
            SiteComponent {
                slug: "file-upload",
                name: "File Upload",
            },
            SiteComponent {
                slug: "prediction-market",
                name: "Prediction Market",
            },
            SiteComponent {
                slug: "wallet-card",
                name: "Wallet Card",
            },
            SiteComponent {
                slug: "otp-input",
                name: "OTP Input",
            },
            SiteComponent {
                slug: "signup-form",
                name: "Sign Up Form",
            },
            SiteComponent {
                slug: "bloom-menu",
                name: "Bloom Menu",
            },
            SiteComponent {
                slug: "feedback-widget",
                name: "Feedback Widget",
            },
            SiteComponent {
                slug: "not-found",
                name: "404 / Not Found",
            },
        ],
    },
];

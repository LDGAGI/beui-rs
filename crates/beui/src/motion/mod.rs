//! The motion components, one module per beUI file.
//!
//! Layout mirrors `components/motion/` in the source: a flat module per
//! component (or per component family), re-exported from the crate root so the
//! gallery and downstream apps can `use beui::prelude::*`.

pub mod alert;
pub mod animated_sidebar;
pub mod animated_toast_stack;
pub mod bottom_sheet;
pub mod bouncy_accordion;
pub mod breadcrumb;
pub mod center_morph_modal;
pub mod collapsible;
pub mod combobox;
pub mod command_palette;
pub mod context_menu;
pub mod dock;
pub mod drawer;
pub mod expandable_tabs;
// Lane-a's shared animation helpers (springs, tweens, keyframe tracks, style
// motions). They live here rather than in a sibling `motion.rs` so both lanes
// resolve to the same `motion` module after the merge.
pub mod helpers;
pub mod morph_popover;
pub mod morph_select;
pub mod morphing_modal;
pub mod morphing_tabs;
pub mod multi_select;
pub mod popover;
pub mod select;
pub mod spring;
pub mod tabs;
pub mod tooltip;

pub use alert::{
    use_alert, Alert, AlertAction, AlertClose, AlertContent, AlertDescription, AlertIcon,
    AlertState, AlertTitle, AlertVariant,
};
pub use animated_sidebar::{
    use_animated_sidebar, AnimatedSidebar, AnimatedSidebarApi, AnimatedSidebarClose,
    AnimatedSidebarCollapsible, AnimatedSidebarContent, AnimatedSidebarFooter,
    AnimatedSidebarGroup, AnimatedSidebarGroupContent, AnimatedSidebarGroupLabel,
    AnimatedSidebarHeader, AnimatedSidebarInset, AnimatedSidebarMenu, AnimatedSidebarMenuButton,
    AnimatedSidebarMenuItem, AnimatedSidebarMenuSub, AnimatedSidebarMenuSubButton,
    AnimatedSidebarMenuSubItem, AnimatedSidebarProvider, AnimatedSidebarRail, AnimatedSidebarSide,
    AnimatedSidebarTrigger, AnimatedSidebarVariant,
};
pub use animated_toast_stack::{
    use_animated_toast_stack, AnimatedToast, AnimatedToastPatch, AnimatedToastStack,
    AnimatedToastStackHandle, AnimatedToastStackOptions, ToastClassNames, ToastPlacement,
    ToastPosition, ToastStatus,
};
pub use bottom_sheet::{BottomSheet, SnapPoint};
pub use bouncy_accordion::{BouncyAccordion, BouncyAccordionItem};
pub use breadcrumb::{
    Breadcrumb, BreadcrumbEllipsis, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage,
    BreadcrumbSeparator,
};
pub use center_morph_modal::{
    CenterMorphModal, CenterMorphModalClose, CenterMorphModalContent, CenterMorphModalTrigger,
};
pub use collapsible::{
    use_collapsible, Collapsible, CollapsibleContent, CollapsibleIndicator, CollapsibleState,
    CollapsibleTrigger,
};
pub use combobox::{
    default_combobox_filter, Combobox, ComboboxAlign, ComboboxContent, ComboboxEmpty,
    ComboboxFilter, ComboboxGroup, ComboboxInput, ComboboxItem, ComboboxLabel, ComboboxList,
    ComboboxSeparator, ComboboxSide, ComboboxTrigger, ComboboxValue, MoveDir,
};
pub use command_palette::{search_commands, CommandItem, CommandPalette};
pub use context_menu::{
    ContextMenu, ContextMenuCheckboxItem, ContextMenuContent, ContextMenuItem, ContextMenuLabel,
    ContextMenuModality, ContextMenuPoint, ContextMenuRadioGroup, ContextMenuRadioItem,
    ContextMenuSeparator, ContextMenuShortcut, ContextMenuTrigger,
};
pub use dock::{Dock, DockItem, DockSeparator};
pub use expandable_tabs::{ExpandableTabs, ExpandableTabsIcon, ExpandableTabsItem};
pub use morph_popover::{
    MorphPopover, MorphPopoverAlign, MorphPopoverContent, MorphPopoverSide, MorphPopoverTrigger,
};
pub use morph_select::{
    MorphSelect, MorphSelectContent, MorphSelectItem, MorphSelectTrigger, MorphSelectValue,
};
pub use morphing_modal::{MorphingModal, MorphingModalPlacement};
pub use morphing_tabs::{MorphingTabs, MorphingTabsItem};
pub use multi_select::{
    default_multi_select_filter, MultiSelect, MultiSelectAlign, MultiSelectContent,
    MultiSelectEmpty, MultiSelectFilter, MultiSelectFilterFn, MultiSelectGroup, MultiSelectInput,
    MultiSelectItem, MultiSelectLabel, MultiSelectList, MultiSelectSeparator, MultiSelectSide,
    MultiSelectTrigger, MultiSelectValue,
};
pub use popover::{
    build_geo, clip_for_progress, trigger_cutout, Popover, PopoverAlign, PopoverContent,
    PopoverGeo, PopoverSide, PopoverTrigger, PopoverTriggerMode, Rect,
};
pub use select::{Select, SelectContent, SelectItem, SelectPlacement, SelectTrigger, SelectValue};
pub use spring::spring_from_visual_duration;
// Re-export the lane-a animation helpers from the `motion` module so
// `crate::motion::{spring, tween, motion_style, ...}` keeps working.
pub use drawer::{Drawer, DrawerSide};
pub use helpers::*;
pub use tabs::{Tabs, TabsContent, TabsList, TabsTrigger, TabsVariant};
pub use tooltip::{
    Tooltip, TooltipAlign, TooltipContent, TooltipProvider, TooltipSide, TooltipTiming,
    TooltipTrigger,
};

use dioxus::prelude::*;

/// Whether a `children: Element` was left empty in the caller's RSX.
///
/// Dioxus has no `undefined`: a component that omits its children still
/// receives an `Element`, a one-root placeholder fragment. beUI's
/// `children ?? <Default/>` idiom therefore has to test the shape, the same
/// test `dioxus-motion`'s presence boundary uses to skip placeholders. The
/// fields it reads (`key`, `template.roots`, `dynamic_root`) are part of the
/// public `VNode` API.
pub(crate) fn is_empty_children(children: &Element) -> bool {
    let Ok(vnode) = children else {
        return false;
    };
    vnode.key.is_none()
        && vnode.template.roots.len() == 1
        && matches!(
            vnode.dynamic_root(0),
            Some(dioxus_core::DynamicNode::Placeholder(_))
        )
}

/// Renders `children` when the caller supplied any, and `default` otherwise —
/// the Rust spelling of `children ?? <Default/>`.
#[component]
pub(crate) fn Slot(children: Element, default: Element) -> Element {
    if is_empty_children(&children) {
        rsx! { {default} }
    } else {
        rsx! { {children} }
    }
}

/// A `display: contents` container that is moved under the app's event root.
///
/// This is the portal an overlay must use when its layers live inside an
/// `AnimatePresence`, and it differs from `beui-core::portal` in two ways for
/// reasons that are both Dioxus-internal:
///
/// - `dioxus-motion` wraps every presence child in two
///   `data-dioxus-motion-layout-*` divs. Moving a layer itself to `<body>` puts
///   it outside the wrapper presence later removes, so the exit finishes and the
///   node is orphaned in `body` forever. Portaling a container *outside* the
///   boundary keeps the layers inside it, and presence removes them normally.
/// - Dioxus dispatches bubbling events from a single listener on the app's
///   mount root. A node under `<body>` is a sibling of that root, so its
///   `onpointerdown`/`onclick` never fire. The container is appended to the
///   topmost ancestor under `<body>` — the event root — so handlers keep
///   working while still escaping every ancestor *inside* the app (transforms,
///   filters, stacking contexts).
///
/// `display: contents` means the container generates no box, so it neither
/// affects layout nor gives `position: fixed` descendants a new containing
/// block.
#[component]
pub fn PortalHost(children: Element) -> Element {
    let id = use_hook(beui_core::platform::next_scope_id);
    let selector = format!("[data-beui-portal-host=\"{id}\"]");
    let script = format!(
        "(() => {{ const el = document.querySelector({selector}); \
         if (!el) return; \
         let root = el; while (root.parentElement && root.parentElement !== document.body) root = root.parentElement; \
         if (root.parentElement !== document.body) root = document.body; \
         if (el.parentElement !== root) {{ el.setAttribute('data-beui-portaled', ''); root.appendChild(el); }} }})()",
        selector = beui_core::platform::js_string(&selector),
    );
    rsx! {
        div {
            style: "display:contents",
            "data-beui-portal-host": "{id}",
            onmounted: move |_| beui_core::platform::fire(&script),
            {children}
        }
    }
}

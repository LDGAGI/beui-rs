//! beUI motion components for Dioxus 0.7.
//!
//! One module per `components/motion/*` file in the beUI source, same public
//! names, same DOM, same class strings. Everything a component needs from the
//! shared foundation is re-exported through [`prelude`].
//!
//! ```ignore
//! use beui::prelude::*;
//! ```

pub mod icons;
pub mod motion;

pub use beui_core::cn;
pub use beui_core::ease;
pub use beui_core::touch;
pub use beui_core::Class;
pub use beui_theme as theme;

pub mod action_swap;
pub mod adaptive_stepper;
pub mod animated_badge;
pub mod animated_number;
pub mod arc_picker;
pub mod aspect_ratio;
pub mod attachment_upload;
pub mod availability_scheduler;
pub mod bloom_menu;
pub mod bounce_sidebar;
pub mod button;
pub mod button_extra;
pub mod card;
pub mod card_folder;
pub mod checkbox;
pub mod chromatic_text_reveal;
pub mod color_picker;
pub mod color_selector;
pub mod cylinder_carousel;
pub mod date_range_picker;
pub mod digit_swap;
pub mod dynamic_island;
pub mod expandable_action_bar;
pub mod expandable_control;
pub mod expanding_arrow_button;
pub mod feedback_widget;
pub mod file_tree;
pub mod file_upload;
pub mod hold_action_button;
pub mod infinite_masonry;
pub mod input;
pub mod knockout_bracket;
pub mod knockout_wheel;
pub mod liquid;
pub mod loader;
pub mod magnetic;
pub mod marquee;
pub mod morphing_lightbox;
pub mod morphing_search;
pub mod morphing_search_chips;
pub mod not_found;
pub mod notification_stack;
pub mod number_ticker;
pub mod otp_input;
pub mod overflow_actions;
pub mod parallax;
pub mod prediction_market;
pub mod prediction_market_card;
pub mod preview_rail;
pub mod project_folder;
pub mod pull_to_refresh;
pub mod radio;
pub mod range_slider;
pub mod range_slider_variants;
pub mod rect;
pub mod scroll;
pub mod scroll_progress;
pub mod scroll_reveal;
pub mod shared_layout_bg;
pub mod signup_form;
pub mod slide_action_button;
pub mod smooth_scroll;
pub mod sortable_list;
pub mod spring;
pub mod swap;
pub mod swipeable_list;
pub mod switch;
pub mod table;
pub mod text;
pub mod text_cascade;
pub mod text_reveal;
pub mod text_scramble;
pub mod text_shimmer;
pub mod theme_toggle;
pub mod tilt_card;
pub mod wallet_card;
pub mod wheel_picker;

pub use icons::{
    ArrowDownIcon, ArrowLeftIcon, ArrowRightIcon, ArrowUpIcon, ArrowUpRightIcon, BanIcon, BellIcon,
    BlocksIcon, BotIcon, Building2Icon, CalendarIcon, CheckIcon, ChevronDownIcon, ChevronRightIcon,
    ChevronsUpDownIcon, CircleAlertIcon, CircleCheckIcon, CircleIcon, CircleUserRoundIcon,
    CommandIcon, CopyIcon, DownloadIcon, EllipsisIcon, EyeIcon, FolderIcon, GithubIcon, HomeIcon,
    InboxIcon, InfoIcon, LayoutGridIcon, ListTodoIcon, LoaderCircleIcon, LockIcon, MailIcon,
    MenuIcon, MonitorIcon, MoonIcon, MusicIcon, NotebookTabsIcon, PanelLeftIcon, PencilIcon,
    ScanFaceIcon, ScrollTextIcon, SearchIcon, SettingsIcon, ShapesIcon, Share2Icon,
    ShieldCheckIcon, SparklesIcon, SunIcon, SwatchBookIcon, TargetIcon, Trash2Icon,
    TriangleAlertIcon, WorkflowIcon, XIcon,
};
pub use motion::{
    default_combobox_filter, default_multi_select_filter, search_commands, use_alert,
    use_animated_sidebar, use_animated_toast_stack, use_collapsible, Alert, AlertAction,
    AlertClose, AlertContent, AlertDescription, AlertIcon, AlertState, AlertTitle, AlertVariant,
    AnimatedSidebar, AnimatedSidebarApi, AnimatedSidebarClose, AnimatedSidebarCollapsible,
    AnimatedSidebarContent, AnimatedSidebarFooter, AnimatedSidebarGroup,
    AnimatedSidebarGroupContent, AnimatedSidebarGroupLabel, AnimatedSidebarHeader,
    AnimatedSidebarInset, AnimatedSidebarMenu, AnimatedSidebarMenuButton, AnimatedSidebarMenuItem,
    AnimatedSidebarMenuSub, AnimatedSidebarMenuSubButton, AnimatedSidebarMenuSubItem,
    AnimatedSidebarProvider, AnimatedSidebarRail, AnimatedSidebarSide, AnimatedSidebarTrigger,
    AnimatedSidebarVariant, AnimatedToast, AnimatedToastPatch, AnimatedToastStack,
    AnimatedToastStackHandle, AnimatedToastStackOptions, BottomSheet, BouncyAccordion,
    BouncyAccordionItem, Breadcrumb, BreadcrumbEllipsis, BreadcrumbItem, BreadcrumbLink,
    BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator, CenterMorphModal, CenterMorphModalClose,
    CenterMorphModalContent, CenterMorphModalTrigger, Collapsible, CollapsibleContent,
    CollapsibleIndicator, CollapsibleState, CollapsibleTrigger, Combobox, ComboboxAlign,
    ComboboxContent, ComboboxEmpty, ComboboxFilter, ComboboxGroup, ComboboxInput, ComboboxItem,
    ComboboxLabel, ComboboxList, ComboboxSeparator, ComboboxSide, ComboboxTrigger, ComboboxValue,
    CommandItem, CommandPalette, ContextMenu, ContextMenuCheckboxItem, ContextMenuContent,
    ContextMenuItem, ContextMenuLabel, ContextMenuModality, ContextMenuPoint,
    ContextMenuRadioGroup, ContextMenuRadioItem, ContextMenuSeparator, ContextMenuShortcut,
    ContextMenuTrigger, Dock, DockItem, DockSeparator, Drawer, DrawerSide, ExpandableTabs,
    ExpandableTabsIcon, ExpandableTabsItem, MorphPopover, MorphPopoverAlign, MorphPopoverContent,
    MorphPopoverSide, MorphPopoverTrigger, MorphSelect, MorphSelectContent, MorphSelectItem,
    MorphSelectTrigger, MorphSelectValue, MorphingModal, MorphingModalPlacement, MorphingTabs,
    MorphingTabsItem, MoveDir, MultiSelect, MultiSelectAlign, MultiSelectContent, MultiSelectEmpty,
    MultiSelectFilter, MultiSelectFilterFn, MultiSelectGroup, MultiSelectInput, MultiSelectItem,
    MultiSelectLabel, MultiSelectList, MultiSelectSeparator, MultiSelectSide, MultiSelectTrigger,
    MultiSelectValue, Popover, PopoverAlign, PopoverContent, PopoverSide, PopoverTrigger,
    PopoverTriggerMode, PortalHost, Select, SelectContent, SelectItem, SelectPlacement,
    SelectTrigger, SelectValue, SnapPoint, Tabs, TabsContent, TabsList, TabsTrigger, TabsVariant,
    ToastClassNames, ToastPlacement, ToastPosition, ToastStatus, Tooltip, TooltipAlign,
    TooltipContent, TooltipProvider, TooltipSide, TooltipTiming, TooltipTrigger,
};

pub use action_swap::{
    ActionSwapAnimation, ActionSwapButton, ActionSwapButtonProps, ActionSwapButtonSize,
    ActionSwapButtonVariant, ActionSwapIcon, ActionSwapIconProps, ActionSwapItem, ActionSwapText,
    ActionSwapTextProps,
};
pub use animated_number::{locale_integer, AnimatedNumber, AnimatedNumberProps};
pub use arc_picker::{ArcPicker, ArcPickerOption, ArcPickerProps, ArcPickerSide};
pub use aspect_ratio::{
    AspectRatio, AspectRatioContext, AspectRatioImage, AspectRatioImageProps, AspectRatioProps,
};
pub use attachment_upload::{
    format_bytes as attachment_format_bytes, format_duration as attachment_format_duration,
    format_max_size as attachment_format_max_size, infer_kind as attachment_infer_kind,
    AttachmentRejectReason, AttachmentUpload, AttachmentUploadItem, AttachmentUploadKind,
    AttachmentUploadProps, AttachmentUploadStatus,
};
pub use availability_scheduler::{
    build_options, clamp_range, default_week, end_options, label12, panel_key, start_options,
    to_minutes, to_value, AvailabilityScheduler, DayAvailability, DayKey, RangeEdge, TimeOption,
    TimeRange, WeekAvailability,
};
pub use bloom_menu::{BloomMenu, BloomMenuItem, BloomMenuProps};
pub use bounce_sidebar::{BounceSidebar, BounceSidebarItem, BounceSidebarProps};
pub use card_folder::{CardFolder, CardFolderProps};
pub use chromatic_text_reveal::{ChromaticTextReveal, ChromaticTextRevealProps, CHROMATIC_PALETTE};
pub use cylinder_carousel::{CylinderCarousel, CylinderCarouselProps, CylinderVariant};
pub use digit_swap::{DigitSwap, DigitSwapDirection, DigitSwapProps};
pub use dynamic_island::{
    DynamicIsland, DynamicIslandProps, DynamicIslandView, DynamicIslandViewProps,
};
pub use expandable_action_bar::{
    ExpandableActionBar, ExpandableActionBarClassNames, ExpandableActionBarItem,
    ExpandableActionBarProps, ExpandableActionBarSize,
};
pub use expandable_control::{
    ExpandableButton, ExpandableButtonProps, ExpandableChip, ExpandableChipProps,
};
pub use expanding_arrow_button::{ExpandingArrowButton, ExpandingArrowButtonProps};
pub use feedback_widget::{FeedbackPosition, FeedbackStatus, FeedbackWidget, FeedbackWidgetProps};
pub use file_tree::{FileTree, FileTreeClassNames, FileTreeItem, FileTreeKind, FileTreeProps};
pub use file_upload::{
    FileUpload, FileUploadAdded, FileUploadItem, FileUploadProps, FileUploadStatus,
    FileUploadVariant,
};
pub use hold_action_button::{HoldActionButton, HoldActionButtonProps, HoldActionType};
pub use infinite_masonry::{InfiniteMasonry, InfiniteMasonryProps, MasonryItem};
pub use knockout_bracket::{
    KnockoutBracket, KnockoutBracketProps, Match, MatchSide, MatchStatus, Round as BracketRound,
    Side as BracketSide, Team,
};
pub use knockout_wheel::{KnockoutWheel, KnockoutWheelProps};
pub use liquid::{Liquid, LiquidBox, LiquidItem, LiquidItemProps, LiquidProps};
pub use magnetic::{Magnetic, MagneticProps};
pub use marquee::{Marquee, MarqueeDirection, MarqueeProps};
pub use morphing_lightbox::{
    use_image_viewer, validate_images, ImageViewer, ImageViewerApi, ImageViewerCaption,
    ImageViewerClose, ImageViewerContent, ImageViewerCounter, ImageViewerGallery, ImageViewerNext,
    ImageViewerPrevious, ImageViewerThumbnail, LightboxImage, MorphingLightbox,
    CONTROL_CLASS as LIGHTBOX_CONTROL_CLASS,
};
pub use morphing_search::{MorphingSearch, MorphingSearchItem, MorphingSearchProps};
pub use morphing_search_chips::{MorphingSearchChips, MorphingSearchChipsProps, SearchSide};
pub use not_found::{
    NotFound, NotFoundActions, NotFoundActionsProps, NotFoundProps, NotFoundStage,
    NotFoundStageProps, NotFoundVariant,
};
pub use notification_stack::{
    NotificationStack, NotificationStackClassNames, NotificationStackItem, NotificationStackProps,
};
pub use number_ticker::{NumberTicker, NumberTickerProps};
pub use overflow_actions::{
    OverflowActionItem, OverflowActions, OverflowActionsClassNames, OverflowActionsProps,
    OverflowActionsSize,
};
pub use parallax::{Parallax, ParallaxAxis, ParallaxProps};
pub use prediction_market::{
    PredictionMarket, PredictionMarketMode, PredictionMarketOrderValue, PredictionMarketOutcome,
    PredictionMarketProps, PredictionMarketQuote, TradeStatus,
};
pub use prediction_market_card::{
    MarketSide, PredictionMarketCard, PredictionMarketCardOutcome, PredictionMarketCardProps,
    PredictionMarketSelection,
};
pub use preview_rail::{
    PreviewRail, PreviewRailItem, PreviewRailOrientation, PreviewRailProps, PreviewSide,
};
pub use project_folder::{ProjectFolder, ProjectFolderPreview, ProjectFolderProps};
pub use pull_to_refresh::{PullToRefresh, PullToRefreshProps, PullToRefreshStatus};
pub use range_slider_variants::{BubbleSlider, FluidSlider, InlineSlider, RulerSlider, WaveSlider};
pub use rect::{use_element_rect, use_element_size, use_element_width, ElementRect, ElementSize};
pub use scroll::{
    use_in_view, use_parallax_progress, use_scroll_y_progress, use_spring_value, InView,
    InViewAmount, InViewOptions, ParallaxProgress,
};
pub use scroll_progress::{
    ScrollProgress, ScrollProgressPosition, ScrollProgressProps, ScrollProgressVariant,
};
pub use scroll_reveal::{ScrollReveal, ScrollRevealProps};
pub use shared_layout_bg::{
    SharedLayoutBg, SharedLayoutBgProps, SharedLayoutItem, SharedLayoutTag,
};
pub use signup_form::{password_strength, SignUpForm, SignUpFormProps, SignUpStatus, SignUpValues};
pub use slide_action_button::{SlideActionButton, SlideActionButtonProps};
pub use smooth_scroll::{
    use_scroll_state, use_smooth_scroll, ScrollTo, ScrollToProps, SmoothScroll, SmoothScrollApi,
    SmoothScrollProps,
};
pub use sortable_list::{
    SortableItem, SortableList, SortableList as SortableStack, SortableListProps,
    SortableListProps as SortableStackProps,
};
pub use spring::{duration_spring, spring_from_duration, spring_from_visual_duration};
pub use swap::{
    Chain, MultiChainSwap, MultiChainSwapProps, SwapField, SwapFieldProps, SwapQuoteRow, Token,
    TokenSide,
};
pub use swipeable_list::{
    SwipeAction, SwipeActionEvent, SwipeActionTone, SwipeSide, SwipeableList,
    SwipeableListClassNames, SwipeableListItem, SwipeableListProps, SwipeableListValue,
};
pub use table::{
    InsertPosition, SortDirection, SortState, Table, TableAlign, TableColumn, TableMenuItem,
    TableRow, TableRows, TableSortValue, CHECKBOX_PX, CHECKBOX_WIDTH, COLUMN_ACTIVE_SHADOW,
};
pub use text::TextTag;
pub use text_cascade::{TextCascade, TextCascadeProps};
pub use text_reveal::{SplitMode, TextReveal, TextRevealProps, TEXT_REVEAL_SPRING};
pub use text_scramble::{TextScramble, TextScrambleProps, DEFAULT_GLYPHS};
pub use text_shimmer::{
    text_shimmer_style, TextShimmer, TextShimmerProps, TEXT_SHIMMER_CLASS_NAME,
    TEXT_SHIMMER_KEYFRAMES,
};
pub use theme_toggle::{
    RectStart, ThemeToggle, ThemeToggleHandle, ThemeToggleProps, ThemeVariant, THEME_TOGGLE_VT_CSS,
};
pub use tilt_card::{TiltCard, TiltCardProps};
pub use wallet_card::{
    dice_bear_glass_url, truncate_address, WalletAccount, WalletCard, WalletCardProps,
};

/// The surface components will import.
pub mod prelude {
    pub use crate::adaptive_stepper::{
        AdaptiveStepper, AdaptiveStepperDecrement, AdaptiveStepperIncrement, AdaptiveStepperValue,
    };
    pub use crate::animated_badge::{AnimatedBadge, AnimatedBadgeSize, AnimatedBadgeStatus};
    pub use crate::arc_picker::{ArcPicker, ArcPickerOption, ArcPickerSide};
    pub use crate::availability_scheduler::{
        AvailabilityScheduler, DayAvailability, DayKey, TimeRange, WeekAvailability,
    };
    pub use crate::button::{Button, ButtonLink, ButtonSize, ButtonVariant};
    pub use crate::button_extra::{
        ButtonState, Magnetic, MagneticButton, MetallicButton, StatefulButton,
    };
    pub use crate::card::{
        Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardSize,
        CardTitle, CardTitleAs, CardVariant, CARD_TARGET_CLASS,
    };
    pub use crate::checkbox::Checkbox;
    pub use crate::cn;
    pub use crate::color_picker::{
        format_hex, hsva_to_rgba, parse_hex, rgba_to_hsva, ColorPicker, ColorPickerAlpha,
        ColorPickerArea, ColorPickerChannels, ColorPickerContent, ColorPickerEyeDropper,
        ColorPickerHexInput, ColorPickerHue, ColorPickerPreset, ColorPickerPresets,
        ColorPickerSwatch, ColorPickerTrigger, Hsva, Rgba,
    };
    pub use crate::color_selector::{
        ColorSelector, ColorSelectorItem, ColorSelectorLabel, ColorSelectorList,
    };
    pub use crate::date_range_picker::{
        DateRange, DateRangePicker, DateRangePickerCalendar, DateRangePickerClear,
        DateRangePickerContent, DateRangePickerFooter, DateRangePickerGrid, DateRangePickerHeader,
        DateRangePickerSelection, DateRangePickerTrigger,
    };
    pub use crate::icons::*;
    pub use crate::input::{Input, InputClassNames, InputError};
    pub use crate::loader::{Loader, LoaderVariant};
    pub use crate::morphing_lightbox::{
        use_image_viewer, ImageViewer, ImageViewerApi, ImageViewerCaption, ImageViewerClose,
        ImageViewerContent, ImageViewerCounter, ImageViewerGallery, ImageViewerNext,
        ImageViewerPrevious, ImageViewerThumbnail, LightboxImage, MorphingLightbox,
    };
    pub use crate::motion::{
        animate_number, animate_style, css, scale_style, spring, tween, use_number_motion,
        use_style_motion, Ease, MotionStyle,
    };
    pub use crate::otp_input::{OTPInput, OTPStatus};
    pub use crate::radio::{RadioGroup, RadioGroupItem, RadioOrientation};
    pub use crate::range_slider::RangeSlider;
    pub use crate::range_slider_variants::{
        BubbleSlider, FluidSlider, InlineSlider, RulerSlider, WaveSlider,
    };
    pub use crate::switch::Switch;
    pub use crate::table::{
        InsertPosition, SortDirection, SortState, Table, TableAlign, TableColumn, TableMenuItem,
        TableRow, TableRows, TableSortValue, CHECKBOX_PX, CHECKBOX_WIDTH, COLUMN_ACTIVE_SHADOW,
    };
    pub use crate::wheel_picker::{WheelPicker, WheelPickerOption};
    pub use beui_core::ease::{
        ease_drawer, ease_in_out, ease_out, EASE_DRAWER, EASE_IN_OUT, EASE_OUT, EASE_OUT_CSS,
        SPRING_GLIDE, SPRING_LAYOUT, SPRING_MOUSE, SPRING_PANEL, SPRING_PRESS, SPRING_SWAP,
    };
    pub use beui_core::hooks::{
        use_dismiss, use_hover_capable, use_hover_gesture, use_media_query, use_reduced_motion,
        use_slider, use_tap_gesture, use_touch_capable, DismissBehavior, DismissOptions,
        HoverGesture, Slider, SliderConfig, TapGesture, TapRecord,
    };
    pub use beui_core::platform;
    pub use beui_core::touch::{TOUCH_GESTURE_CLASS, TOUCH_GESTURE_CONTENT_CLASS};
    pub use beui_theme::{ThemeId, BASE_DARK, BASE_LIGHT, THEMES};
    pub use dioxus::prelude::*;
}

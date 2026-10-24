//! beUI chart components (`components/charts/`).
//!
//! SVG/HTML data surfaces with an inspecting cursor. The port starts with
//! `status-bar`, then `donut-chart` and `treemap`. Gallery pages for the chart
//! components are still to be added.

pub mod bump_chart;
pub mod candlestick_chart;
pub mod common;
pub mod composition_chart;
pub mod donut_chart;
pub mod funnel_chart;
pub mod heat_calendar;
pub mod icons;
pub mod liquidity_heatmap;
pub mod order_book;
pub mod price_target_fan;
pub mod returns_calendar;
pub mod status_bar;
pub mod treemap;
pub mod volume_profile;

pub use common::Icon;
pub use icons::IconName;

/// The surface components import.
pub mod prelude {
    pub use crate::common::{use_unique_id, Icon};
    pub use crate::icons::IconName;
    pub use beui::prelude::*;
}

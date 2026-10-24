//! Charts, on screen: the status history bar with its inspecting cursor.

use beui::prelude::*;
use beui_charts::bump_chart::{BumpChart, BumpSeries};
use beui_charts::candlestick_chart::{Candle, CandlestickChart};
use beui_charts::composition_chart::{CompositionChart, CompositionSeries};
use beui_charts::donut_chart::{
    DonutChart, DonutChartCenter, DonutChartLegend, DonutChartPlot, DonutSegment,
};
use beui_charts::funnel_chart::{FunnelChart, FunnelStage};
use beui_charts::heat_calendar::civil_from_days;
use beui_charts::heat_calendar::{days_from_civil, HeatCalendar};
use beui_charts::liquidity_heatmap::{LiquidityHeatmap, LiquidityLevel, LiquiditySnapshot};
use beui_charts::order_book::{
    OrderBook, OrderBookBalance, OrderBookDepthSummary, OrderBookHeader, OrderBookLevel,
    OrderBookSide, OrderBookSpread, Side,
};
use beui_charts::price_target_fan::{FanDates, PriceHistoryPoint, PriceTarget, PriceTargetFan};
use beui_charts::returns_calendar::ReturnsCalendar;
use beui_charts::status_bar::{status_bar_statuses, StatusBar, StatusBarDatum};
use beui_charts::treemap::{Treemap, TreemapGroup, TreemapLeaf, TreemapNode};
use beui_charts::volume_profile::{VolumeProfile, VolumeProfileBin};

#[component]
pub fn Charts() -> Element {
    let data = vec![
        StatusBarDatum::new("d1", "Mar 1", "operational"),
        StatusBarDatum::new("d2", "Mar 2", "operational"),
        StatusBarDatum::new("d3", "Mar 3", "degraded").description("Elevated latency"),
        StatusBarDatum::new("d4", "Mar 4", "operational"),
        StatusBarDatum::new("d5", "Mar 5", "outage").description("API unavailable for 22 minutes"),
        StatusBarDatum::new("d6", "Mar 6", "maintenance").description("Scheduled database work"),
        StatusBarDatum::new("d7", "Mar 7", "operational"),
        StatusBarDatum::new("d8", "Mar 8", "operational"),
        StatusBarDatum::new("d9", "Mar 9", "degraded").description("Slow exports"),
        StatusBarDatum::new("d10", "Mar 10", "operational"),
        StatusBarDatum::new("d11", "Mar 11", "operational"),
        StatusBarDatum::new("d12", "Mar 12", "operational"),
    ];

    let segments = vec![
        DonutSegment::new("direct", "Direct", "#2563eb", 4200.0),
        DonutSegment::new("search", "Search", "#059669", 3100.0),
        DonutSegment::new("social", "Social", "#d97706", 1800.0),
        DonutSegment::new("referral", "Referral", "#e11d48", 900.0),
    ];

    let volume_bins: Vec<VolumeProfileBin> = (0..30)
        .map(|index| {
            let low = 100.0 + index as f64 * 0.5;
            let distance = (index as f64 - 16.0).abs();
            let volume =
                (900.0 - distance * distance * 22.0).max(40.0) + (index as f64 * 7.0) % 60.0;
            VolumeProfileBin::new(format!("b{index}"), low, low + 0.5, volume)
        })
        .collect();

    let return_years: Vec<i32> = (2019..=2025).collect();
    let returns: Vec<Vec<f64>> = (0..7)
        .map(|year| {
            (0..12)
                .map(|month| {
                    let seed = (year * 12 + month) as f64;
                    (((seed * 0.7).sin() + (seed * 0.23).cos()) * 3.2).round() / 1.0
                })
                .collect()
        })
        .collect();

    let history: Vec<PriceHistoryPoint> = {
        let start = beui_charts::heat_calendar::days_from_civil(2025, 1, 6);
        (0..52)
            .map(|index| {
                let (year, month, day) = civil_from_days(start + index * 7);
                let price = 118.0 + (index as f64 * 0.42).sin() * 9.0 + index as f64 * 0.28;
                PriceHistoryPoint::new(format!("{year:04}-{month:02}-{day:02}"), price)
            })
            .collect()
    };
    let fan_targets = vec![
        PriceTarget::new("High", 168.0, 5),
        PriceTarget::new("Mean", 142.5, 12),
        PriceTarget::new("Low", 118.0, 4),
    ];

    let bids: Vec<OrderBookLevel> = (0..9)
        .map(|index| {
            OrderBookLevel::new(
                104.98 - index as f64 * 0.05,
                2.0 + (index as f64 * 1.7) % 11.0,
            )
        })
        .collect();
    let asks: Vec<OrderBookLevel> = (0..9)
        .map(|index| {
            OrderBookLevel::new(
                105.02 + index as f64 * 0.05,
                1.5 + (index as f64 * 2.3) % 12.0,
            )
        })
        .collect();

    let prices = [100.0, 102.0, 104.0, 106.0, 108.0, 110.0];
    let snapshots: Vec<LiquiditySnapshot> = (0..20)
        .map(|step| {
            let levels: Vec<LiquidityLevel> = prices
                .iter()
                .enumerate()
                .map(|(index, price)| {
                    LiquidityLevel::new(*price, ((step * 7 + index * 3) % 11) as f64 * 22.0)
                })
                .collect();
            LiquiditySnapshot::new(format!("t{step}"), format!("T{step}"), levels)
                .price(105.0 + ((step % 5) as f64 - 2.0) * 0.5)
        })
        .collect();

    let heat_values: Vec<Vec<f64>> = (0..16)
        .map(|week| {
            (0..7)
                .map(|day| {
                    let seed = (week * 7 + day) % 13;
                    (seed as f64 / 12.0).powf(1.4)
                })
                .collect()
        })
        .collect();
    let heat_end = days_from_civil(2026, 3, 20);

    let funnel = vec![
        FunnelStage::new("visit", "Visited", 10000.0),
        FunnelStage::new("signup", "Signed up", 4200.0),
        FunnelStage::new("activate", "Activated", 2600.0),
        FunnelStage::new("subscribe", "Subscribed", 1100.0),
        FunnelStage::new("retain", "Retained", 640.0),
    ];

    let candles = vec![
        Candle::new("t1", "09:30", 100.0, 103.2, 99.1, 102.4, 3200.0),
        Candle::new("t2", "09:45", 102.4, 104.1, 101.0, 101.6, 2100.0),
        Candle::new("t3", "10:00", 101.6, 105.8, 101.2, 105.1, 4100.0),
        Candle::new("t4", "10:15", 105.1, 106.4, 103.9, 104.2, 2600.0),
        Candle::new("t5", "10:30", 104.2, 104.9, 101.8, 102.1, 3000.0),
        Candle::new("t6", "10:45", 102.1, 103.6, 100.4, 100.9, 3500.0),
        Candle::new("t7", "11:00", 100.9, 102.2, 99.6, 101.9, 2800.0),
        Candle::new("t8", "11:15", 101.9, 104.7, 101.5, 104.3, 3900.0),
        Candle::new("t9", "11:30", 104.3, 107.2, 104.0, 106.8, 4600.0),
        Candle::new("t10", "11:45", 106.8, 108.9, 106.1, 108.4, 5100.0),
        Candle::new("t11", "12:00", 108.4, 109.2, 105.7, 106.0, 3700.0),
        Candle::new("t12", "12:15", 106.0, 107.4, 104.8, 107.1, 2900.0),
    ];
    let last_price = 107.1;

    let quarters: Vec<String> = (1..=6).map(|quarter| format!("Q{quarter}")).collect();
    let rankings = vec![
        BumpSeries::new(
            "alpha",
            "Alpha",
            vec![Some(3), Some(2), Some(3), Some(1), Some(2), Some(1)],
        ),
        BumpSeries::new(
            "bravo",
            "Bravo",
            vec![Some(1), Some(1), Some(2), Some(3), Some(1), Some(2)],
        ),
        BumpSeries::new(
            "charlie",
            "Charlie",
            vec![Some(2), Some(3), Some(1), Some(2), Some(3), Some(3)],
        ),
        BumpSeries::new(
            "delta",
            "Delta",
            vec![Some(4), Some(4), None, Some(4), Some(4), Some(4)],
        ),
    ];

    let weeks: Vec<String> = (1..=6).map(|week| format!("W{week}")).collect();
    let composition = vec![
        CompositionSeries::new(
            "organic",
            "Organic",
            "#2563eb",
            vec![
                Some(120.0),
                Some(140.0),
                Some(160.0),
                Some(180.0),
                Some(200.0),
                Some(230.0),
            ],
        ),
        CompositionSeries::new(
            "paid",
            "Paid",
            "#7c3aed",
            vec![
                Some(90.0),
                Some(96.0),
                Some(110.0),
                Some(105.0),
                Some(130.0),
                Some(150.0),
            ],
        ),
        CompositionSeries::new(
            "referral",
            "Referral",
            "#0f766e",
            vec![
                Some(60.0),
                Some(64.0),
                Some(58.0),
                None,
                Some(72.0),
                Some(80.0),
            ],
        ),
        CompositionSeries::new(
            "social",
            "Social",
            "#b45309",
            vec![
                Some(40.0),
                Some(45.0),
                Some(50.0),
                Some(56.0),
                Some(60.0),
                Some(70.0),
            ],
        ),
    ];

    let tree = vec![
        TreemapNode::Group(
            TreemapGroup::new(
                "app",
                "Application",
                vec![
                    TreemapNode::Leaf(TreemapLeaf::new("render", "Rendering", 3200.0)),
                    TreemapNode::Leaf(TreemapLeaf::new("state", "State", 2100.0)),
                    TreemapNode::Leaf(TreemapLeaf::new("route", "Routing", 900.0)),
                ],
            )
            .color("#1d4ed8"),
        ),
        TreemapNode::Group(
            TreemapGroup::new(
                "vendor",
                "Vendor",
                vec![
                    TreemapNode::Leaf(TreemapLeaf::new("dioxus", "Dioxus", 2600.0)),
                    TreemapNode::Leaf(TreemapLeaf::new("motion", "Motion", 1200.0)),
                    TreemapNode::Leaf(TreemapLeaf::new("misc", "Misc", 400.0)),
                ],
            )
            .color("#7c3aed"),
        ),
        TreemapNode::Leaf(
            TreemapLeaf::new("assets", "Assets", 700.0).description("Images and fonts"),
        ),
    ];

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Charts"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "Charts" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "SVG/HTML data surfaces with an inspecting cursor. The status bar is the first chart ported."
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "DonutChart" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Hover a segment or a legend row to pop it out and read it in the centre."
                }
                div { class: "max-w-2xl",
                    DonutChart { segments, label: "Traffic by channel",
                        DonutChartPlot { DonutChartCenter { total_label: "Sessions" } }
                        DonutChartLegend {}
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "VolumeProfile" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Executed volume by price; the POC and the value area are marked, arrow keys inspect a level."
                }
                div { class: "max-w-2xl",
                    VolumeProfile {
                        data: volume_bins,
                        label: "Session volume profile".to_string(),
                        unit: "contracts".to_string(),
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "ReturnsCalendar" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Monthly returns with a compounded Year column; click a month to anchor a span, the Year column spans whole years."
                }
                div { class: "max-w-2xl",
                    ReturnsCalendar { years: return_years, returns }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "PriceTargetFan" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "A year of history then three analyst targets; hover the fan or scrub the history."
                }
                div { class: "max-w-2xl",
                    PriceTargetFan {
                        current: 131.2,
                        targets: fan_targets,
                        history,
                        label: "Price target · 12 months".to_string(),
                        dates: FanDates {
                            start: Some("Jan 2025".to_string()),
                            mid: Some("Jul 2025".to_string()),
                            horizon: Some("Jan 2026".to_string()),
                        },
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "OrderBook" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Snapshot depth ladder; hover or focus a level to total the cost of filling up to it."
                }
                div { class: "max-w-xl",
                    OrderBook {
                        bids,
                        asks,
                        last_price: Some(105.0),
                        label: "ETH/USD".to_string(),
                        base_symbol: "ETH".to_string(),
                        quote_symbol: "USD".to_string(),
                        on_level_select: move |_| {},
                        OrderBookHeader {}
                        OrderBookSide { side: Side::Ask }
                        OrderBookSpread {}
                        OrderBookSide { side: Side::Bid }
                        OrderBookBalance {}
                        OrderBookDepthSummary {}
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "LiquidityHeatmap" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Resting liquidity by price and time; hover or arrow-key across cells, the line traces the market price."
                }
                div { class: "max-w-3xl",
                    LiquidityHeatmap {
                        snapshots,
                        label: "Order book depth".to_string(),
                        unit: "units".to_string(),
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "HeatCalendar" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Weeks of activity; hover a cell for its day and count, click to pin it, hover the legend to filter a level."
                }
                div { class: "max-w-3xl",
                    HeatCalendar {
                        values: heat_values,
                        end_date: Some(heat_end),
                        weeks: 16,
                        unit: "commits".to_string(),
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "FunnelChart" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Ordered conversion stages; hover a stage for its step and total conversion."
                }
                div { class: "max-w-3xl",
                    FunnelChart { stages: funnel, label: "Signup funnel" }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "CandlestickChart" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Move the pointer or press arrow keys to read a candle on the crosshair."
                }
                div { class: "max-w-3xl",
                    CandlestickChart { candles, last_price: Some(last_price), label: "Price history" }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "BumpChart" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Hover a point for its rank change; the legend highlights or pins a series."
                }
                div { class: "max-w-3xl",
                    BumpChart { series: rankings, periods: quarters, label: "League standings" }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "CompositionChart" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Hover or click a legend item to highlight a series; the range inspects a period."
                }
                div { class: "max-w-3xl",
                    CompositionChart { series: composition, periods: weeks, label: "Traffic composition" }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "Treemap" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Hover a tile to inspect it; arrow keys move between tiles."
                }
                div { class: "max-w-3xl",
                    Treemap { data: tree, label: "Bundle size" }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-lg font-semibold tracking-tight", "StatusBar" }
                p { class: "max-w-3xl text-sm text-muted-foreground",
                    "Arrow keys or a click inspect a period; the legend and the screen-reader table carry the data non-visually."
                }
                div { class: "max-w-2xl",
                    StatusBar {
                        data,
                        label: "Service status",
                        statuses: status_bar_statuses(),
                    }
                }
            }
        }
    }
}

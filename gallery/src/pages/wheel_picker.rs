//! WheelPicker, on screen — `wheel-picker.preview.tsx`.
//!
//! Month / day / year on three drums. Picking a short month or a non-leap
//! February can strand the day past the end, so the day is pulled back to the
//! last valid one.

use crate::pages::common::*;
use beui::prelude::*;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn days_in(month: usize, year: i32) -> usize {
    match month {
        0 | 2 | 4 | 6 | 7 | 9 | 11 => 31,
        3 | 5 | 8 | 10 => 30,
        _ => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

#[component]
pub fn WheelPickerPage() -> Element {
    let mut month = use_signal(|| "June".to_string());
    let mut day = use_signal(|| "9".to_string());
    let mut year = use_signal(|| "2004".to_string());
    let mut sound = use_signal(|| false);

    let month_index = MONTHS.iter().position(|m| *m == month()).unwrap_or(0);
    let day_count = days_in(month_index, year().parse::<i32>().unwrap_or(2004));
    let days: Vec<WheelPickerOption> = (1..=day_count)
        .map(|d| WheelPickerOption::plain(d.to_string()))
        .collect();

    use_effect(move || {
        if day().parse::<usize>().unwrap_or(1) > day_count {
            day.set(day_count.to_string());
        }
    });

    let months: Vec<WheelPickerOption> = MONTHS
        .iter()
        .map(|m| WheelPickerOption::plain(*m))
        .collect();
    let years: Vec<WheelPickerOption> = (1980..2040)
        .map(|y| WheelPickerOption::plain(y.to_string()))
        .collect();

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "WheelPicker",
                lede: "Rows on a 3D drum. A drag or a flick coasts with the source's deceleration and snaps to a row with a small spring bounce."
            }

            Section {
                title: "Date of birth",
                note: "Three drums on a shared card; the last valid day is enforced as the month and year change.",
                Stage { class: "flex-col",
                    span { class: "text-sm text-muted-foreground",
                        "Born "
                        span { class: "font-medium text-foreground tabular-nums", "{month()} {day()}, {year()}" }
                    }
                    div { class: "flex items-stretch gap-1 rounded-3xl border border-border bg-background p-2",
                        WheelPicker {
                            options: months,
                            value: month(),
                            on_value_change: move |next| month.set(next),
                            class: "w-32 border-0 bg-transparent",
                            visible_count: 7,
                            item_height: 42.0,
                            sound: sound(),
                            aria_label: "Month",
                        }
                        WheelPicker {
                            options: days,
                            value: day(),
                            on_value_change: move |next| day.set(next),
                            class: "w-14 border-0 bg-transparent",
                            visible_count: 7,
                            item_height: 42.0,
                            sound: sound(),
                            aria_label: "Day",
                        }
                        WheelPicker {
                            options: years,
                            value: year(),
                            on_value_change: move |next| year.set(next),
                            class: "w-20 border-0 bg-transparent",
                            visible_count: 7,
                            item_height: 42.0,
                            sound: sound(),
                            aria_label: "Year",
                        }
                    }
                    Switch {
                        checked: sound(),
                        on_checked_change: move |next| sound.set(next),
                        label: "Tick sound",
                        class: "origin-left scale-[0.85] [&_label]:text-sm",
                    }
                }
            }
        }
    }
}

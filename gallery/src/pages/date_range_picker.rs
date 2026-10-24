//! DateRangePicker, on screen — `date-range-picker.preview.tsx`.
//!
//! The last seven days ending "today" (`max`), which is also the reference date
//! the today ring uses.

use crate::pages::common::*;
use beui::prelude::*;

const TODAY: &str = "2026-01-15";
const WEEK_START: &str = "2026-01-09";

#[component]
pub fn DateRangePickerPage() -> Element {
    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "DateRangePicker",
                lede: "A range calendar over plain YYYY-MM-DD dates. Click a start, preview the range as you hover, then click an end; the range is always ordered."
            }

            Section {
                title: "Reporting period",
                note: "Defaults to the last seven days; future dates are disabled by max.",
                Stage { class: "items-start",
                    DateRangePicker {
                        label: "Reporting period",
                        default_month: TODAY.to_string(),
                        default_value: DateRange { from: WEEK_START.to_string(), to: Some(TODAY.to_string()) },
                        max: TODAY.to_string(),
                        DateRangePickerTrigger {}
                        DateRangePickerContent {
                            DateRangePickerCalendar { class: "w-full rounded-none border-0",
                                DateRangePickerHeader {}
                                DateRangePickerGrid {}
                                DateRangePickerFooter {
                                    div { class: "flex items-center justify-between gap-2",
                                        DateRangePickerSelection {}
                                        DateRangePickerClear {}
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

//! OTPInput, on screen — `blocks/otp-input.preview.tsx`.
//!
//! Enter the code `123456`: the field reports completion and flips to success,
//! any other full code shakes and turns red.

use crate::pages::common::*;
use beui::prelude::*;

const CODE: &str = "123456";

#[component]
pub fn OTPInputPage() -> Element {
    let mut value = use_signal(String::new);
    let mut status = use_signal(|| OTPStatus::Idle);

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "OTPInput",
                lede: "Each digit rolls into its slot, the active slot blinks a caret, and the whole row shakes on an error while a check draws itself on success."
            }

            Section {
                title: "Verification",
                note: "Enter 123456 to verify.",
                Stage {
                    div { class: "flex flex-col items-center gap-4",
                        OTPInput {
                            label: "Verification code",
                            hint: "Enter {CODE} to verify.",
                            success_message: "Verified.",
                            error_message: "Wrong code, try again.",
                            value: value(),
                            status: status(),
                            on_change: move |next| {
                                value.set(next);
                                if status() != OTPStatus::Idle {
                                    status.set(OTPStatus::Idle);
                                }
                            },
                            on_complete: move |complete| {
                                status.set(if complete == CODE { OTPStatus::Success } else { OTPStatus::Error });
                            },
                        }
                    }
                }
            }
        }
    }
}

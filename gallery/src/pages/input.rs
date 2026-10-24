//! Input, on screen — `components/previews/motion/input.preview.tsx`.
//!
//! Three fields: an email that errors as you type (with `reserveErrorLine` so
//! the message does not shift the layout), a password with a show/hide button,
//! and a search field whose check draws itself once there is a query.

use crate::pages::common::*;
use beui::prelude::*;

#[component]
pub fn InputPage() -> Element {
    let mut email = use_signal(String::new);
    let mut pass = use_signal(|| "hunter2".to_string());
    let mut query = use_signal(|| "Ada".to_string());
    let mut show = use_signal(|| false);

    let email_value = email();
    let email_error = if !email_value.is_empty() && !email_value.contains('@') {
        InputError::Message("Enter a valid email address.".to_string())
    } else {
        InputError::None
    };

    rsx! {
        PageBody {
            PageHeader {
                category: "Motion",
                title: "Input",
                lede: "A field that shakes on error, draws a success check, and keeps its message line reserved so validation does not move the page."
            }

            Section {
                title: "Fields",
                note: "label, left and right icons, error and success, all on the same rounded shell.",
                div { class: "flex w-full max-w-xs flex-col gap-1",
                    Input {
                        label: "Email",
                        r#type: "email",
                        placeholder: "you@example.com",
                        left_icon: rsx! { IconMail {} },
                        value: email_value.clone(),
                        on_change: move |next| email.set(next),
                        error: email_error,
                        reserve_error_line: true,
                    }
                    Input {
                        label: "Password",
                        r#type: if show() { "text" } else { "password" },
                        value: pass(),
                        on_change: move |next| pass.set(next),
                        reserve_error_line: true,
                        right_icon: rsx! {
                            button {
                                r#type: "button",
                                "aria-label": if show() { "Hide password" } else { "Show password" },
                                class: "pointer-events-auto",
                                onclick: move |_| show.toggle(),
                                if show() { IconEyeOff {} } else { IconEye {} }
                            }
                        },
                    }
                    Input {
                        label: "Search",
                        left_icon: rsx! { IconSearch {} },
                        value: query(),
                        on_change: move |next| query.set(next),
                        success: query().len() > 1,
                        reserve_error_line: true,
                    }
                }
            }
        }
    }
}

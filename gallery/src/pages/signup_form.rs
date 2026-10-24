//! `SignUpForm` — the beUI preview: any email succeeds except one that starts
//! with `taken@`, which reports a form-level error.

use beui::prelude::*;
use beui::{SignUpForm, SignUpStatus, SignUpValues};
use beui_core::platform;

#[component]
pub fn SignUpFormPage() -> Element {
    let mut status = use_signal(SignUpStatus::default);
    let mut form_error = use_signal(|| None::<String>);

    rsx! {
        div { class: "mx-auto flex max-w-5xl flex-col gap-12 px-6 py-16",
            header { class: "flex flex-col gap-3",
                p { class: "text-xs font-medium uppercase tracking-widest text-muted-foreground",
                    "Blocks"
                }
                h1 { class: "text-balance text-3xl font-semibold tracking-tight", "SignUpForm" }
                p { class: "max-w-2xl text-pretty text-sm leading-6 text-muted-foreground",
                    "Validation rewards early and punishes late: errors are computed on every change, but a field only shows one once it has been blurred, so a first entry is never flagged mid-typing while a field already in error clears the moment it becomes valid."
                }
            }
            div { class: "flex w-full justify-center py-4",
                SignUpForm {
                    status: Some(status()),
                    error_message: form_error(),
                    description: Some("Sign up with taken@example.com to see the failure state.".to_string()),
                    on_submit: Some(EventHandler::new(move |values: SignUpValues| {
                        status.set(SignUpStatus::Loading);
                        form_error.set(None);
                        spawn(async move {
                            let _ = platform::json::<bool>(
                                "await new Promise((resolve) => setTimeout(resolve, 1200));\nreturn true;",
                            )
                            .await;
                            if values.email.to_lowercase().starts_with("taken@") {
                                form_error.set(Some("That email is already registered.".to_string()));
                                status.set(SignUpStatus::Error);
                            } else {
                                status.set(SignUpStatus::Success);
                            }
                        });
                    })),
                }
            }
        }
    }
}

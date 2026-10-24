//! Class-name concatenation — the `cn()` replacement from `lib/utils.ts`.
//!
//! The source uses `twMerge(clsx(...))`. That is deliberately not mirrored here:
//! beUI never builds a class string at runtime (variants are static literals in
//! `Record<Variant, string>` maps), so the only job left is joining with single
//! spaces. Conflicts are resolved by CSS cascade instead — variant classes stay
//! mutually exclusive inside a component and a consumer's `class` is appended
//! last, which is what Tailwind's cascade needs anyway.

/// What [`cn!`] accepts per argument.
///
/// Implemented for `str`/`String` (and references to them, through the blanket
/// impl) plus `Option<&str>` / `Option<String>`, so a component can pass its
/// optional `class` prop straight through without unwrapping. `None` and `""`
/// contribute nothing and never leave a stray space behind.
pub trait Class {
    fn class(&self) -> &str;
}

impl Class for str {
    fn class(&self) -> &str {
        self
    }
}

impl Class for String {
    fn class(&self) -> &str {
        self
    }
}

impl<T: Class + ?Sized> Class for &T {
    fn class(&self) -> &str {
        (**self).class()
    }
}

impl Class for Option<String> {
    fn class(&self) -> &str {
        self.as_deref().unwrap_or_default()
    }
}

impl Class for Option<&str> {
    fn class(&self) -> &str {
        self.unwrap_or_default()
    }
}

/// Join class strings with single spaces, skipping empty ones.
///
/// ```ignore
/// let class = cn!("inline-flex h-9", SIZE_CLASS[size], VARIANT_CLASS[variant], class);
/// ```
#[macro_export]
macro_rules! cn {
    () => {
        ::std::string::String::new()
    };
    ($($part:expr),+ $(,)?) => {{
        let mut joined = ::std::string::String::new();
        $(
            let part_value = $part;
            let part: &str = $crate::class::Class::class(&part_value);
            if !part.is_empty() {
                if !joined.is_empty() {
                    joined.push(' ');
                }
                joined.push_str(part);
            }
        )+
        joined
    }};
}

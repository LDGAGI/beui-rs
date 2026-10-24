//! Media queries and other JS boolean expressions, kept in sync.
//!
//! beUI's capability hooks are all the same shape: read a `matchMedia` query
//! (sometimes two), and update when it changes. On the web renderer the first
//! read is synchronous — `web-sys` is right there — which is what keeps the
//! first frame correct; on desktop the read has to go through `document::eval`,
//! so the first frame gets the default and the answer lands a beat later, the
//! same way a React effect lands after the first paint.
//!
//! Change notifications come from one portable script in both cases: the
//! evaluator pushes each new value over the `Eval` channel.

use dioxus::prelude::*;

use crate::platform;

/// The three queries a capability answer can depend on. Re-evaluating on any of
/// them is cheap and avoids guessing which one a given expression reads.
const WATCHED_QUERIES: [&str; 3] = [
    "(hover: hover) and (pointer: fine)",
    "(any-pointer: coarse)",
    "(prefers-reduced-motion: reduce)",
];

#[derive(Clone, Copy)]
enum BooleanSource {
    /// A JS expression that evaluates to a boolean.
    Expr(&'static str),
    /// A media query, evaluated as `matchMedia(query).matches`.
    Query(&'static str),
}

impl BooleanSource {
    fn js(self) -> String {
        match self {
            BooleanSource::Expr(expr) => format!("Boolean({expr})"),
            BooleanSource::Query(query) => {
                format!(
                    "Boolean(matchMedia({}).matches)",
                    platform::js_string(query)
                )
            }
        }
    }
}

/// Track a JS boolean expression, re-evaluated whenever a relevant media query
/// changes or the window resizes.
///
/// `sync_query` is the media query that backs `expr` verbatim; it is what makes
/// the first render correct on web. Pass `None` when `expr` is not a single
/// query (e.g. the `maxTouchPoints` fallback), and the value stays `false` until
/// the evaluator answers — exactly like the source's `useState(false)` plus
/// `useEffect`.
pub fn use_js_boolean(sync_query: Option<&'static str>, expr: &'static str) -> Signal<bool> {
    use_boolean_source(sync_query, BooleanSource::Expr(expr))
}

/// Track a plain media query, e.g. `use_media_query("(prefers-reduced-motion: reduce)")`.
pub fn use_media_query(query: &'static str) -> Signal<bool> {
    use_boolean_source(Some(query), BooleanSource::Query(query))
}

fn use_boolean_source(sync_query: Option<&'static str>, source: BooleanSource) -> Signal<bool> {
    let initial = sync_query
        .and_then(platform::media_matches)
        .unwrap_or(false);
    let mut state = use_signal(|| initial);

    // Built once, when the future first runs.
    use_future(move || async move {
        let mut queries: Vec<&str> = WATCHED_QUERIES.to_vec();
        if let Some(query) = sync_query {
            if !queries.contains(&query) {
                queries.push(query);
            }
        }
        let queries = queries
            .into_iter()
            .map(platform::js_string)
            .collect::<Vec<_>>()
            .join(", ");
        let script = format!(
            "const read = () => {};\n\
             dioxus.send(read());\n\
             const re = () => dioxus.send(read());\n\
             for (const q of [{queries}]) {{\n\
               try {{ matchMedia(q).addEventListener('change', re); }} catch (e) {{}}\n\
             }}\n\
             addEventListener('resize', re);",
            source.js()
        );
        let Some(mut eval) = platform::eval(&script) else {
            return;
        };
        while let Ok(value) = eval.recv::<bool>().await {
            state.set(value);
        }
    });

    state
}

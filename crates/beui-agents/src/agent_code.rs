//! `agents/agent-code.tsx` — syntax-highlighted code for the agent surfaces.
//!
//! The source tokenizes with **shiki** (`codeToTokensWithThemes` over the
//! `github-light-high-contrast` / `github-dark-high-contrast` pair) and drops
//! each token's two colours into `--agent-code-light` / `--agent-code-dark`,
//! which `dark:` then picks between. shiki is a TextMate engine and a large
//! bundle, so there are three options: interop, syntect, or an approximate
//! tokenizer. The port takes the third for now — a hand-written
//! scanner for the six languages the agents components use, emitting the same
//! two-per-token CSS variables so the DOM and the styling hook are identical.
//! Token boundaries therefore differ from shiki, but the colour roles map onto
//! shiki's high-contrast palettes.

use beui_core::cn;
use dioxus::prelude::*;

/// The languages the agents components highlight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AgentCodeLanguage {
    Bash,
    Diff,
    Json,
    Text,
    Tsx,
    #[default]
    TypeScript,
}

impl AgentCodeLanguage {
    /// The lowercase label the code header shows, e.g. `typescript`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Diff => "diff",
            Self::Json => "json",
            Self::Text => "text",
            Self::Tsx => "tsx",
            Self::TypeScript => "typescript",
        }
    }
}

/// One highlighted run of code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentCodeToken {
    pub content: String,
    pub offset: usize,
    pub light: String,
    pub dark: String,
}

/// Tokenized code, one inner vector per line.
pub type AgentCodeTokenLines = Vec<Vec<AgentCodeToken>>;

/// What a token means, which picks its colour pair.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Plain,
    Keyword,
    Constant,
    String,
    Comment,
    Number,
    Function,
    Type,
    Property,
    Operator,
    Variable,
    Added,
    Removed,
    Meta,
}

/// shiki's `github-light-high-contrast` / `github-dark-high-contrast` colours,
/// grouped by role. `currentColor` keeps unclassified runs inheriting the
/// block's own `text-foreground/85`.
fn colors(kind: TokenKind) -> (&'static str, &'static str) {
    match kind {
        TokenKind::Keyword | TokenKind::Removed => ("#cf222e", "#ff7b72"),
        TokenKind::String | TokenKind::Added => ("#0a3069", "#a5d6ff"),
        TokenKind::Comment => ("#6e7781", "#8b949e"),
        TokenKind::Constant | TokenKind::Number => ("#0550ae", "#79c0ff"),
        TokenKind::Function | TokenKind::Property => ("#8250df", "#d2a8ff"),
        TokenKind::Type | TokenKind::Variable => ("#953800", "#ffa657"),
        TokenKind::Meta => ("#0550ae", "#79c0ff"),
        TokenKind::Operator | TokenKind::Plain => ("currentColor", "currentColor"),
    }
}

fn is_ident_start(c: u8) -> bool {
    c == b'_' || c == b'$' || c.is_ascii_alphabetic()
}

fn is_ident(c: u8) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

const TS_KEYWORDS: &[&str] = &[
    "abstract",
    "as",
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "get",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "keyof",
    "let",
    "new",
    "null",
    "of",
    "package",
    "private",
    "protected",
    "public",
    "readonly",
    "return",
    "satisfies",
    "set",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

const BASH_KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
    "function", "in", "return", "export", "local", "readonly", "declare", "echo", "cd", "set",
    "source", "alias", "unset", "exit", "trap", "eval", "sudo", "git", "bun", "npm", "npx",
    "cargo", "docker", "kubectl", "make", "curl", "grep", "sed", "awk", "cat", "ls", "rm", "mkdir",
    "chmod", "find", "which", "sleep", "kill", "pkill",
];

/// Tokenize `code` line by line: the port's stand-in for
/// `useAgentCodeTokens(code, language)`.
pub fn tokenize(code: &str, language: AgentCodeLanguage) -> AgentCodeTokenLines {
    let mut lines = Vec::new();
    let mut block_comment = false;
    let mut offset = 0usize;

    for line in code.split('\n') {
        let mut line_tokens: Vec<AgentCodeToken> = Vec::new();
        if language == AgentCodeLanguage::Text {
            if !line.is_empty() {
                line_tokens.push(token(TokenKind::Plain, line, offset));
            }
        } else if language == AgentCodeLanguage::Diff {
            tokenize_diff_line(line, offset, &mut line_tokens);
        } else {
            tokenize_code_line(line, language, &mut block_comment, offset, &mut line_tokens);
        }
        lines.push(line_tokens);
        offset += line.len() + 1;
    }
    lines
}

fn token(kind: TokenKind, content: &str, offset: usize) -> AgentCodeToken {
    let (light, dark) = colors(kind);
    AgentCodeToken {
        content: content.to_string(),
        offset,
        light: light.to_string(),
        dark: dark.to_string(),
    }
}

fn tokenize_diff_line(line: &str, offset: usize, out: &mut Vec<AgentCodeToken>) {
    if line.is_empty() {
        return;
    }
    let (kind, rest) = match line.as_bytes()[0] {
        b'+' if !line.starts_with("+++") => (TokenKind::Added, line),
        b'-' if !line.starts_with("---") => (TokenKind::Removed, line),
        b'@' => (TokenKind::Meta, line),
        b'd' if line.starts_with("diff ") => (TokenKind::Keyword, line),
        b'i' if line.starts_with("index ") => (TokenKind::Comment, line),
        _ => (TokenKind::Plain, line),
    };
    out.push(token(kind, rest, offset));
}

fn tokenize_code_line(
    line: &str,
    language: AgentCodeLanguage,
    block_comment: &mut bool,
    line_offset: usize,
    out: &mut Vec<AgentCodeToken>,
) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    // JSON has no comments; bash uses `#`.
    let hash_comment = language == AgentCodeLanguage::Bash;
    let slash_comment = language != AgentCodeLanguage::Json;

    while i < len {
        if *block_comment {
            match line[i..].find("*/") {
                Some(end) => {
                    let stop = i + end + 2;
                    out.push(token(TokenKind::Comment, &line[i..stop], line_offset + i));
                    i = stop;
                    *block_comment = false;
                }
                None => {
                    out.push(token(TokenKind::Comment, &line[i..], line_offset + i));
                    return;
                }
            }
            continue;
        }

        let c = bytes[i];

        if c == b' ' || c == b'\t' {
            let start = i;
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            out.push(token(
                TokenKind::Plain,
                &line[start..i],
                line_offset + start,
            ));
            continue;
        }

        if hash_comment && c == b'#' {
            out.push(token(TokenKind::Comment, &line[i..], line_offset + i));
            return;
        }

        if slash_comment && c == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            out.push(token(TokenKind::Comment, &line[i..], line_offset + i));
            return;
        }

        if slash_comment && c == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            *block_comment = true;
            continue;
        }

        // Strings and template literals (`json`/ts use double, bash single).
        if c == b'"' || c == b'\'' || (c == b'`' && language != AgentCodeLanguage::Bash) {
            let quote = c;
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            i = i.min(len);
            out.push(token(
                TokenKind::String,
                &line[start..i],
                line_offset + start,
            ));
            continue;
        }

        if language == AgentCodeLanguage::Bash && c == b'$' {
            let start = i;
            i += 1;
            if i < len && bytes[i] == b'{' {
                while i < len && bytes[i] != b'}' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
            } else if i < len && bytes[i] == b'(' {
                while i < len && bytes[i] != b')' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
            } else {
                while i < len && (is_ident(bytes[i]) || bytes[i] == b'?') {
                    i += 1;
                }
            }
            out.push(token(
                TokenKind::Variable,
                &line[start..i],
                line_offset + start,
            ));
            continue;
        }

        if c.is_ascii_digit() {
            let start = i;
            while i < len
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'.'
                    || bytes[i] == b'_'
                    || bytes[i] == b'x'
                    || bytes[i] == b'X')
            {
                i += 1;
            }
            out.push(token(
                TokenKind::Number,
                &line[start..i],
                line_offset + start,
            ));
            continue;
        }

        if is_ident_start(c) {
            let start = i;
            while i < len && is_ident(bytes[i]) {
                i += 1;
            }
            let word = &line[start..i];
            let mut j = i;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            let followed_by_open = j < len && bytes[j] == b'(';
            let followed_by_colon = j < len && bytes[j] == b':';
            let kind = if language == AgentCodeLanguage::Json {
                if word == "true" || word == "false" || word == "null" {
                    TokenKind::Constant
                } else {
                    TokenKind::Plain
                }
            } else if language == AgentCodeLanguage::Bash {
                if BASH_KEYWORDS.contains(&word) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Plain
                }
            } else if TS_KEYWORDS.contains(&word) {
                if word == "true" || word == "false" || word == "null" || word == "undefined" {
                    TokenKind::Constant
                } else {
                    TokenKind::Keyword
                }
            } else if followed_by_open {
                TokenKind::Function
            } else if followed_by_colon {
                TokenKind::Property
            } else if word
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            {
                TokenKind::Type
            } else {
                TokenKind::Plain
            };
            out.push(token(kind, word, line_offset + start));
            continue;
        }

        // Operators and punctuation: emit one run per class so colours stay put.
        let start = i;
        if b"+-*/%=<>!&|^~?".contains(&c) {
            while i < len && b"+-*/%=<>!&|^~?".contains(&bytes[i]) {
                i += 1;
            }
            out.push(token(
                TokenKind::Operator,
                &line[start..i],
                line_offset + start,
            ));
        } else {
            i += 1;
            out.push(token(
                TokenKind::Plain,
                &line[start..i],
                line_offset + start,
            ));
        }
    }
}

/// `AgentCodeLine`.
#[component]
pub fn AgentCodeLine(
    code: String,
    #[props(default)] tokens: Option<Vec<AgentCodeToken>>,
    #[props(default)] class: String,
) -> Element {
    rsx! {
        span { class: "{class}",
            match tokens {
                Some(tokens) => rsx! {
                    for (index, token) in tokens.iter().enumerate() {
                        span {
                            key: "{index}",
                            class: "text-[var(--agent-code-light)] dark:text-[var(--agent-code-dark)]",
                            style: "--agent-code-light:{token.light}; --agent-code-dark:{token.dark};",
                            "{token.content}"
                        }
                    }
                },
                None => rsx! { "{code}" },
            }
        }
    }
}

/// The line split shared by `AgentCode` and `CodeBlock`.
pub fn code_lines(code: &str) -> Vec<(String, usize)> {
    let mut offset = 0usize;
    code.split('\n')
        .map(|content| {
            let line = (content.to_string(), offset);
            offset += content.len() + 1;
            line
        })
        .collect()
}

/// `AgentCode`.
#[component]
pub fn AgentCode(
    code: String,
    #[props(default)] language: AgentCodeLanguage,
    #[props(default)] class: String,
) -> Element {
    let tokens = tokenize(&code, language);
    let lines = code_lines(&code);
    let count = lines.len();

    rsx! {
        pre {
            class: cn!(
                "m-0 overflow-x-auto whitespace-pre font-mono text-xs leading-5 text-foreground/85",
                class,
            ),
            code {
                for (index, (content, _offset)) in lines.iter().enumerate() {
                    AgentCodeLine {
                        key: "{index}",
                        code: content.clone(),
                        tokens: tokens.get(index).cloned(),
                    }
                    if index + 1 < count {
                        "\n"
                    }
                }
            }
        }
    }
}

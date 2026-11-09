//! Card text as tokens, so it can move between dialects.
//!
//! Lorealm text is Weaver. Other formats (SillyTavern cards) use their own
//! macro syntax. Each dialect turns its text into [`Token`]s and back; a
//! conversion is "read with one dialect, write with another".
//!
//! Weaver text is read through `weaver_lang`'s own parser, one token per
//! top-level node, so this never drifts from the language. Every token keeps
//! the exact source it came from, so a dialect that can't express a token
//! writes [`Token::source`] back unchanged and nothing is lost.

use weaver_lang::ast::{CommandCall, Expr, ExprKind, ForEachBlock, IfBlock, NodeKind};

/// A macro both dialects can spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroRef {
    /// The character's name: `{{char:name}}` / `{{char}}`.
    CharName,
    /// The user's (persona's) name: `{{user:name}}` / `{{user}}`.
    UserName,
}

impl MacroRef {
    fn weaver(self) -> &'static str {
        match self {
            Self::CharName => "{{char:name}}",
            Self::UserName => "{{user:name}}",
        }
    }

    fn sillytavern(self) -> &'static str {
        match self {
            Self::CharName => "{{char}}",
            Self::UserName => "{{user}}",
        }
    }

    fn from_weaver(scope: &str, name: &str) -> Option<Self> {
        match (scope, name) {
            ("char", "name") => Some(Self::CharName),
            ("user", "name") => Some(Self::UserName),
            _ => None,
        }
    }

    fn from_sillytavern(inner: &str) -> Option<Self> {
        match inner.trim().to_ascii_lowercase().as_str() {
            "char" => Some(Self::CharName),
            "user" => Some(Self::UserName),
            _ => None,
        }
    }
}

/// One piece of card text. Weaver's top-level node kinds each have a token;
/// the SillyTavern dialect only ever produces [`Text`](Token::Text) and
/// [`Macro`](Token::Macro).
#[derive(Debug, Clone)]
pub enum Token<'a> {
    Text {
        source: &'a str,
    },
    Macro {
        source: &'a str,
        macro_ref: MacroRef,
    },
    /// Any other `{{…}}` expression: variables, processors, documents.
    Expression {
        source: &'a str,
        expr: ExprKind,
    },
    Command {
        source: &'a str,
        call: CommandCall,
    },
    If {
        source: &'a str,
        block: IfBlock,
    },
    ForEach {
        source: &'a str,
        block: ForEachBlock,
    },
    Break {
        source: &'a str,
    },
    Continue {
        source: &'a str,
    },
    Return {
        source: &'a str,
        value: Option<Expr>,
    },
    Stop {
        source: &'a str,
    },
}

impl<'a> Token<'a> {
    /// The exact text this token was read from.
    pub fn source(&self) -> &'a str {
        match self {
            Self::Text { source }
            | Self::Macro { source, .. }
            | Self::Expression { source, .. }
            | Self::Command { source, .. }
            | Self::If { source, .. }
            | Self::ForEach { source, .. }
            | Self::Break { source }
            | Self::Continue { source }
            | Self::Return { source, .. }
            | Self::Stop { source } => source,
        }
    }

    /// Whether every dialect can express this token.
    pub fn is_portable(&self) -> bool {
        matches!(self, Self::Text { .. } | Self::Macro { .. })
    }
}

/// Read Weaver text. Text that doesn't parse comes back as one `Text` token,
/// so a conversion never fails on a field.
pub fn weaver_tokens(source: &str) -> Vec<Token<'_>> {
    let Ok(template) = weaver_lang::parse(source) else {
        return text_only(source);
    };

    let mut tokens = Vec::with_capacity(template.nodes.len());
    let mut cursor = 0;
    for node in template.nodes {
        let (start, end) = (node.span.start, node.span.end);
        let (Some(_), Some(slice)) = (source.get(cursor..start), source.get(start..end)) else {
            return text_only(source);
        };
        // Standalone commands swallow their line; keep anything the parser
        // skipped between nodes as text so the source stays byte-exact.
        if start > cursor {
            tokens.push(Token::Text {
                source: &source[cursor..start],
            });
        }
        cursor = end;

        tokens.push(match node.node {
            NodeKind::Literal(_) => Token::Text { source: slice },
            NodeKind::Expression(ExprKind::Variable(var)) => {
                match var
                    .scope
                    .as_deref()
                    .filter(|_| var.path.is_empty())
                    .and_then(|scope| MacroRef::from_weaver(scope, &var.name))
                {
                    Some(macro_ref) => Token::Macro {
                        source: slice,
                        macro_ref,
                    },
                    None => Token::Expression {
                        source: slice,
                        expr: ExprKind::Variable(var),
                    },
                }
            }
            NodeKind::Expression(expr) => Token::Expression {
                source: slice,
                expr,
            },
            NodeKind::Command(call) => Token::Command {
                source: slice,
                call,
            },
            NodeKind::IfBlock(block) => Token::If {
                source: slice,
                block,
            },
            NodeKind::ForEach(block) => Token::ForEach {
                source: slice,
                block,
            },
            NodeKind::Break => Token::Break { source: slice },
            NodeKind::Continue => Token::Continue { source: slice },
            NodeKind::Return(value) => Token::Return {
                source: slice,
                value,
            },
            NodeKind::Stop => Token::Stop { source: slice },
        });
    }
    if cursor < source.len() {
        tokens.push(Token::Text {
            source: &source[cursor..],
        });
    }
    tokens
}

/// Read SillyTavern text: prose with `{{macro}}`s. Macros without a Weaver
/// spelling (`{{random::a::b}}`, …) stay text.
pub fn sillytavern_tokens(source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut text_start = 0;
    let mut cursor = 0;
    while let Some(open) = source[cursor..].find("{{").map(|i| cursor + i) {
        let Some(close) = source[open + 2..].find("}}").map(|i| open + 2 + i) else {
            break;
        };
        let end = close + 2;
        match MacroRef::from_sillytavern(&source[open + 2..close]) {
            Some(macro_ref) => {
                if open > text_start {
                    tokens.push(Token::Text {
                        source: &source[text_start..open],
                    });
                }
                tokens.push(Token::Macro {
                    source: &source[open..end],
                    macro_ref,
                });
                text_start = end;
                cursor = end;
            }
            None => cursor = open + 2,
        }
    }
    if text_start < source.len() {
        tokens.push(Token::Text {
            source: &source[text_start..],
        });
    }
    tokens
}

fn text_only(source: &str) -> Vec<Token<'_>> {
    if source.is_empty() {
        Vec::new()
    } else {
        vec![Token::Text { source }]
    }
}

pub fn render_weaver(tokens: &[Token<'_>]) -> String {
    tokens
        .iter()
        .map(|t| match t {
            Token::Macro { macro_ref, .. } => macro_ref.weaver(),
            other => other.source(),
        })
        .collect()
}

pub fn render_sillytavern(tokens: &[Token<'_>]) -> String {
    tokens
        .iter()
        .map(|t| match t {
            Token::Macro { macro_ref, .. } => macro_ref.sillytavern(),
            other => other.source(),
        })
        .collect()
}

/// Weaver text rewritten for SillyTavern.
pub struct Converted {
    pub text: String,
    /// False when the text uses Weaver that SillyTavern can't run; that part
    /// is copied over unchanged.
    pub portable: bool,
}

pub fn weaver_to_sillytavern(source: &str) -> Converted {
    let tokens = weaver_tokens(source);
    Converted {
        text: render_sillytavern(&tokens),
        portable: tokens.iter().all(Token::is_portable),
    }
}

pub fn sillytavern_to_weaver(source: &str) -> String {
    render_weaver(&sillytavern_tokens(source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weaver_tokens_keep_the_source_exact() {
        let source = "Hi {{user:name}}, I'm {{char:name}}.\n{# if {{local:x}} #}yes{# endif #} {{local:mood}}";
        let tokens = weaver_tokens(source);
        let rebuilt: String = tokens.iter().map(Token::source).collect();
        assert_eq!(rebuilt, source);
        assert!(tokens.iter().any(|t| matches!(t, Token::If { .. })));
        assert!(tokens.iter().any(|t| matches!(
            t,
            Token::Macro {
                macro_ref: MacroRef::UserName,
                ..
            }
        )));
        assert!(tokens.iter().any(|t| matches!(t, Token::Expression { .. })));
    }

    #[test]
    fn macros_convert_both_ways() {
        let weaver = "{{char:name}} greets {{user:name}}.";
        let st = weaver_to_sillytavern(weaver);
        assert_eq!(st.text, "{{char}} greets {{user}}.");
        assert!(st.portable);
        assert_eq!(sillytavern_to_weaver(&st.text), weaver);
    }

    #[test]
    fn weaver_constructs_pass_through_and_mark_the_field() {
        let weaver = "{{char:name}} {# if {{local:x}} #}a{# endif #}";
        let st = weaver_to_sillytavern(weaver);
        assert_eq!(st.text, "{{char}} {# if {{local:x}} #}a{# endif #}");
        assert!(!st.portable);
    }

    #[test]
    fn sillytavern_macros_are_case_insensitive_and_unknown_ones_stay() {
        assert_eq!(
            sillytavern_to_weaver("{{Char}} rolls {{random::1::2}} for {{ user }}"),
            "{{char:name}} rolls {{random::1::2}} for {{user:name}}"
        );
    }

    #[test]
    fn unparseable_weaver_is_one_text_token() {
        let tokens = weaver_tokens("broken {{ value");
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_portable());
    }

    #[test]
    fn unclosed_sillytavern_braces_are_text() {
        assert_eq!(sillytavern_to_weaver("a {{char"), "a {{char");
    }
}

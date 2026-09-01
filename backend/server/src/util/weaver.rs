use std::collections::BTreeMap;

use thiserror::Error;
use weaver_lang::ParseError;

/// A value that has no counterpart on the other side of the bridge.
#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("number not representable as f64: {0}")]
    UnrepresentableNumber(serde_json::Number),
    #[error("non-finite number cannot be stored as JSON: {0}")]
    NonFiniteNumber(f64),
}

pub fn json_to_weaver(v: &serde_json::Value) -> Result<weaver_lang::Value, ConversionError> {
    Ok(match v {
        serde_json::Value::String(s) => weaver_lang::Value::String(s.clone()),
        serde_json::Value::Number(n) => weaver_lang::Value::Number(
            n.as_f64()
                .ok_or_else(|| ConversionError::UnrepresentableNumber(n.clone()))?,
        ),
        serde_json::Value::Bool(b) => weaver_lang::Value::Bool(*b),
        serde_json::Value::Array(items) => {
            weaver_lang::Value::Array(items.iter().map(json_to_weaver).collect::<Result<_, _>>()?)
        }
        serde_json::Value::Object(fields) => weaver_lang::Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), json_to_weaver(v)?)))
                .collect::<Result<BTreeMap<_, _>, ConversionError>>()?,
        ),
        serde_json::Value::Null => weaver_lang::Value::None,
    })
}

pub fn weaver_to_json(v: &weaver_lang::Value) -> Result<serde_json::Value, ConversionError> {
    Ok(match v {
        weaver_lang::Value::String(s) => serde_json::Value::String(s.clone()),
        weaver_lang::Value::Bool(b) => serde_json::Value::Bool(*b),
        weaver_lang::Value::Number(n) => {
            if !n.is_finite() {
                return Err(ConversionError::NonFiniteNumber(*n));
            }
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                serde_json::Value::Number((*n as i64).into())
            } else {
                serde_json::Value::Number(
                    serde_json::Number::from_f64(*n).expect("finite f64 is representable"),
                )
            }
        }
        weaver_lang::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(weaver_to_json).collect::<Result<_, _>>()?)
        }
        weaver_lang::Value::Object(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), weaver_to_json(v)?)))
                .collect::<Result<_, ConversionError>>()?,
        ),
        weaver_lang::Value::None => serde_json::Value::Null,
    })
}

/// Parser output reduced to one readable line.
pub fn summarize_parser_text(text: &str) -> String {
    let summary = text
        .lines()
        .rev()
        .find_map(|line| line.trim_start().strip_prefix("= "));

    let line = match summary {
        Some(summary) => summary,
        None => text.lines().next().unwrap_or(text),
    };
    let line = line.trim();
    let line = line.strip_prefix("parse error:").unwrap_or(line).trim();

    // Nothing but pest's location header survived, which says nothing a
    // caller can act on.
    if line.is_empty() || line.starts_with("-->") {
        return "invalid syntax".to_string();
    }
    let line = line.split_whitespace().collect::<Vec<_>>().join(" ");

    match line.strip_prefix("expected ") {
        Some(rules) => format!("expected {}", name_rules(rules)),
        None => line,
    }
}

/// Grammar rules under the names an author would use for them.
const RULE_NAMES: &[(&str, &str)] = &[
    ("identifier", "a name"),
    ("dotted_name", "a dotted name"),
    ("trim_l", "a trim marker (-)"),
    ("trim_r", "a trim marker (-) or a closing delimiter"),
    ("expr", "an expression"),
    ("unary_expr", "an expression"),
    ("atom", "an expression"),
    ("bin_op", "an operator"),
    ("unary_op", "an operator"),
    ("quoted_string", "a quoted string"),
    ("string_inner", "string contents"),
    ("escape_seq", "an escape sequence"),
    ("number", "a number"),
    ("bool_literal", "true or false"),
    ("none_literal", "none"),
    ("array_literal", "an array"),
    ("variable", "a variable"),
    ("processor_call", "a processor call"),
    ("command_call", "a command call"),
    ("trigger", "a trigger"),
    ("document_ref", "a document reference"),
    ("document_id", "a document id"),
    ("property", "a property"),
    ("property_list", "a property"),
    ("arg_list", "an argument"),
    ("literal_text", "text"),
    ("node", "content"),
    ("if_close", "{# endif #}"),
    ("foreach_close", "{# endforeach #}"),
    ("elif_branch", "{# elif ... #}"),
    ("else_branch", "{# else #}"),
    ("EOI", "end of input"),
];

/// Rewrite pest's `a, b, or c` rule list into the same list in English.
fn name_rules(rules: &str) -> String {
    let mut named: Vec<String> = Vec::new();
    for rule in rules.split(',').flat_map(|part| part.split(" or ")) {
        let rule = rule.trim();
        if rule.is_empty() {
            continue;
        }
        let name = RULE_NAMES
            .iter()
            .find(|(grammar, _)| *grammar == rule)
            .map(|(_, name)| (*name).to_string())
            .unwrap_or_else(|| rule.replace('_', " "));
        // Several rules share a name, and repeating it helps nobody.
        if !named.contains(&name) {
            named.push(name);
        }
    }

    match named.as_slice() {
        [] => rules.to_string(),
        [one] => one.clone(),
        [first, second] => format!("{first} or {second}"),
        [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
    }
}

/// One parse failure as a sentence
pub fn describe_parse_error(source: &str, error: &ParseError) -> String {
    let (line, column) = line_col(source, error.span.start);
    format!(
        "{} (line {line}, column {column})",
        summarize_parser_text(&error.message)
    )
}

pub fn describe_parse_errors(source: &str, errors: &[ParseError]) -> String {
    match errors {
        [] => "invalid syntax".to_string(),
        [only] => describe_parse_error(source, only),
        [first, rest @ ..] => {
            format!(
                "{} and {} more",
                describe_parse_error(source, first),
                rest.len()
            )
        }
    }
}

/// The 1-based line and column a byte offset falls on
fn line_col(source: &str, byte: usize) -> (usize, usize) {
    let mut byte = byte.min(source.len());
    while !source.is_char_boundary(byte) {
        byte -= 1;
    }
    let before = &source[..byte];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().unwrap_or(before).chars().count() + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn compile_errors(source: &str) -> Vec<ParseError> {
        weaver_lang::CompiledTemplate::compile(source)
            .err()
            .expect("source is meant to be broken")
    }

    fn round_trip(v: serde_json::Value) -> serde_json::Value {
        weaver_to_json(&json_to_weaver(&v).unwrap()).unwrap()
    }

    #[test]
    fn a_syntax_error_reads_as_one_line() {
        let source = "{{";
        let errors = compile_errors(source);
        let described = describe_parse_errors(source, &errors);

        // Whatever pest expected, none of its drawing comes through.
        assert!(!described.contains('\n'), "{described}");
        assert!(!described.contains("-->"), "{described}");
        assert!(!described.contains('^'), "{described}");
        assert!(described.starts_with("expected "), "{described}");
        assert!(described.ends_with("(line 1, column 3)"), "{described}");
    }

    #[test]
    fn a_flow_error_keeps_its_own_sentence() {
        let source = "{# break #}";
        let errors = compile_errors(source);
        assert_eq!(
            describe_parse_errors(source, &errors),
            "`break` outside of a loop (line 1, column 1)"
        );
    }

    #[test]
    fn only_the_first_of_several_is_spelled_out() {
        let source = "{# break #}{# continue #}";
        let errors = compile_errors(source);
        assert_eq!(errors.len(), 2);
        assert!(describe_parse_errors(source, &errors).ends_with(" and 1 more"));
    }

    #[test]
    fn position_counts_lines_and_characters_not_bytes() {
        let source = "café\n  {# break #}";
        let errors = compile_errors(source);
        assert!(
            describe_parse_errors(source, &errors).ends_with("(line 2, column 3)"),
            "{}",
            describe_parse_errors(source, &errors)
        );
    }

    #[test]
    fn runs_of_whitespace_collapse() {
        assert_eq!(summarize_parser_text("a    b\t\tc"), "a b c");
    }

    #[test]
    fn grammar_rule_names_are_translated() {
        assert_eq!(
            summarize_parser_text("  = expected identifier, dotted_name, or trim_l"),
            "expected a name, a dotted name, or a trim marker (-)"
        );
        assert_eq!(
            summarize_parser_text("  = expected unary_expr"),
            "expected an expression"
        );
    }

    #[test]
    fn rules_sharing_a_name_are_not_listed_twice() {
        assert_eq!(
            summarize_parser_text("  = expected expr, unary_expr, or atom"),
            "expected an expression"
        );
    }

    #[test]
    fn an_unmapped_rule_still_reads_as_words() {
        assert_eq!(
            summarize_parser_text("  = expected some_future_rule"),
            "expected some future rule"
        );
    }

    #[test]
    fn nested_objects_round_trip_unchanged() {
        let original = json!({ "stats": { "hp": 10, "tags": ["a", "b"] }, "alive": true });
        assert_eq!(round_trip(original.clone()), original);
    }

    #[test]
    fn whole_numbers_keep_their_integer_form() {
        assert_eq!(round_trip(json!(10)), json!(10));
        assert_eq!(round_trip(json!(-3)), json!(-3));
        assert_eq!(round_trip(json!(1.5)), json!(1.5));
    }

    #[test]
    fn null_and_empty_object_stay_distinct() {
        assert_eq!(
            json_to_weaver(&json!(null)).unwrap(),
            weaver_lang::Value::None
        );
        assert_eq!(
            json_to_weaver(&json!({})).unwrap(),
            weaver_lang::Value::Object(BTreeMap::new())
        );
    }

    #[test]
    fn non_finite_numbers_are_rejected_rather_than_stored() {
        assert!(weaver_to_json(&weaver_lang::Value::Number(f64::NAN)).is_err());
        assert!(weaver_to_json(&weaver_lang::Value::Number(f64::INFINITY)).is_err());
    }

    #[test]
    fn large_integers_widen_lossily_rather_than_failing() {
        let widened = json_to_weaver(&json!(u64::MAX)).unwrap();
        assert_eq!(widened, weaver_lang::Value::Number(u64::MAX as f64));
        assert!(weaver_to_json(&widened).unwrap().is_f64());
    }
}

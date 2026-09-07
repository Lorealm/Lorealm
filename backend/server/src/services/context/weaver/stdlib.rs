use context_weaver::Plugin;
use weaver_lang::{EvalError, EvalErrorKind, Registry, Value};
use weaver_macros::weaver_processor;

/// Registers every standard-library command and processor.
pub struct StdlibPlugin;

impl Plugin for StdlibPlugin {
    fn name(&self) -> &str {
        "stdlib"
    }

    fn register(&self, registry: &mut Registry) {
        register_all(registry);
    }
}

/// Registers every command and processor onto `registry`.
pub fn register_all(registry: &mut Registry) {
    context_weaver::stdlib::register(registry);
    register_processors(registry);
}

/// Registers the `text.*`, `math.*`, and `array.*` processors.
pub fn register_processors(registry: &mut Registry) {
    register_text(registry);
    register_math(registry);
    register_array(registry);
    register_random(registry);
}

fn register_text(registry: &mut Registry) {
    registry.register_processor(TextUpperProcessor);
    registry.register_processor(TextLowerProcessor);
    registry.register_processor(TextLengthProcessor);
    registry.register_processor(TextTrimProcessor);
    registry.register_processor(TextCapitalizeProcessor);
    registry.register_processor(TextContainsProcessor);
    registry.register_processor(TextStartsWithProcessor);
    registry.register_processor(TextEndsWithProcessor);
    registry.register_processor(TextReplaceProcessor);
    registry.register_processor(TextSubstrProcessor);
    registry.register_processor(TextJoinProcessor);
    registry.register_processor(TextRepeatProcessor);
}

/// `@[text.upper(text: "hello")]` → `"HELLO"`
#[weaver_processor(namespace = "text", name = "upper", returns = "string")]
fn text_upper(text: String) -> Result<Value, EvalError> {
    Ok(Value::String(text.to_uppercase()))
}

/// `@[text.lower(text: "HELLO")]` → `"hello"`
#[weaver_processor(namespace = "text", name = "lower", returns = "string")]
fn text_lower(text: String) -> Result<Value, EvalError> {
    Ok(Value::String(text.to_lowercase()))
}

/// `@[text.length(text: "hello")]` → `5` (counts Unicode scalar values).
#[weaver_processor(namespace = "text", name = "length", returns = "number")]
fn text_length(text: String) -> Result<Value, EvalError> {
    Ok(Value::Number(text.chars().count() as f64))
}

/// `@[text.trim(text: "  hello  ")]` → `"hello"`
#[weaver_processor(namespace = "text", name = "trim", returns = "string")]
fn text_trim(text: String) -> Result<Value, EvalError> {
    Ok(Value::String(text.trim().to_string()))
}

/// `@[text.capitalize(text: "hello world")]` → `"Hello world"`
#[weaver_processor(namespace = "text", name = "capitalize", returns = "string")]
fn text_capitalize(text: String) -> Result<Value, EvalError> {
    let mut chars = text.chars();
    let capitalized = match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    };
    Ok(Value::String(capitalized))
}

/// `@[text.contains(text: "hello world", substring: "world")]` → `true`
#[weaver_processor(namespace = "text", name = "contains", returns = "bool")]
fn text_contains(text: String, substring: String) -> Result<Value, EvalError> {
    Ok(Value::Bool(text.contains(&substring)))
}

/// `@[text.starts_with(text: "hello", prefix: "hel")]` → `true`
#[weaver_processor(namespace = "text", name = "starts_with", returns = "bool")]
fn text_starts_with(text: String, prefix: String) -> Result<Value, EvalError> {
    Ok(Value::Bool(text.starts_with(&prefix)))
}

/// `@[text.ends_with(text: "hello", suffix: "llo")]` → `true`
#[weaver_processor(namespace = "text", name = "ends_with", returns = "bool")]
fn text_ends_with(text: String, suffix: String) -> Result<Value, EvalError> {
    Ok(Value::Bool(text.ends_with(&suffix)))
}

/// `@[text.replace(text: "hello world", from: "world", to: "rust")]` → `"hello rust"`
#[weaver_processor(namespace = "text", name = "replace", returns = "string")]
fn text_replace(text: String, from: String, to: String) -> Result<Value, EvalError> {
    Ok(Value::String(text.replace(&from, &to)))
}

/// `@[text.substr(text: "hello", start: 1, length: 3)]` → `"ell"`
///
/// `length` is optional; when omitted the substring runs to the end.
#[weaver_processor(namespace = "text", name = "substr", returns = "string")]
fn text_substr(text: String, start: f64, length: Option<f64>) -> Result<Value, EvalError> {
    let start = start as usize;
    let result: String = match length {
        Some(len) => text.chars().skip(start).take(len as usize).collect(),
        None => text.chars().skip(start).collect(),
    };
    Ok(Value::String(result))
}

/// `@[text.join(items: ["a", "b", "c"], separator: ", ")]` → `"a, b, c"`
///
/// `separator` is optional and defaults to `", "`.
#[weaver_processor(namespace = "text", name = "join", returns = "string")]
fn text_join(items: Vec<Value>, separator: Option<String>) -> Result<Value, EvalError> {
    let sep = separator.as_deref().unwrap_or(", ");
    let joined: String = items
        .iter()
        .map(|v| v.to_output_string())
        .collect::<Vec<_>>()
        .join(sep);
    Ok(Value::String(joined))
}

/// `@[text.repeat(text: "ha", count: 3)]` → `"hahaha"`
#[weaver_processor(namespace = "text", name = "repeat", returns = "string")]
fn text_repeat(text: String, count: f64) -> Result<Value, EvalError> {
    Ok(Value::String(text.repeat(count as usize)))
}

fn register_math(registry: &mut Registry) {
    registry.register_processor(MathAddProcessor);
    registry.register_processor(MathSubProcessor);
    registry.register_processor(MathMulProcessor);
    registry.register_processor(MathDivProcessor);
    registry.register_processor(MathModProcessor);
    registry.register_processor(MathAbsProcessor);
    registry.register_processor(MathMinProcessor);
    registry.register_processor(MathMaxProcessor);
    registry.register_processor(MathClampProcessor);
    registry.register_processor(MathFloorProcessor);
    registry.register_processor(MathCeilProcessor);
    registry.register_processor(MathRoundProcessor);
}

/// `@[math.add(a: 1, b: 2)]` → `3`
#[weaver_processor(namespace = "math", name = "add", returns = "number")]
fn math_add(a: f64, b: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(a + b))
}

/// `@[math.sub(a: 5, b: 3)]` → `2`
#[weaver_processor(namespace = "math", name = "sub", returns = "number")]
fn math_sub(a: f64, b: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(a - b))
}

/// `@[math.mul(a: 2, b: 3)]` → `6`
#[weaver_processor(namespace = "math", name = "mul", returns = "number")]
fn math_mul(a: f64, b: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(a * b))
}

/// `@[math.div(a: 10, b: 2)]` → `5`. Errors when `b` is 0.
#[weaver_processor(namespace = "math", name = "div", returns = "number")]
fn math_div(a: f64, b: f64) -> Result<Value, EvalError> {
    if b == 0.0 {
        return Err(EvalError::new(
            EvalErrorKind::HostError,
            "division by zero".to_string(),
        ));
    }
    Ok(Value::Number(a / b))
}

/// `@[math.mod(a: 10, b: 3)]` → `1`. Errors when `b` is 0.
#[weaver_processor(namespace = "math", name = "mod", returns = "number")]
fn math_mod(a: f64, b: f64) -> Result<Value, EvalError> {
    if b == 0.0 {
        return Err(EvalError::new(
            EvalErrorKind::HostError,
            "modulo by zero".to_string(),
        ));
    }
    Ok(Value::Number(a % b))
}

/// `@[math.abs(value: -5)]` → `5`
#[weaver_processor(namespace = "math", name = "abs", returns = "number")]
fn math_abs(value: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(value.abs()))
}

/// `@[math.min(a: 3, b: 7)]` → `3`
#[weaver_processor(namespace = "math", name = "min", returns = "number")]
fn math_min(a: f64, b: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(a.min(b)))
}

/// `@[math.max(a: 3, b: 7)]` → `7`
#[weaver_processor(namespace = "math", name = "max", returns = "number")]
fn math_max(a: f64, b: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(a.max(b)))
}

/// `@[math.clamp(value: 150, min: 0, max: 100)]` → `100`
#[weaver_processor(namespace = "math", name = "clamp", returns = "number")]
fn math_clamp(value: f64, min: f64, max: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(value.max(min).min(max)))
}

/// `@[math.floor(value: 3.7)]` → `3`
#[weaver_processor(namespace = "math", name = "floor", returns = "number")]
fn math_floor(value: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(value.floor()))
}

/// `@[math.ceil(value: 3.2)]` → `4`
#[weaver_processor(namespace = "math", name = "ceil", returns = "number")]
fn math_ceil(value: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(value.ceil()))
}

/// `@[math.round(value: 3.5)]` → `4`
#[weaver_processor(namespace = "math", name = "round", returns = "number")]
fn math_round(value: f64) -> Result<Value, EvalError> {
    Ok(Value::Number(value.round()))
}

fn register_array(registry: &mut Registry) {
    registry.register_processor(ArrayLengthProcessor);
    registry.register_processor(ArrayContainsProcessor);
    registry.register_processor(ArrayFirstProcessor);
    registry.register_processor(ArrayLastProcessor);
    registry.register_processor(ArrayReverseProcessor);
    registry.register_processor(ArraySliceProcessor);
    registry.register_processor(ArrayRangeProcessor);
    registry.register_processor(ArrayConcatProcessor);
}

/// `@[array.length(items: [1, 2, 3])]` → `3`
#[weaver_processor(namespace = "array", name = "length", returns = "number")]
fn array_length(items: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Number(items.len() as f64))
}

/// `@[array.contains(items: ["a", "b"], value: "a")]` → `true`
#[weaver_processor(namespace = "array", name = "contains", returns = "bool")]
fn array_contains(items: Vec<Value>, value: Value) -> Result<Value, EvalError> {
    Ok(Value::Bool(items.contains(&value)))
}

/// `@[array.first(items: [1, 2, 3])]` → `1`. `none` for an empty array.
#[weaver_processor(namespace = "array", name = "first", returns = "any")]
fn array_first(items: Vec<Value>) -> Result<Value, EvalError> {
    Ok(items.into_iter().next().unwrap_or(Value::None))
}

/// `@[array.last(items: [1, 2, 3])]` → `3`. `none` for an empty array.
#[weaver_processor(namespace = "array", name = "last", returns = "any")]
fn array_last(items: Vec<Value>) -> Result<Value, EvalError> {
    Ok(items.into_iter().last().unwrap_or(Value::None))
}

/// `@[array.reverse(items: [1, 2, 3])]` → `[3, 2, 1]`
#[weaver_processor(namespace = "array", name = "reverse", returns = "array")]
fn array_reverse(items: Vec<Value>) -> Result<Value, EvalError> {
    let mut reversed = items;
    reversed.reverse();
    Ok(Value::Array(reversed))
}

/// `@[array.slice(items: [1, 2, 3, 4], start: 1, end: 3)]` → `[2, 3]`
///
/// `end` is optional; when omitted the slice runs to the end. Both bounds are
/// clamped to the array length.
#[weaver_processor(namespace = "array", name = "slice", returns = "array")]
fn array_slice(items: Vec<Value>, start: f64, end: Option<f64>) -> Result<Value, EvalError> {
    let start = start as usize;
    let end = end.map(|n| n as usize).unwrap_or(items.len());
    let sliced = items
        .get(start..end.min(items.len()))
        .unwrap_or(&[])
        .to_vec();
    Ok(Value::Array(sliced))
}

/// `@[array.range(start: 1, end: 4)]` → `[1, 2, 3]` (end-exclusive).
#[weaver_processor(namespace = "array", name = "range", returns = "array")]
fn array_range(start: f64, end: f64) -> Result<Value, EvalError> {
    let (start, end) = (start as i64, end as i64);
    let range: Vec<Value> = (start..end).map(|n| Value::Number(n as f64)).collect();
    Ok(Value::Array(range))
}

/// `@[array.concat(a: [1, 2], b: [3, 4])]` → `[1, 2, 3, 4]`
#[weaver_processor(namespace = "array", name = "concat", returns = "array")]
fn array_concat(a: Vec<Value>, b: Vec<Value>) -> Result<Value, EvalError> {
    let mut result = a;
    result.extend(b);
    Ok(Value::Array(result))
}

fn register_random(registry: &mut Registry) {
    registry.register_processor(GenerateRandomProcessor);
    registry.register_processor(PickRandomProcessor);
}

/// Generate a random integer in the half-open range `[min, max)`.
#[weaver_processor(
    namespace = "random",
    name = "gen",
    returns = "number",
    params(min = "The minimum number", max = "The maximum number")
)]
pub fn generate_random(min: f64, max: f64) -> Result<Value, EvalError> {
    let min = min as i32;
    let max = max as i32;
    tracing::debug!("Generating random between {min} and {max}");
    Ok(Value::Number(rand::random_range(min..max) as f64))
}

/// Pick one item from an array at random.
#[weaver_processor(
    namespace = "random",
    name = "pick",
    returns = "any",
    params(items = "items to pick from")
)]
pub fn pick_random(items: Vec<Value>) -> Result<Value, EvalError> {
    let min = 0;
    let max = items.len();

    let idx = rand::random_range(min..max);
    let item = items[idx].clone();
    tracing::debug!("Picked random item: {item}");

    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use weaver_lang::EvalContext;

    struct SimpleContext {
        variables: HashMap<String, HashMap<String, Value>>,
    }

    impl SimpleContext {
        fn new() -> Self {
            Self {
                variables: HashMap::new(),
            }
        }

        fn set(&mut self, scope: &str, name: &str, value: impl Into<Value>) {
            self.variables
                .entry(scope.to_string())
                .or_default()
                .insert(name.to_string(), value.into());
        }
    }

    impl EvalContext for SimpleContext {
        fn resolve_variable(&self, scope: &str, name: &str) -> Result<Option<Value>, EvalError> {
            Ok(self
                .variables
                .get(scope)
                .and_then(|ns| ns.get(name))
                .cloned())
        }

        fn set_variable(&mut self, scope: &str, name: &str, value: Value) -> Result<(), EvalError> {
            self.variables
                .entry(scope.to_string())
                .or_default()
                .insert(name.to_string(), value);
            Ok(())
        }

        fn fire_trigger(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
            Ok(String::new())
        }

        fn resolve_document(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
            Ok(String::new())
        }
    }

    /// The var commands now come from context_weaver; these stay as a
    /// contract test that they still behave as Lorealm expects.
    fn make_registry() -> Registry {
        let mut r = Registry::new();
        context_weaver::stdlib::register(&mut r);
        r
    }

    #[test]
    fn set_then_get() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();

        registry
            .call_command(
                "set_var",
                vec![
                    Value::String("global:name".into()),
                    Value::String("Kael".into()),
                ],
                &mut ctx,
            )
            .unwrap();

        let got = registry
            .call_command(
                "get_var",
                vec![Value::String("global:name".into())],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(got, Some(Value::String("Kael".into())));
    }

    #[test]
    fn get_missing_returns_none() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();

        let got = registry
            .call_command(
                "get_var",
                vec![Value::String("global:missing".into())],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(got, Some(Value::None));
    }

    #[test]
    fn var_exists_cases() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();
        ctx.set("global", "hp", 100i64);

        let present = registry
            .call_command(
                "var_exists",
                vec![Value::String("global:hp".into())],
                &mut ctx,
            )
            .unwrap();
        let absent = registry
            .call_command(
                "var_exists",
                vec![Value::String("global:nope".into())],
                &mut ctx,
            )
            .unwrap();
        let malformed = registry.call_command(
            "var_exists",
            vec![Value::String("not_a_key".into())],
            &mut ctx,
        );

        assert_eq!(present, Some(Value::Bool(true)));
        assert_eq!(absent, Some(Value::Bool(false)));
        assert!(malformed.is_err());
    }

    #[test]
    fn inc_var_initializes() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();

        registry
            .call_command(
                "inc_var",
                vec![Value::String("global:score".into()), Value::Number(5.0)],
                &mut ctx,
            )
            .unwrap();

        assert_eq!(
            ctx.resolve_variable("global", "score").unwrap(),
            Some(Value::Number(5.0))
        );
    }

    #[test]
    fn inc_var_increments_existing() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();
        ctx.set("global", "score", 10i64);

        registry
            .call_command(
                "inc_var",
                vec![Value::String("global:score".into()), Value::Number(3.0)],
                &mut ctx,
            )
            .unwrap();

        assert_eq!(
            ctx.resolve_variable("global", "score").unwrap(),
            Some(Value::Number(13.0))
        );
    }

    #[test]
    fn inc_var_default_increment() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();
        ctx.set("global", "count", 0i64);

        registry
            .call_command(
                "inc_var",
                vec![Value::String("global:count".into())],
                &mut ctx,
            )
            .unwrap();

        assert_eq!(
            ctx.resolve_variable("global", "count").unwrap(),
            Some(Value::Number(1.0))
        );
    }

    #[test]
    fn push_var_creates_then_appends() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();

        registry
            .call_command(
                "push_var",
                vec![
                    Value::String("global:log".into()),
                    Value::String("a".into()),
                ],
                &mut ctx,
            )
            .unwrap();
        registry
            .call_command(
                "push_var",
                vec![
                    Value::String("global:log".into()),
                    Value::String("b".into()),
                ],
                &mut ctx,
            )
            .unwrap();

        assert_eq!(
            ctx.resolve_variable("global", "log").unwrap(),
            Some(Value::Array(vec![
                Value::String("a".into()),
                Value::String("b".into()),
            ]))
        );
    }

    #[test]
    fn default_var_sets_then_skips() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();

        registry
            .call_command(
                "default_var",
                vec![Value::String("global:x".into()), Value::Number(42.0)],
                &mut ctx,
            )
            .unwrap();
        registry
            .call_command(
                "default_var",
                vec![Value::String("global:x".into()), Value::Number(99.0)],
                &mut ctx,
            )
            .unwrap();

        assert_eq!(
            ctx.resolve_variable("global", "x").unwrap(),
            Some(Value::Number(42.0))
        );
    }

    #[test]
    fn invalid_key_rejected() {
        let registry = make_registry();
        let mut ctx = SimpleContext::new();

        let ok = registry
            .call_command(
                "set_var",
                vec![Value::String("global:hp".into()), Value::Number(10.0)],
                &mut ctx,
            )
            .unwrap();
        let bad = registry.call_command(
            "set_var",
            vec![Value::String("{global:hp}".into()), Value::Number(10.0)],
            &mut ctx,
        );

        assert_eq!(ok, None);
        assert!(bad.is_err());
    }
}

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use context_weaver::Plugin;
use rand::seq::{IndexedRandom, SliceRandom};
use weaver_lang::{EvalError, EvalErrorKind, Registry, Value};
use weaver_macros::weaver_processor;

use crate::util::weaver::{json_to_weaver, weaver_to_json};

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

/// Registers the `text.*`, `math.*`, `array.*`, `object.*`, `random.*`,
/// `convert.*` and `value.*` processors.
pub fn register_processors(registry: &mut Registry) {
    register_text(registry);
    register_math(registry);
    register_array(registry);
    register_object(registry);
    register_random(registry);
    register_convert(registry);
    register_value(registry);
}

fn type_error(message: String) -> EvalError {
    EvalError::new(EvalErrorKind::TypeError, message)
}

fn host_error(message: String) -> EvalError {
    EvalError::new(EvalErrorKind::HostError, message)
}

/// Follows a dotted `path` (`"stats.hp"`) into `value`. `None` when any step
/// is missing or not an object.
fn lookup<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let segments: Vec<String> = path.split('.').map(str::to_string).collect();
    value.get_path(&segments).ok().flatten()
}

/// Every item as a number, or a type error naming the first one that isn't.
fn numbers(processor: &str, items: &[Value]) -> Result<Vec<f64>, EvalError> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            item.as_number().ok_or_else(|| {
                type_error(format!(
                    "{processor}: item {i} is {}, expected a number",
                    item.type_name()
                ))
            })
        })
        .collect()
}

fn expect_object<'a>(
    processor: &str,
    value: &'a Value,
) -> Result<&'a std::collections::BTreeMap<String, Value>, EvalError> {
    value.as_object().ok_or_else(|| {
        type_error(format!(
            "{processor}: expected an object, got {}",
            value.type_name()
        ))
    })
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
    registry.register_processor(TextSplitProcessor);
    registry.register_processor(TextPadStartProcessor);
    registry.register_processor(TextPadEndProcessor);
    registry.register_processor(TextIndexOfProcessor);
    registry.register_processor(TextTitleProcessor);
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

/// Splits `text` on `separator`. An empty separator splits into characters.
///
/// `@[text.split(text: "a,b,c", separator: ",")]` → `["a", "b", "c"]`
#[weaver_processor(namespace = "text", name = "split", returns = "array")]
fn text_split(text: String, separator: String) -> Result<Value, EvalError> {
    let parts: Vec<Value> = if separator.is_empty() {
        text.chars().map(|c| Value::String(c.to_string())).collect()
    } else {
        text.split(separator.as_str())
            .map(|part| Value::String(part.to_string()))
            .collect()
    };
    Ok(Value::Array(parts))
}

/// Pads `text` to `length` characters with repeated `fill`.
fn pad(
    processor: &str,
    text: String,
    length: f64,
    fill: Option<String>,
    at_start: bool,
) -> Result<Value, EvalError> {
    let fill = fill.unwrap_or_else(|| " ".to_string());
    if fill.is_empty() {
        return Err(host_error(format!("{processor}: fill can't be empty")));
    }
    let missing = (length.max(0.0) as usize).saturating_sub(text.chars().count());
    let padding: String = fill.chars().cycle().take(missing).collect();
    Ok(Value::String(if at_start {
        padding + &text
    } else {
        text + &padding
    }))
}

/// Pads the start of `text` to `length` characters. `fill` defaults to a space.
///
/// `@[text.pad_start(text: "7", length: 3, fill: "0")]` → `"007"`
#[weaver_processor(namespace = "text", name = "pad_start", returns = "string")]
fn text_pad_start(text: String, length: f64, fill: Option<String>) -> Result<Value, EvalError> {
    pad("text.pad_start", text, length, fill, true)
}

/// Pads the end of `text` to `length` characters. `fill` defaults to a space.
///
/// `@[text.pad_end(text: "ab", length: 4, fill: ".")]` → `"ab.."`
#[weaver_processor(namespace = "text", name = "pad_end", returns = "string")]
fn text_pad_end(text: String, length: f64, fill: Option<String>) -> Result<Value, EvalError> {
    pad("text.pad_end", text, length, fill, false)
}

/// Character index of the first `substring` in `text`, or `-1`.
///
/// `@[text.index_of(text: "hello", substring: "l")]` → `2`
#[weaver_processor(namespace = "text", name = "index_of", returns = "number")]
fn text_index_of(text: String, substring: String) -> Result<Value, EvalError> {
    let index = text
        .find(&substring)
        .map(|byte| text[..byte].chars().count() as f64)
        .unwrap_or(-1.0);
    Ok(Value::Number(index))
}

/// Uppercases the first letter of every word.
///
/// `@[text.title(text: "the old tower")]` → `"The Old Tower"`
#[weaver_processor(namespace = "text", name = "title", returns = "string")]
fn text_title(text: String) -> Result<Value, EvalError> {
    let mut out = String::with_capacity(text.len());
    let mut at_word_start = true;
    for c in text.chars() {
        if at_word_start && !c.is_whitespace() {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
        at_word_start = c.is_whitespace();
    }
    Ok(Value::String(out))
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
    registry.register_processor(MathPowProcessor);
    registry.register_processor(MathSqrtProcessor);
    registry.register_processor(MathSignProcessor);
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

/// Rounds `value` to `digits` decimal places (default 0).
///
/// `@[math.round(value: 2.71828, digits: 2)]` → `2.72`
#[weaver_processor(namespace = "math", name = "round", returns = "number")]
fn math_round(value: f64, digits: Option<f64>) -> Result<Value, EvalError> {
    let digits = digits.unwrap_or(0.0).clamp(0.0, 15.0) as i32;
    let factor = 10f64.powi(digits);
    Ok(Value::Number((value * factor).round() / factor))
}

/// `base` raised to `exponent`. Errors on a non-real result.
///
/// `@[math.pow(base: 2, exponent: 10)]` → `1024`
#[weaver_processor(namespace = "math", name = "pow", returns = "number")]
fn math_pow(base: f64, exponent: f64) -> Result<Value, EvalError> {
    let result = base.powf(exponent);
    if result.is_nan() {
        return Err(EvalError::new(
            EvalErrorKind::ArithmeticError,
            format!("math.pow: {base} to the power of {exponent} isn't a real number"),
        ));
    }
    Ok(Value::Number(result))
}

/// Square root of `value`. Errors on a negative number.
///
/// `@[math.sqrt(value: 9)]` → `3`
#[weaver_processor(namespace = "math", name = "sqrt", returns = "number")]
fn math_sqrt(value: f64) -> Result<Value, EvalError> {
    if value < 0.0 {
        return Err(EvalError::new(
            EvalErrorKind::ArithmeticError,
            format!("math.sqrt: {value} is negative"),
        ));
    }
    Ok(Value::Number(value.sqrt()))
}

/// `-1`, `0` or `1` by the sign of `value`.
///
/// `@[math.sign(value: -4)]` → `-1`
#[weaver_processor(namespace = "math", name = "sign", returns = "number")]
fn math_sign(value: f64) -> Result<Value, EvalError> {
    let sign = if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    };
    Ok(Value::Number(sign))
}

fn register_array(registry: &mut Registry) {
    registry.register_processor(ArrayLengthProcessor);
    registry.register_processor(ArrayContainsProcessor);
    registry.register_processor(ArrayFirstProcessor);
    registry.register_processor(ArrayLastProcessor);
    registry.register_processor(ArrayAtProcessor);
    registry.register_processor(ArrayReverseProcessor);
    registry.register_processor(ArraySliceProcessor);
    registry.register_processor(ArrayRangeProcessor);
    registry.register_processor(ArrayConcatProcessor);
    registry.register_processor(ArrayPluckProcessor);
    registry.register_processor(ArraySumProcessor);
    registry.register_processor(ArrayMinProcessor);
    registry.register_processor(ArrayMaxProcessor);
    registry.register_processor(ArraySortProcessor);
    registry.register_processor(ArrayUniqueProcessor);
    registry.register_processor(ArrayIndexOfProcessor);
    registry.register_processor(ArrayJoinProcessor);
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

/// `@[array.at(items: ["a", "b", "c"], index: 1)]` → `"b"`. `none` when the
/// index falls outside the array; negative indices count from the end.
#[weaver_processor(namespace = "array", name = "at", returns = "any")]
fn array_at(items: Vec<Value>, index: f64) -> Result<Value, EvalError> {
    let index = index as i64;
    let resolved = if index < 0 {
        items.len() as i64 + index
    } else {
        index
    };
    Ok(usize::try_from(resolved)
        .ok()
        .and_then(|i| items.into_iter().nth(i))
        .unwrap_or(Value::None))
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

/// The `key` field of each item, or `none` where missing. `key` may be a dotted path.
///
/// `@[array.pluck(items: [{name: "a"}, {name: "b"}], key: "name")]` → `["a", "b"]`
#[weaver_processor(namespace = "array", name = "pluck", returns = "array")]
fn array_pluck(items: Vec<Value>, key: String) -> Result<Value, EvalError> {
    Ok(Value::Array(
        items
            .iter()
            .map(|item| lookup(item, &key).cloned().unwrap_or(Value::None))
            .collect(),
    ))
}

/// Sum of numeric `items`. `0` when empty.
///
/// `@[array.sum(items: [1, 2, 3])]` → `6`
#[weaver_processor(namespace = "array", name = "sum", returns = "number")]
fn array_sum(items: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Number(
        numbers("array.sum", &items)?.into_iter().sum(),
    ))
}

/// Smallest of numeric `items`. `none` when empty.
///
/// `@[array.min(items: [3, 1, 2])]` → `1`
#[weaver_processor(namespace = "array", name = "min", returns = "any")]
fn array_min(items: Vec<Value>) -> Result<Value, EvalError> {
    Ok(numbers("array.min", &items)?
        .into_iter()
        .reduce(f64::min)
        .map_or(Value::None, Value::Number))
}

/// Largest of numeric `items`. `none` when empty.
///
/// `@[array.max(items: [3, 1, 2])]` → `3`
#[weaver_processor(namespace = "array", name = "max", returns = "any")]
fn array_max(items: Vec<Value>) -> Result<Value, EvalError> {
    Ok(numbers("array.max", &items)?
        .into_iter()
        .reduce(f64::max)
        .map_or(Value::None, Value::Number))
}

/// Sorts all-number or all-string `items`, ascending unless `descending`.
///
/// `@[array.sort(items: [3, 1, 2])]` → `[1, 2, 3]`
#[weaver_processor(namespace = "array", name = "sort", returns = "array")]
fn array_sort(items: Vec<Value>, descending: Option<bool>) -> Result<Value, EvalError> {
    let mut sorted = items;
    if sorted.iter().all(|v| matches!(v, Value::Number(_))) {
        sorted.sort_by(|a, b| {
            let (Value::Number(a), Value::Number(b)) = (a, b) else {
                unreachable!("checked above")
            };
            a.total_cmp(b)
        });
    } else if sorted.iter().all(|v| matches!(v, Value::String(_))) {
        sorted.sort_by(|a, b| a.as_string().cmp(&b.as_string()));
    } else {
        return Err(type_error(
            "array.sort: items must be all numbers or all strings".to_string(),
        ));
    }
    if descending.unwrap_or(false) {
        sorted.reverse();
    }
    Ok(Value::Array(sorted))
}

/// `items` without duplicates, in first-seen order.
///
/// `@[array.unique(items: [1, 2, 1, 3])]` → `[1, 2, 3]`
#[weaver_processor(namespace = "array", name = "unique", returns = "array")]
fn array_unique(items: Vec<Value>) -> Result<Value, EvalError> {
    let mut seen: Vec<Value> = Vec::with_capacity(items.len());
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    Ok(Value::Array(seen))
}

/// Index of the first `value` in `items`, or `-1`.
///
/// `@[array.index_of(items: ["a", "b"], value: "b")]` → `1`
#[weaver_processor(namespace = "array", name = "index_of", returns = "number")]
fn array_index_of(items: Vec<Value>, value: Value) -> Result<Value, EvalError> {
    let index = items
        .iter()
        .position(|item| *item == value)
        .map_or(-1.0, |i| i as f64);
    Ok(Value::Number(index))
}

/// Alias of `text.join`. `separator` defaults to `", "`.
///
/// `@[array.join(items: ["a", "b", "c"], separator: ", ")]` → `"a, b, c"`
#[weaver_processor(namespace = "array", name = "join", returns = "string")]
fn array_join(items: Vec<Value>, separator: Option<String>) -> Result<Value, EvalError> {
    TextJoinProcessor::execute(items, separator)
}

fn register_object(registry: &mut Registry) {
    registry.register_processor(ObjectKeysProcessor);
    registry.register_processor(ObjectValuesProcessor);
    registry.register_processor(ObjectHasProcessor);
    registry.register_processor(ObjectGetProcessor);
    registry.register_processor(ObjectMergeProcessor);
}

/// `@[object.keys(of: {b: 2, a: 1})]` → `["a", "b"]`.
#[weaver_processor(namespace = "object", name = "keys", returns = "array")]
fn object_keys(of: Value) -> Result<Value, EvalError> {
    let fields = expect_object("object.keys", &of)?;
    Ok(Value::Array(
        fields.keys().cloned().map(Value::String).collect(),
    ))
}

/// Values of `of`, in key order.
///
/// `@[object.values(of: {b: 2, a: 1})]` → `[1, 2]`
#[weaver_processor(namespace = "object", name = "values", returns = "array")]
fn object_values(of: Value) -> Result<Value, EvalError> {
    let fields = expect_object("object.values", &of)?;
    Ok(Value::Array(fields.values().cloned().collect()))
}

/// Whether `of` contains `key`, even if set to `none`. `key` may be a dotted path.
///
/// `@[object.has(of: {hp: 10}, key: "hp")]` → `true`
#[weaver_processor(namespace = "object", name = "has", returns = "bool")]
fn object_has(of: Value, key: String) -> Result<Value, EvalError> {
    expect_object("object.has", &of)?;
    Ok(Value::Bool(lookup(&of, &key).is_some()))
}

/// The `key` field of `of`, or `default` when missing or `none`. `key` may be a dotted path.
///
/// `@[object.get(of: {hp: 10}, key: "mp", default: 0)]` → `0`
#[weaver_processor(namespace = "object", name = "get", returns = "any")]
fn object_get(of: Value, key: String, default: Option<Value>) -> Result<Value, EvalError> {
    expect_object("object.get", &of)?;
    Ok(lookup(&of, &key)
        .filter(|v| !matches!(v, Value::None))
        .cloned()
        .or(default)
        .unwrap_or(Value::None))
}

/// Shallow merge of `a` and `b`; `b` wins on conflicts.
///
/// `@[object.merge(a: {hp: 10, mp: 5}, b: {hp: 3})]` → `{hp: 3, mp: 5}`
#[weaver_processor(namespace = "object", name = "merge", returns = "any")]
fn object_merge(a: Value, b: Value) -> Result<Value, EvalError> {
    let mut merged = expect_object("object.merge", &a)?.clone();
    merged.extend(expect_object("object.merge", &b)?.clone());
    Ok(Value::Object(merged))
}

fn register_random(registry: &mut Registry) {
    registry.register_processor(GenerateRandomProcessor);
    registry.register_processor(RandomFloatProcessor);
    registry.register_processor(RandomChanceProcessor);
    registry.register_processor(PickRandomProcessor);
    registry.register_processor(RandomWeightedProcessor);
    registry.register_processor(RandomShuffleProcessor);
    registry.register_processor(RandomSampleProcessor);
    registry.register_processor(RandomDiceProcessor);
}

/// Random integer from `min` to `max`, inclusive.
///
/// `@[random.gen(min: 1, max: 6)]` → `4`
#[weaver_processor(
    namespace = "random",
    name = "gen",
    returns = "number",
    params(min = "Lower bound, inclusive", max = "Upper bound, inclusive")
)]
pub fn generate_random(min: f64, max: f64) -> Result<Value, EvalError> {
    let (low, high) = (min.ceil(), max.floor());
    if !low.is_finite() || !high.is_finite() || low > high {
        return Err(host_error(format!(
            "random.gen: there's no whole number from {min} to {max}"
        )));
    }
    Ok(Value::Number(
        rand::random_range(low as i64..=high as i64) as f64
    ))
}

/// Random decimal from `min` (default 0) up to `max` (default 1), exclusive.
///
/// `@[random.float()]` → `0.4271`
#[weaver_processor(
    namespace = "random",
    name = "float",
    returns = "number",
    params(
        min = "Lower bound, inclusive (default 0)",
        max = "Upper bound, exclusive (default 1)"
    )
)]
pub fn random_float(min: Option<f64>, max: Option<f64>) -> Result<Value, EvalError> {
    let (min, max) = (min.unwrap_or(0.0), max.unwrap_or(1.0));
    if !min.is_finite() || !max.is_finite() || min > max {
        return Err(host_error(format!(
            "random.float: min ({min}) must not be above max ({max})"
        )));
    }
    if min == max {
        return Ok(Value::Number(min));
    }
    Ok(Value::Number(rand::random_range(min..max)))
}

/// `true` with probability `p`.
///
/// `@[random.chance(p: 0.2)]` → `false`
#[weaver_processor(
    namespace = "random",
    name = "chance",
    returns = "bool",
    params(p = "Probability, from 0 to 1")
)]
pub fn random_chance(p: f64) -> Result<Value, EvalError> {
    if p.is_nan() {
        return Err(host_error(
            "random.chance: p must be a number from 0 to 1".to_string(),
        ));
    }
    Ok(Value::Bool(rand::random_bool(p.clamp(0.0, 1.0))))
}

/// Random item from `items`. `none` when empty.
///
/// `@[random.pick(items: ["a", "b"])]` → `"b"`
#[weaver_processor(
    namespace = "random",
    name = "pick",
    returns = "any",
    params(items = "Items to pick from")
)]
pub fn pick_random(items: Vec<Value>) -> Result<Value, EvalError> {
    Ok(items
        .choose(&mut rand::rng())
        .cloned()
        .unwrap_or(Value::None))
}

/// Random item from `items`, weighted by `weights` or by each item's `key` field.
/// Items weighted 0 or less are never picked; `none` when none can be.
///
/// `@[random.weighted(items: ["a", "b"], weights: [3, 1])]` → `"a"`
#[weaver_processor(
    namespace = "random",
    name = "weighted",
    returns = "any",
    params(
        items = "Items to pick from",
        weights = "One weight per item",
        key = "Field holding each item's weight; may be a dotted path"
    )
)]
pub fn random_weighted(
    items: Vec<Value>,
    weights: Option<Vec<Value>>,
    key: Option<String>,
) -> Result<Value, EvalError> {
    let weights: Vec<f64> = match (weights, key) {
        (Some(weights), None) => {
            if weights.len() != items.len() {
                return Err(host_error(format!(
                    "random.weighted: got {} weights for {} items",
                    weights.len(),
                    items.len()
                )));
            }
            numbers("random.weighted", &weights)?
        }
        (None, Some(key)) => items
            .iter()
            .enumerate()
            .map(|(i, item)| match lookup(item, &key) {
                None | Some(Value::None) => Ok(0.0),
                Some(Value::Number(n)) => Ok(*n),
                Some(other) => Err(type_error(format!(
                    "random.weighted: `{key}` on item {i} is {}, expected a number",
                    other.type_name()
                ))),
            })
            .collect::<Result<_, _>>()?,
        _ => {
            return Err(host_error(
                "random.weighted: give either `weights` or `key`, not both or neither".to_string(),
            ));
        }
    };

    let usable = |w: f64| w.is_finite() && w > 0.0;
    let total: f64 = weights.iter().copied().filter(|w| usable(*w)).sum();
    if total <= 0.0 {
        return Ok(Value::None);
    }

    let roll = rand::random_range(0.0..total);
    let mut cumulative = 0.0;
    let mut last_usable = None;
    for (item, weight) in items.into_iter().zip(weights) {
        if !usable(weight) {
            continue;
        }
        cumulative += weight;
        if roll < cumulative {
            return Ok(item);
        }
        last_usable = Some(item);
    }
    // Float rounding can leave `roll` a hair above the final sum.
    Ok(last_usable.unwrap_or(Value::None))
}

/// `items` in random order.
///
/// `@[random.shuffle(items: [1, 2, 3])]` → `[2, 3, 1]`
#[weaver_processor(
    namespace = "random",
    name = "shuffle",
    returns = "array",
    params(items = "Items to shuffle")
)]
pub fn random_shuffle(items: Vec<Value>) -> Result<Value, EvalError> {
    let mut shuffled = items;
    shuffled.shuffle(&mut rand::rng());
    Ok(Value::Array(shuffled))
}

/// `count` distinct random items from `items`, in random order.
///
/// `@[random.sample(items: [1, 2, 3], count: 2)]` → `[3, 1]`
#[weaver_processor(
    namespace = "random",
    name = "sample",
    returns = "array",
    params(items = "Items to pick from", count = "Number of items")
)]
pub fn random_sample(items: Vec<Value>, count: f64) -> Result<Value, EvalError> {
    if count.is_nan() || count < 0.0 {
        return Err(host_error(format!(
            "random.sample: count must be 0 or more, got {count}"
        )));
    }
    let mut sampled = items;
    sampled.shuffle(&mut rand::rng());
    sampled.truncate(count as usize);
    Ok(Value::Array(sampled))
}

/// Upper bound on dice per roll.
const MAX_DICE: u32 = 1000;

/// Total of a dice roll in `NdS±K` notation.
///
/// `@[random.dice(notation: "2d6+3")]` → `11`
#[weaver_processor(
    namespace = "random",
    name = "dice",
    returns = "number",
    params(notation = "Dice notation, e.g. \"2d6+3\" or \"d20\"")
)]
pub fn random_dice(notation: String) -> Result<Value, EvalError> {
    let (count, sides, modifier) = parse_dice(&notation).ok_or_else(|| {
        host_error(format!(
            "random.dice: `{notation}` isn't dice notation like \"2d6+3\" (at most {MAX_DICE} dice)"
        ))
    })?;
    let rolled: i64 = (0..count)
        .map(|_| rand::random_range(1..=sides) as i64)
        .sum();
    Ok(Value::Number((rolled + modifier) as f64))
}

/// `"2d6+3"` → `(2, 6, 3)`. `None` for anything malformed.
fn parse_dice(notation: &str) -> Option<(u32, u32, i64)> {
    let compact: String = notation
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let (dice, modifier) = match compact.find(['+', '-']) {
        Some(at) => (&compact[..at], compact[at..].parse::<i64>().ok()?),
        None => (compact.as_str(), 0),
    };
    let (count, sides) = dice.split_once('d')?;
    let count = if count.is_empty() {
        1
    } else {
        count.parse::<u32>().ok()?
    };
    let sides = sides.parse::<u32>().ok()?;
    ((1..=MAX_DICE).contains(&count) && sides >= 1).then_some((count, sides, modifier))
}

fn register_convert(registry: &mut Registry) {
    registry.register_processor(ConvertToNumberProcessor);
    registry.register_processor(ConvertToStringProcessor);
    registry.register_processor(ConvertToBoolProcessor);
    registry.register_processor(ConvertToJsonProcessor);
    registry.register_processor(ConvertFromJsonProcessor);
    registry.register_processor(ConvertToBase64Processor);
    registry.register_processor(ConvertFromBase64Processor);
}

/// `value` as a number. Accepts numbers, numeric text and booleans.
///
/// `@[convert.to_number(value: "42")]` → `42`
#[weaver_processor(namespace = "convert", name = "to_number", returns = "number")]
fn convert_to_number(value: Value) -> Result<Value, EvalError> {
    let number = match &value {
        Value::Number(n) => Some(*n),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|n| n.is_finite()),
        _ => None,
    };
    number.map(Value::Number).ok_or_else(|| {
        type_error(format!(
            "convert.to_number: can't read a number from {}",
            describe(&value)
        ))
    })
}

/// `value` as text, exactly as a template would render it.
///
/// `@[convert.to_string(value: 42)]` → `"42"`
#[weaver_processor(namespace = "convert", name = "to_string", returns = "string")]
fn convert_to_string(value: Value) -> Result<Value, EvalError> {
    Ok(Value::String(value.to_output_string()))
}

/// `value` as a boolean. Parses `"true"`/`"false"`; otherwise uses truthiness.
///
/// `@[convert.to_bool(value: "false")]` → `false`
#[weaver_processor(namespace = "convert", name = "to_bool", returns = "bool")]
fn convert_to_bool(value: Value) -> Result<Value, EvalError> {
    if let Value::String(s) = &value {
        match s.trim().to_ascii_lowercase().as_str() {
            "true" => return Ok(Value::Bool(true)),
            "false" => return Ok(Value::Bool(false)),
            _ => {}
        }
    }
    Ok(Value::Bool(value.is_truthy()))
}

/// `value` as JSON with sorted keys, indented if `pretty`.
///
/// `@[convert.to_json(value: {name: "Kael", hp: 10})]` → `{"hp":10,"name":"Kael"}`
#[weaver_processor(namespace = "convert", name = "to_json", returns = "string")]
fn convert_to_json(value: Value, pretty: Option<bool>) -> Result<Value, EvalError> {
    let json = weaver_to_json(&value).map_err(|e| type_error(format!("convert.to_json: {e}")))?;
    let text = if pretty.unwrap_or(false) {
        serde_json::to_string_pretty(&json)
    } else {
        serde_json::to_string(&json)
    }
    .map_err(|e| host_error(format!("convert.to_json: {e}")))?;
    Ok(Value::String(text))
}

/// Parses JSON `text`. `null` becomes `none`.
///
/// `@[convert.from_json(text: "[1, 2]")]` → `[1, 2]`
#[weaver_processor(namespace = "convert", name = "from_json", returns = "any")]
fn convert_from_json(text: String) -> Result<Value, EvalError> {
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| host_error(format!("convert.from_json: invalid JSON: {e}")))?;
    json_to_weaver(&json).map_err(|e| type_error(format!("convert.from_json: {e}")))
}

/// Base64-encodes `text`.
///
/// `@[convert.to_base64(text: "hi")]` → `"aGk="`
#[weaver_processor(namespace = "convert", name = "to_base64", returns = "string")]
fn convert_to_base64(text: String) -> Result<Value, EvalError> {
    Ok(Value::String(BASE64.encode(text.as_bytes())))
}

/// Decodes base64 `text`. Errors unless the result is valid UTF-8.
///
/// `@[convert.from_base64(text: "aGk=")]` → `"hi"`
#[weaver_processor(namespace = "convert", name = "from_base64", returns = "string")]
fn convert_from_base64(text: String) -> Result<Value, EvalError> {
    let bytes = BASE64
        .decode(text.trim())
        .map_err(|e| host_error(format!("convert.from_base64: invalid base64: {e}")))?;
    String::from_utf8(bytes)
        .map(Value::String)
        .map_err(|_| host_error("convert.from_base64: the decoded data isn't text".to_string()))
}

/// A value's type, plus the value when short, for error messages.
fn describe(value: &Value) -> String {
    match value {
        Value::String(s) if s.chars().count() <= 40 => format!("the text \"{s}\""),
        Value::String(_) => "a long piece of text".to_string(),
        other => format!("{} ({})", other.type_name(), other.to_output_string()),
    }
}

fn register_value(registry: &mut Registry) {
    registry.register_processor(ValueTypeOfProcessor);
    registry.register_processor(ValueIsNoneProcessor);
}

/// Type name of `value`: `string`, `number`, `bool`, `array`, `object` or `none`.
///
/// `@[value.type_of(value: 42)]` → `"number"`
#[weaver_processor(namespace = "value", name = "type_of", returns = "string")]
fn value_type_of(value: Value) -> Result<Value, EvalError> {
    Ok(Value::String(value.type_name().to_string()))
}

/// Whether `value` is `none`. Unlike `if`, distinguishes `none` from `0`, `""` and `false`.
///
/// `@[value.is_none(value: {{global:missing}})]` → `true`
#[weaver_processor(namespace = "value", name = "is_none", returns = "bool")]
fn value_is_none(value: Value) -> Result<Value, EvalError> {
    Ok(Value::Bool(matches!(value, Value::None)))
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

    fn keys_of(value: Value) -> Result<Value, EvalError> {
        let mut r = Registry::new();
        register_all(&mut r);
        r.call_processor("object", "keys", HashMap::from([("of".to_string(), value)]))
    }

    #[test]
    fn object_keys_come_back_sorted() {
        let obj = Value::object([("b", Value::Number(2.0)), ("a", Value::Number(1.0))]);
        assert_eq!(
            keys_of(obj).unwrap(),
            Value::Array(vec![Value::String("a".into()), Value::String("b".into())])
        );
    }

    #[test]
    fn an_empty_object_has_no_keys() {
        assert_eq!(
            keys_of(Value::Object(Default::default())).unwrap(),
            Value::Array(vec![])
        );
    }

    #[test]
    fn object_keys_rejects_a_non_object() {
        // An array is the near miss worth being loud about.
        assert!(keys_of(Value::Array(vec![Value::Number(1.0)])).is_err());
        assert!(keys_of(Value::None).is_err());
    }

    fn call(processor: &str, props: Vec<(&str, Value)>) -> Result<Value, EvalError> {
        let mut r = Registry::new();
        register_all(&mut r);
        let (ns, name) = processor.split_once('.').unwrap();
        let props: HashMap<String, Value> =
            props.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        r.call_processor(ns, name, props)
    }

    fn ok(processor: &str, props: Vec<(&str, Value)>) -> Value {
        call(processor, props).unwrap_or_else(|e| panic!("{processor} failed: {e:?}"))
    }

    fn n(x: f64) -> Value {
        Value::Number(x)
    }

    fn s(x: &str) -> Value {
        Value::String(x.into())
    }

    fn nums(xs: &[f64]) -> Value {
        Value::Array(xs.iter().copied().map(Value::Number).collect())
    }

    #[test]
    fn gen_includes_both_bounds() {
        let rolls: std::collections::BTreeSet<i64> = (0..500)
            .map(|_| ok("random.gen", vec![("min", n(1.0)), ("max", n(3.0))]))
            .map(|v| v.as_number().unwrap() as i64)
            .collect();
        assert_eq!(rolls, [1, 2, 3].into());
    }

    #[test]
    fn gen_rounds_decimal_bounds_inward() {
        for _ in 0..200 {
            let v = ok("random.gen", vec![("min", n(0.5)), ("max", n(2.5))]);
            assert!(matches!(v.as_number(), Some(x) if x == 1.0 || x == 2.0));
        }
        assert_eq!(
            ok("random.gen", vec![("min", n(4.0)), ("max", n(4.0))]),
            n(4.0)
        );
    }

    #[test]
    fn gen_errors_instead_of_panicking_on_an_empty_range() {
        assert!(call("random.gen", vec![("min", n(5.0)), ("max", n(1.0))]).is_err());
        assert!(call("random.gen", vec![("min", n(0.2)), ("max", n(0.8))]).is_err());
    }

    #[test]
    fn float_stays_in_range() {
        for _ in 0..200 {
            let x = ok("random.float", vec![]).as_number().unwrap();
            assert!((0.0..1.0).contains(&x));
            let y = ok("random.float", vec![("min", n(-2.0)), ("max", n(-1.0))])
                .as_number()
                .unwrap();
            assert!((-2.0..-1.0).contains(&y));
        }
        assert_eq!(
            ok("random.float", vec![("min", n(3.0)), ("max", n(3.0))]),
            n(3.0)
        );
        assert!(call("random.float", vec![("min", n(2.0)), ("max", n(1.0))]).is_err());
    }

    #[test]
    fn chance_honours_the_extremes() {
        for _ in 0..100 {
            assert_eq!(ok("random.chance", vec![("p", n(0.0))]), Value::Bool(false));
            assert_eq!(ok("random.chance", vec![("p", n(1.0))]), Value::Bool(true));
            assert_eq!(ok("random.chance", vec![("p", n(7.0))]), Value::Bool(true));
        }
        assert!(call("random.chance", vec![("p", n(f64::NAN))]).is_err());
    }

    #[test]
    fn pick_from_nothing_is_none() {
        assert_eq!(ok("random.pick", vec![("items", nums(&[]))]), Value::None);
        assert_eq!(ok("random.pick", vec![("items", nums(&[9.0]))]), n(9.0));
    }

    #[test]
    fn weighted_by_array_never_picks_zero_weights() {
        for _ in 0..200 {
            let v = ok(
                "random.weighted",
                vec![
                    ("items", Value::from(vec!["a", "b", "c"])),
                    ("weights", nums(&[0.0, 1.0, -3.0])),
                ],
            );
            assert_eq!(v, s("b"));
        }
    }

    #[test]
    fn weighted_by_key_reaches_every_weighted_item() {
        let items = Value::Array(vec![
            Value::object([("handle", s("a")), ("talk", n(0.5))]),
            Value::object([("handle", s("b")), ("talk", n(0.5))]),
            Value::object([("handle", s("c"))]),
        ]);
        let picked: std::collections::BTreeSet<String> = (0..300)
            .map(|_| {
                ok(
                    "random.weighted",
                    vec![("items", items.clone()), ("key", s("talk"))],
                )
            })
            .map(|v| lookup(&v, "handle").unwrap().to_output_string())
            .collect();
        assert_eq!(picked, ["a".to_string(), "b".to_string()].into());
    }

    #[test]
    fn weighted_edge_cases() {
        let items = Value::from(vec!["a", "b"]);
        // Nobody can be picked.
        assert_eq!(
            ok(
                "random.weighted",
                vec![("items", items.clone()), ("weights", nums(&[0.0, 0.0]))]
            ),
            Value::None
        );
        assert_eq!(
            ok(
                "random.weighted",
                vec![("items", nums(&[])), ("weights", nums(&[]))]
            ),
            Value::None
        );
        // Mismatched lengths, both sources, neither source, a bad weight.
        assert!(
            call(
                "random.weighted",
                vec![("items", items.clone()), ("weights", nums(&[1.0]))]
            )
            .is_err()
        );
        assert!(
            call(
                "random.weighted",
                vec![
                    ("items", items.clone()),
                    ("weights", nums(&[1.0, 1.0])),
                    ("key", s("w"))
                ]
            )
            .is_err()
        );
        assert!(call("random.weighted", vec![("items", items.clone())]).is_err());
        assert!(
            call(
                "random.weighted",
                vec![
                    (
                        "items",
                        Value::Array(vec![Value::object([("w", s("lots"))])])
                    ),
                    ("key", s("w"))
                ]
            )
            .is_err()
        );
    }

    #[test]
    fn shuffle_and_sample_keep_the_same_items() {
        let mut shuffled: Vec<i64> = ok(
            "random.shuffle",
            vec![("items", nums(&[1.0, 2.0, 3.0, 4.0]))],
        )
        .into_array()
        .unwrap()
        .iter()
        .map(|v| v.as_number().unwrap() as i64)
        .collect();
        shuffled.sort();
        assert_eq!(shuffled, vec![1, 2, 3, 4]);

        let sample = ok(
            "random.sample",
            vec![("items", nums(&[1.0, 2.0, 3.0, 4.0])), ("count", n(2.0))],
        )
        .into_array()
        .unwrap();
        assert_eq!(sample.len(), 2);
        assert_ne!(sample[0], sample[1]);

        let all = ok(
            "random.sample",
            vec![("items", nums(&[1.0, 2.0])), ("count", n(10.0))],
        );
        assert_eq!(all.as_array().unwrap().len(), 2);
        assert!(
            call(
                "random.sample",
                vec![("items", nums(&[1.0])), ("count", n(-1.0))]
            )
            .is_err()
        );
    }

    #[test]
    fn dice_parses_common_notation() {
        assert_eq!(parse_dice("2d6+3"), Some((2, 6, 3)));
        assert_eq!(parse_dice("d20"), Some((1, 20, 0)));
        assert_eq!(parse_dice(" 3D8 - 2 "), Some((3, 8, -2)));
        for bad in [
            "", "d", "2d", "0d6", "2d0", "2x6", "2d6+", "2d6+-1", "1001d6", "-2d6",
        ] {
            assert_eq!(parse_dice(bad), None, "`{bad}` should be rejected");
        }
    }

    #[test]
    fn dice_rolls_stay_in_range() {
        for _ in 0..200 {
            let x = ok("random.dice", vec![("notation", s("2d6+3"))])
                .as_number()
                .unwrap();
            assert!((5.0..=15.0).contains(&x));
        }
        assert!(call("random.dice", vec![("notation", s("lots"))]).is_err());
    }

    #[test]
    fn text_split_cases() {
        assert_eq!(
            ok(
                "text.split",
                vec![("text", s("a,b,,c")), ("separator", s(","))]
            ),
            Value::from(vec!["a", "b", "", "c"])
        );
        assert_eq!(
            ok("text.split", vec![("text", s("hé")), ("separator", s(""))]),
            Value::from(vec!["h", "é"])
        );
    }

    #[test]
    fn text_padding() {
        assert_eq!(
            ok(
                "text.pad_start",
                vec![("text", s("7")), ("length", n(3.0)), ("fill", s("0"))]
            ),
            s("007")
        );
        assert_eq!(
            ok(
                "text.pad_end",
                vec![("text", s("ab")), ("length", n(5.0)), ("fill", s("xy"))]
            ),
            s("abxyx")
        );
        assert_eq!(
            ok(
                "text.pad_start",
                vec![("text", s("long")), ("length", n(2.0))]
            ),
            s("long")
        );
        assert!(
            call(
                "text.pad_end",
                vec![("text", s("a")), ("length", n(3.0)), ("fill", s(""))]
            )
            .is_err()
        );
    }

    #[test]
    fn text_index_of_counts_characters() {
        assert_eq!(
            ok(
                "text.index_of",
                vec![("text", s("héllo")), ("substring", s("l"))]
            ),
            n(2.0)
        );
        assert_eq!(
            ok(
                "text.index_of",
                vec![("text", s("abc")), ("substring", s("z"))]
            ),
            n(-1.0)
        );
    }

    #[test]
    fn text_title_keeps_spacing() {
        assert_eq!(
            ok("text.title", vec![("text", s("the  old\ttower"))]),
            s("The  Old\tTower")
        );
    }

    #[test]
    fn math_additions() {
        assert_eq!(
            ok(
                "math.round",
                vec![("value", n(1.23456)), ("digits", n(2.0))]
            ),
            n(1.23)
        );
        assert_eq!(ok("math.round", vec![("value", n(2.5))]), n(3.0));
        assert_eq!(
            ok("math.pow", vec![("base", n(2.0)), ("exponent", n(10.0))]),
            n(1024.0)
        );
        assert!(call("math.pow", vec![("base", n(-8.0)), ("exponent", n(0.5))]).is_err());
        assert_eq!(ok("math.sqrt", vec![("value", n(9.0))]), n(3.0));
        assert!(call("math.sqrt", vec![("value", n(-1.0))]).is_err());
        assert_eq!(ok("math.sign", vec![("value", n(-4.0))]), n(-1.0));
        assert_eq!(ok("math.sign", vec![("value", n(0.0))]), n(0.0));
        assert_eq!(ok("math.sign", vec![("value", n(0.1))]), n(1.0));
    }

    #[test]
    fn array_pluck_follows_paths() {
        let items = Value::Array(vec![
            Value::object([("stats", Value::object([("hp", 3i64)]))]),
            Value::object([("name", "x")]),
            n(1.0),
        ]);
        assert_eq!(
            ok(
                "array.pluck",
                vec![("items", items), ("key", s("stats.hp"))]
            ),
            Value::Array(vec![n(3.0), Value::None, Value::None])
        );
    }

    #[test]
    fn array_aggregates() {
        assert_eq!(
            ok("array.sum", vec![("items", nums(&[1.0, 2.0, 3.5]))]),
            n(6.5)
        );
        assert_eq!(ok("array.sum", vec![("items", nums(&[]))]), n(0.0));
        assert_eq!(
            ok("array.min", vec![("items", nums(&[3.0, 1.0, 2.0]))]),
            n(1.0)
        );
        assert_eq!(
            ok("array.max", vec![("items", nums(&[3.0, 1.0, 2.0]))]),
            n(3.0)
        );
        assert_eq!(ok("array.max", vec![("items", nums(&[]))]), Value::None);
        assert!(call("array.sum", vec![("items", Value::from(vec!["1"]))]).is_err());
    }

    #[test]
    fn array_sort_cases() {
        assert_eq!(
            ok("array.sort", vec![("items", nums(&[3.0, 1.0, 2.0]))]),
            nums(&[1.0, 2.0, 3.0])
        );
        assert_eq!(
            ok(
                "array.sort",
                vec![
                    ("items", Value::from(vec!["b", "c", "a"])),
                    ("descending", Value::Bool(true))
                ]
            ),
            Value::from(vec!["c", "b", "a"])
        );
        assert_eq!(ok("array.sort", vec![("items", nums(&[]))]), nums(&[]));
        assert!(
            call(
                "array.sort",
                vec![("items", Value::Array(vec![n(1.0), s("a")]))]
            )
            .is_err()
        );
    }

    #[test]
    fn array_unique_index_of_and_join() {
        assert_eq!(
            ok(
                "array.unique",
                vec![("items", nums(&[1.0, 2.0, 1.0, 3.0, 2.0]))]
            ),
            nums(&[1.0, 2.0, 3.0])
        );
        assert_eq!(
            ok(
                "array.index_of",
                vec![("items", Value::from(vec!["a", "b"])), ("value", s("b"))]
            ),
            n(1.0)
        );
        assert_eq!(
            ok(
                "array.index_of",
                vec![("items", Value::from(vec!["a"])), ("value", s("z"))]
            ),
            n(-1.0)
        );
        assert_eq!(
            ok("array.join", vec![("items", Value::from(vec!["a", "b"]))]),
            s("a, b")
        );
    }

    #[test]
    fn object_additions() {
        let obj = Value::object([
            ("hp", n(10.0)),
            ("gone", Value::None),
            ("stats", Value::object([("str", 4i64)])),
        ]);
        assert_eq!(
            ok(
                "object.values",
                vec![("of", Value::object([("b", 2i64), ("a", 1i64)]))]
            ),
            nums(&[1.0, 2.0])
        );
        assert_eq!(
            ok(
                "object.has",
                vec![("of", obj.clone()), ("key", s("stats.str"))]
            ),
            Value::Bool(true)
        );
        assert_eq!(
            ok("object.has", vec![("of", obj.clone()), ("key", s("gone"))]),
            Value::Bool(true)
        );
        assert_eq!(
            ok("object.has", vec![("of", obj.clone()), ("key", s("mp"))]),
            Value::Bool(false)
        );
        assert_eq!(
            ok("object.get", vec![("of", obj.clone()), ("key", s("hp"))]),
            n(10.0)
        );
        assert_eq!(
            ok(
                "object.get",
                vec![("of", obj.clone()), ("key", s("gone")), ("default", n(0.0))]
            ),
            n(0.0)
        );
        assert_eq!(
            ok("object.get", vec![("of", obj.clone()), ("key", s("mp"))]),
            Value::None
        );
        assert!(call("object.get", vec![("of", nums(&[])), ("key", s("a"))]).is_err());
        assert_eq!(
            ok(
                "object.merge",
                vec![
                    ("a", Value::object([("hp", 10i64), ("mp", 5i64)])),
                    ("b", Value::object([("hp", 3i64)])),
                ]
            ),
            Value::object([("hp", 3i64), ("mp", 5i64)])
        );
    }

    #[test]
    fn convert_to_number_cases() {
        assert_eq!(ok("convert.to_number", vec![("value", s(" 4.5 "))]), n(4.5));
        assert_eq!(
            ok("convert.to_number", vec![("value", Value::Bool(true))]),
            n(1.0)
        );
        assert_eq!(ok("convert.to_number", vec![("value", n(2.0))]), n(2.0));
        for bad in [s("abc"), s(""), s("inf"), Value::None, nums(&[1.0])] {
            assert!(
                call("convert.to_number", vec![("value", bad.clone())]).is_err(),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn convert_to_string_and_bool() {
        assert_eq!(ok("convert.to_string", vec![("value", n(42.0))]), s("42"));
        assert_eq!(ok("convert.to_string", vec![("value", Value::None)]), s(""));
        assert_eq!(
            ok("convert.to_bool", vec![("value", s("FALSE"))]),
            Value::Bool(false)
        );
        assert_eq!(
            ok("convert.to_bool", vec![("value", s("true"))]),
            Value::Bool(true)
        );
        assert_eq!(
            ok("convert.to_bool", vec![("value", s("no"))]),
            Value::Bool(true)
        );
        assert_eq!(
            ok("convert.to_bool", vec![("value", n(0.0))]),
            Value::Bool(false)
        );
    }

    #[test]
    fn convert_json_round_trips() {
        let obj = Value::object([("name", s("Kael")), ("tags", Value::from(vec!["a"]))]);
        let text = ok("convert.to_json", vec![("value", obj.clone())]);
        assert_eq!(text, s(r#"{"name":"Kael","tags":["a"]}"#));
        assert_eq!(ok("convert.from_json", vec![("text", text)]), obj);

        let pretty = ok(
            "convert.to_json",
            vec![("value", obj), ("pretty", Value::Bool(true))],
        );
        assert!(pretty.as_string().unwrap().contains('\n'));

        assert_eq!(
            ok("convert.from_json", vec![("text", s("null"))]),
            Value::None
        );
        assert!(call("convert.from_json", vec![("text", s("{nope"))]).is_err());
        assert!(call("convert.to_json", vec![("value", n(f64::INFINITY))]).is_err());
    }

    #[test]
    fn convert_base64_round_trips() {
        assert_eq!(ok("convert.to_base64", vec![("text", s("hi"))]), s("aGk="));
        assert_eq!(
            ok("convert.from_base64", vec![("text", s("aGk="))]),
            s("hi")
        );
        assert!(call("convert.from_base64", vec![("text", s("!!"))]).is_err());
        // Valid base64, but not UTF-8.
        assert!(call("convert.from_base64", vec![("text", s("/w=="))]).is_err());
    }

    #[test]
    fn value_type_checks() {
        assert_eq!(ok("value.type_of", vec![("value", n(1.0))]), s("number"));
        assert_eq!(ok("value.type_of", vec![]), s("none"));
        assert_eq!(
            ok("value.is_none", vec![("value", Value::None)]),
            Value::Bool(true)
        );
        assert_eq!(
            ok("value.is_none", vec![("value", n(0.0))]),
            Value::Bool(false)
        );
    }
}

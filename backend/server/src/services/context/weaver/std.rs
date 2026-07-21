use weaver_lang::Value;
use weaver_macros::weaver_processor;

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

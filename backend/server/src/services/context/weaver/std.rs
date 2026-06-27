use weaver_lang::Value;
use weaver_macros::weaver_processor;

#[weaver_processor(namespace = "random", name = "gen")]
pub fn generate_random(min: f64, max: f64) -> Result<Value, EvalError> {
    let min = min as i32;
    let max = max as i32;
    tracing::debug!("Generating random between {min} and {max}");
    Ok(Value::Number(rand::random_range(min..max) as f64))
}

#[weaver_processor(namespace = "random", name = "pick")]
pub fn pick_random(items: Vec<Value>) -> Result<Value, EvalError> {
    let min = 0;
    let max = items.len();

    let idx = rand::random_range(min..max);
    let item = items[idx].clone();
    tracing::debug!("Picked random item: {item}");

    Ok(item)
}

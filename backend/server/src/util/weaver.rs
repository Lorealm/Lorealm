use serde_json::json;

pub fn json_to_weaver(v: &serde_json::Value) -> weaver_lang::Value {
    match v {
        serde_json::Value::String(s) => weaver_lang::Value::String(s.clone()),
        serde_json::Value::Number(n) => weaver_lang::Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => weaver_lang::Value::Bool(*b),
        serde_json::Value::Array(a) => {
            weaver_lang::Value::Array(a.iter().map(json_to_weaver).collect())
        }
        serde_json::Value::Null | serde_json::Value::Object(_) => weaver_lang::Value::None,
    }
}

pub fn weaver_to_json(v: &weaver_lang::Value) -> serde_json::Value {
    match v {
        weaver_lang::Value::String(s) => json!(s),
        weaver_lang::Value::Number(n) => json!(n),
        weaver_lang::Value::Bool(b) => json!(b),
        weaver_lang::Value::Array(a) => {
            serde_json::Value::Array(a.iter().map(weaver_to_json).collect())
        }
        weaver_lang::Value::None => serde_json::Value::Null,
    }
}

use std::collections::BTreeMap;

use thiserror::Error;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn round_trip(v: serde_json::Value) -> serde_json::Value {
        weaver_to_json(&json_to_weaver(&v).unwrap()).unwrap()
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

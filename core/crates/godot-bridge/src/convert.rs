//! Conversions between Godot `Variant`s and `serde_json::Value`.
//!
//! Orders come from GDScript as a `Dictionary`; they are turned into JSON and
//! deserialised into `sim_campaign::Order` so that the bridge never has to
//! know the order fields.
//!
//! These functions call into the Godot runtime, so they cannot be unit-tested
//! with `cargo test`; `core/checks/campaign_sim_check.gd` covers them headless.

use godot::builtin::VariantType;
use godot::prelude::*;
use serde_json::{Map, Number, Value};

/// Converts a GDScript value into JSON. Unsupported types (objects, vectors,
/// colours...) give an error naming the type so that a malformed order is
/// reported instead of silently dropped.
pub fn variant_to_json(variant: &Variant) -> Result<Value, String> {
    match variant.get_type() {
        VariantType::NIL => Ok(Value::Null),
        VariantType::BOOL => Ok(Value::Bool(variant.to::<bool>())),
        VariantType::INT => Ok(Value::from(variant.to::<i64>())),
        VariantType::FLOAT => Number::from_f64(variant.to::<f64>())
            .map(Value::Number)
            .ok_or_else(|| "non-finite float".to_string()),
        VariantType::STRING => Ok(Value::String(variant.to::<GString>().to_string())),
        VariantType::STRING_NAME => Ok(Value::String(variant.to::<StringName>().to_string())),
        VariantType::ARRAY => variant
            .to::<VarArray>()
            .iter_shared()
            .map(|item| variant_to_json(&item))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        VariantType::PACKED_STRING_ARRAY => Ok(Value::Array(
            variant
                .to::<PackedStringArray>()
                .as_slice()
                .iter()
                .map(|item| Value::String(item.to_string()))
                .collect(),
        )),
        VariantType::PACKED_INT32_ARRAY => Ok(Value::Array(
            variant
                .to::<PackedInt32Array>()
                .as_slice()
                .iter()
                .map(|item| Value::from(*item))
                .collect(),
        )),
        VariantType::PACKED_INT64_ARRAY => Ok(Value::Array(
            variant
                .to::<PackedInt64Array>()
                .as_slice()
                .iter()
                .map(|item| Value::from(*item))
                .collect(),
        )),
        VariantType::DICTIONARY => {
            let mut map = Map::new();
            for (key, value) in variant.to::<VarDictionary>().iter_shared() {
                let key = match key.get_type() {
                    VariantType::STRING => key.to::<GString>().to_string(),
                    VariantType::STRING_NAME => key.to::<StringName>().to_string(),
                    other => {
                        return Err(format!("dictionary key of type {other:?} is not a string"))
                    }
                };
                map.insert(key, variant_to_json(&value)?);
            }
            Ok(Value::Object(map))
        }
        other => Err(format!("unsupported Godot type {other:?}")),
    }
}

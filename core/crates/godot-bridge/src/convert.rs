//! Conversions between Godot `Variant`s and `serde_json::Value`.
//!
//! Orders come from GDScript as a `Dictionary`; they are turned into JSON and
//! deserialised into `sim_campaign::Order` so that the bridge never has to
//! know the order fields.
//!
//! These functions call into the Godot runtime, so they cannot be unit-tested
//! with `cargo test`; `core/checks/campaign_sim_check.gd` covers them headless.

use godot::builtin::{AnyArray, AnyDictionary, VariantType};
use godot::prelude::*;
use serde_json::{Map, Number, Value};

/// Converts a GDScript value into JSON. Unsupported types (objects, vectors,
/// colours...) give an error naming the type so that a malformed order is
/// reported instead of silently dropped.
///
/// Typed containers (`Array[int]`, typed dictionaries) are accepted: reading
/// them as `VarArray` panicked, which broke every battle order whose unit list
/// came from a typed GDScript array (general retreat, halt, fire at will...).
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
            .to::<AnyArray>()
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
            for (key, value) in variant.to::<AnyDictionary>().iter_shared() {
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

/// Converts JSON into plain Godot values (integers stay integers).
pub fn json_to_variant(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_variant()
            } else if let Some(u) = n.as_u64() {
                (u as i64).to_variant()
            } else {
                n.as_f64().unwrap_or(0.0).to_variant()
            }
        }
        Value::String(s) => GString::from(s.as_str()).to_variant(),
        Value::Array(items) => items
            .iter()
            .map(json_to_variant)
            .collect::<VarArray>()
            .to_variant(),
        Value::Object(map) => {
            let mut dict = VarDictionary::new();
            for (key, item) in map {
                dict.set(key.as_str(), &json_to_variant(item));
            }
            dict.to_variant()
        }
    }
}

pub fn to_dict<T: serde::Serialize>(value: &T) -> VarDictionary {
    serde_json::to_value(value)
        .ok()
        .map(|json| json_to_variant(&json))
        .and_then(|variant| variant.try_to::<VarDictionary>().ok())
        .unwrap_or_default()
}

/// Rows `{text, value}` of an explained score (treaty and diplomacy panels).
pub fn reasons_array(reasons: &[(String, i32)]) -> VarArray {
    reasons
        .iter()
        .map(|(text, value)| {
            vdict! { "text" => text.as_str(), "value" => i64::from(*value) }.to_variant()
        })
        .collect()
}

/// Standard reply `{ok, error}` of an order: the Rust result as a dictionary
/// (`error` empty on success).
pub fn resolve_reply(result: Result<(), String>) -> VarDictionary {
    match result {
        Ok(()) => vdict! { "ok" => true, "error" => "" },
        Err(error) => vdict! { "ok" => false, "error" => error.as_str() },
    }
}

pub fn from_dict<T: serde::de::DeserializeOwned>(dict: &VarDictionary) -> Result<T, String> {
    variant_to_json(&dict.to_variant())
        .and_then(|json| serde_json::from_value::<T>(json).map_err(|e| e.to_string()))
}

/// Godot vector from a pair of coordinates.
pub fn vec2_f32(point: [f32; 2]) -> Vector2 {
    Vector2::new(point[0], point[1])
}

/// Godot vector from a pair of `f64` coordinates.
pub fn vec2_f64(point: [f64; 2]) -> Vector2 {
    Vector2::new(point[0] as f32, point[1] as f32)
}

/// Tuple of a Godot vector.
pub fn vec2_pair(v: Vector2) -> (f32, f32) {
    (v.x, v.y)
}

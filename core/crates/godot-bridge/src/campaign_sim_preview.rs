//! IB5 (ADR 0109): tooltip previews read by `RichTooltip` in the `live`
//! dictionaries — `before_after` `{effect key: [before, after]}` and
//! `requirements` `[{id, met}]` (see `sim_campaign::preview`).

use godot::prelude::*;
use sim_campaign::preview::{BeforeAfter, Requirement};

/// `{effect key: [before, after]}` (an `Array`, as `RichTooltip.effect_item` reads it).
pub(crate) fn before_after_dict(map: &BeforeAfter) -> VarDictionary {
    let mut dict = VarDictionary::new();
    for (key, [before, after]) in map {
        let mut pair = VarArray::new();
        pair.push(&before.to_variant());
        pair.push(&after.to_variant());
        dict.set(key.as_str(), &pair);
    }
    dict
}

/// `[{id, met}]`.
pub(crate) fn requirements_array(rows: &[Requirement]) -> VarArray {
    rows.iter()
        .map(|row| {
            vdict! {
                "id" => row.id.as_str(),
                "met" => row.met,
            }
            .to_variant()
        })
        .collect()
}

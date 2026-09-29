//! `CampaignSim` grouped province reads for the map refresh (lot PB3d).
//!
//! `refresh_all()` used to call `get_province_state` — a full dictionary
//! with garrison and governor — once per province in several loops. The map
//! layers only read owner, controller, devastation, population and siege:
//! one call now returns them for every province as packed arrays.

use data_model::ProvinceId;
use godot::prelude::*;
use sim_campaign::CampaignState;

use crate::campaign_sim::{ids, CampaignSim};

/// One row of `get_provinces_snapshot`, in plain Rust types — kept apart
/// from the `#[func]` method so the siege fields (lot RS-L) can be unit
/// tested without a live Godot engine (`CampaignSim` needs one to build a
/// `Base<RefCounted>`).
pub(crate) struct ProvinceSnapshotRow<'a> {
    pub owner: &'a str,
    pub controller: &'a str,
    pub devastation: i32,
    pub population_total: i64,
    pub besieged: bool,
    pub constructing: bool,
    /// Empty when `besieged` is false.
    pub siege_attacker: &'a str,
    pub siege_supplies: i32,
    pub siege_turns_elapsed: i32,
}

/// The snapshot row of province `id` in `state`, or `None` for an unknown
/// province (no city or province state, e.g. a malformed id).
pub(crate) fn province_snapshot_row<'a>(
    state: &'a CampaignState,
    id: &ProvinceId,
) -> Option<ProvinceSnapshotRow<'a>> {
    let province = state.province_state(id)?;
    let city = state.city_state(id)?;
    let siege = city.siege.as_ref();
    Some(ProvinceSnapshotRow {
        owner: city.owner.as_str(),
        controller: city.controller.as_str(),
        devastation: i32::from(province.devastation),
        population_total: province.population.total() as i64,
        besieged: siege.is_some(),
        constructing: city.construction.is_some(),
        siege_attacker: siege.map_or("", |s| s.attacker.as_str()),
        siege_supplies: siege.map_or(0, |s| i32::from(s.supplies)),
        siege_turns_elapsed: siege
            .map_or(0, |s| i32::try_from(s.turns_elapsed).unwrap_or(i32::MAX)),
    })
}

#[godot_api(secondary)]
impl CampaignSim {
    /// For `ids` (in that order): `{known: PackedByteArray, owner,
    /// controller: PackedStringArray, devastation: PackedInt32Array,
    /// population_total: PackedInt64Array, besieged, constructing:
    /// PackedByteArray, siege_attacker: PackedStringArray, siege_supplies,
    /// siege_turns_elapsed: PackedInt32Array}`, the fields of
    /// `get_province_state` of the same name; `constructing` = the city has
    /// a construction under way (as `get_province_city().construction`).
    /// The `siege_*` fields (lot RS-L) are the same-named fields of
    /// `get_province_state`'s `siege` dict, read straight from
    /// `get_provinces_snapshot` so callers such as `alerts.gd` no longer
    /// need a per-province `get_province_state` for the siege detail; empty
    /// string / 0 where `besieged` is 0. An unknown id (or no campaign)
    /// gives `known = 0`, empty strings and zeros.
    #[func]
    fn get_provinces_snapshot(&self, ids: PackedStringArray) -> VarDictionary {
        let count = ids.len();
        let mut known = PackedByteArray::new();
        let mut owner = PackedStringArray::new();
        let mut controller = PackedStringArray::new();
        let mut devastation = PackedInt32Array::new();
        let mut population = PackedInt64Array::new();
        let mut besieged = PackedByteArray::new();
        let mut constructing = PackedByteArray::new();
        let mut siege_attacker = PackedStringArray::new();
        let mut siege_supplies = PackedInt32Array::new();
        let mut siege_turns_elapsed = PackedInt32Array::new();
        known.resize(count);
        owner.resize(count);
        controller.resize(count);
        devastation.resize(count);
        population.resize(count);
        besieged.resize(count);
        constructing.resize(count);
        siege_attacker.resize(count);
        siege_supplies.resize(count);
        siege_turns_elapsed.resize(count);
        if let Some(state) = &self.state {
            for (i, id) in ids.as_slice().iter().enumerate() {
                let Some(row) = ProvinceId::new(id.to_string())
                    .ok()
                    .and_then(|id| province_snapshot_row(state, &id))
                else {
                    continue;
                };
                known[i] = 1;
                owner[i] = GString::from(row.owner);
                controller[i] = GString::from(row.controller);
                devastation[i] = row.devastation;
                population[i] = row.population_total;
                besieged[i] = u8::from(row.besieged);
                constructing[i] = u8::from(row.constructing);
                siege_attacker[i] = GString::from(row.siege_attacker);
                siege_supplies[i] = row.siege_supplies;
                siege_turns_elapsed[i] = row.siege_turns_elapsed;
            }
        }
        vdict! {
            "known" => &known,
            "owner" => &owner,
            "controller" => &controller,
            "devastation" => &devastation,
            "population_total" => &population,
            "besieged" => &besieged,
            "constructing" => &constructing,
            "siege_attacker" => &siege_attacker,
            "siege_supplies" => &siege_supplies,
            "siege_turns_elapsed" => &siege_turns_elapsed,
        }
    }

    /// Live settlements for the map refresh, in the order of `settlements()`:
    /// aligned arrays `{id, controller, owner, name: PackedStringArray,
    /// fortification_level: PackedInt32Array, is_city: PackedByteArray,
    /// buildings: Array[PackedStringArray]}` (the fields of
    /// `settlement_detail` of the same name, without its per-settlement
    /// income and panels). Empty arrays before a campaign.
    #[func]
    fn get_settlements_live(&self) -> VarDictionary {
        let mut id = PackedStringArray::new();
        let mut controller = PackedStringArray::new();
        let mut owner = PackedStringArray::new();
        let mut name = PackedStringArray::new();
        let mut fortification = PackedInt32Array::new();
        let mut is_city = PackedByteArray::new();
        let mut buildings = VarArray::new();
        if let (Some(state), Some(data)) = (&self.state, &self.data) {
            for (settlement_id, settlement) in &data.settlements {
                let Some(live) = state.settlement_state(settlement_id) else {
                    continue;
                };
                id.push(settlement_id.as_str());
                controller.push(live.controller.as_str());
                owner.push(live.owner.as_str());
                name.push(settlement.name.display.as_str());
                fortification.push(
                    i32::try_from(state.fortification_level(data, settlement_id))
                        .unwrap_or(i32::MAX),
                );
                is_city.push(u8::from(
                    state.province_city_id(&live.province) == Some(settlement_id),
                ));
                buildings.push(&ids(live.buildings.iter()));
            }
        }
        vdict! {
            "id" => &id,
            "controller" => &controller,
            "owner" => &owner,
            "name" => &name,
            "fortification_level" => &fortification,
            "is_city" => &is_city,
            "buildings" => &buildings,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use data_model::{FactionId, GameData};
    use sim_campaign::SiegeState;

    use super::*;

    /// Real game data, or `None` to skip the test when `data/` is not
    /// checked out next to the crate (mirrors `turn_job`'s tests).
    fn load_data() -> Option<Arc<GameData>> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        if !dir.join("factions").exists() {
            return None;
        }
        Some(Arc::new(GameData::load(&dir).expect("game data loads").0))
    }

    /// RS-L: `province_snapshot_row` reads the siege detail the same way
    /// `get_province_state`'s `siege` dict does — `alerts.gd` reads these
    /// fields off the grouped snapshot instead of a per-province call.
    #[test]
    fn siege_fields_read_the_ongoing_siege() {
        let Some(data) = load_data() else {
            return;
        };
        let player = FactionId::new("fac_france").expect("well-formed id");
        let mut state =
            CampaignState::new_1337(&data, player.clone(), 1337).expect("campaign builds");
        let attacker = FactionId::new("fac_angleterre").expect("well-formed id");
        let province_id = state
            .provinces
            .keys()
            .next()
            .cloned()
            .expect("at least one province");

        // No siege yet: besieged is false, siege fields are empty/zero.
        let row = province_snapshot_row(&state, &province_id).expect("known province");
        assert!(!row.besieged);
        assert_eq!(row.siege_attacker, "");
        assert_eq!(row.siege_supplies, 0);
        assert_eq!(row.siege_turns_elapsed, 0);

        let city = state
            .city_state_mut(&province_id)
            .expect("province has a city");
        city.siege = Some(SiegeState {
            attacker: attacker.clone(),
            turns_left: 3,
            turns_elapsed: 2,
            supplies: 40,
            breach: 0,
            started_turn: 0,
            engine_work: 0,
        });

        let row = province_snapshot_row(&state, &province_id).expect("known province");
        assert!(row.besieged);
        assert_eq!(row.siege_attacker, attacker.as_str());
        assert_eq!(row.siege_supplies, 40);
        assert_eq!(row.siege_turns_elapsed, 2);
    }

    /// An id with no province/city state gives `None` (the `#[func]`
    /// method leaves `known` at 0 for it).
    #[test]
    fn unknown_province_gives_no_row() {
        let Some(data) = load_data() else {
            return;
        };
        let player = FactionId::new("fac_france").expect("well-formed id");
        let state = CampaignState::new_1337(&data, player, 1337).expect("campaign builds");
        let bogus = ProvinceId::new("prov_does_not_exist".to_owned()).expect("well-formed id");
        assert!(province_snapshot_row(&state, &bogus).is_none());
    }
}

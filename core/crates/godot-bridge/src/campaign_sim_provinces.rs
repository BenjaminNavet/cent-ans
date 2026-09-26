//! `CampaignSim` grouped province reads for the map refresh (lot PB3d).
//!
//! `refresh_all()` used to call `get_province_state` — a full dictionary
//! with garrison and governor — once per province in several loops. The map
//! layers only read owner, controller, devastation, population and siege:
//! one call now returns them for every province as packed arrays.

use data_model::ProvinceId;
use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// For `ids` (in that order): `{known: PackedByteArray, owner,
    /// controller: PackedStringArray, devastation: PackedInt32Array,
    /// population_total: PackedInt64Array, besieged: PackedByteArray}`,
    /// the fields of `get_province_state` of the same name. An unknown id
    /// (or no campaign) gives `known = 0`, empty strings and zeros.
    #[func]
    fn get_provinces_snapshot(&self, ids: PackedStringArray) -> VarDictionary {
        let count = ids.len();
        let mut known = PackedByteArray::new();
        let mut owner = PackedStringArray::new();
        let mut controller = PackedStringArray::new();
        let mut devastation = PackedInt32Array::new();
        let mut population = PackedInt64Array::new();
        let mut besieged = PackedByteArray::new();
        known.resize(count);
        owner.resize(count);
        controller.resize(count);
        devastation.resize(count);
        population.resize(count);
        besieged.resize(count);
        if let Some(state) = &self.state {
            for (i, id) in ids.as_slice().iter().enumerate() {
                let Some((province, city)) = ProvinceId::new(id.to_string())
                    .ok()
                    .and_then(|id| Some((state.province_state(&id)?, state.city_state(&id)?)))
                else {
                    continue;
                };
                known[i] = 1;
                owner[i] = GString::from(city.owner.as_str());
                controller[i] = GString::from(city.controller.as_str());
                devastation[i] = i32::from(province.devastation);
                population[i] = province.population.total() as i64;
                besieged[i] = u8::from(city.siege.is_some());
            }
        }
        vdict! {
            "known" => &known,
            "owner" => &owner,
            "controller" => &controller,
            "devastation" => &devastation,
            "population_total" => &population,
            "besieged" => &besieged,
        }
    }

    /// Live settlements for the map refresh, in the order of `settlements()`:
    /// `{id, controller, owner, name: PackedStringArray}` (four aligned
    /// arrays, no per-settlement dictionary). Empty arrays before a campaign.
    #[func]
    fn get_settlements_live(&self) -> VarDictionary {
        let mut id = PackedStringArray::new();
        let mut controller = PackedStringArray::new();
        let mut owner = PackedStringArray::new();
        let mut name = PackedStringArray::new();
        if let (Some(state), Some(data)) = (&self.state, &self.data) {
            for (settlement_id, settlement) in &data.settlements {
                let Some(live) = state.settlement_state(settlement_id) else {
                    continue;
                };
                id.push(settlement_id.as_str());
                controller.push(live.controller.as_str());
                owner.push(live.owner.as_str());
                name.push(settlement.name.display.as_str());
            }
        }
        vdict! {
            "id" => &id,
            "controller" => &controller,
            "owner" => &owner,
            "name" => &name,
        }
    }
}

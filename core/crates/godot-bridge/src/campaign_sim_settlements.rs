//! `CampaignSim` settlements API (lot C1, `docs/design/2026-09-24-echelle-colonies.md` § 4.6).

use data_model::{ProvinceId, SettlementId};
use godot::prelude::*;

use crate::campaign_sim::{
    buildable_array, buildings_array, construction_dict, demolition_preview_array, ids,
    units_array, CampaignSim,
};

#[godot_api(secondary)]
impl CampaignSim {
    /// Every settlement, sorted by id: `[{id, province, kind, name, lonlat,
    /// controller, owner, fortification_level}]`. `kind` is `city`, `town`,
    /// `castle`, `abbey` or `village`; `name` is the French display name;
    /// `lonlat` is a `Vector2(lon, lat)` in WGS84 degrees. Empty before a
    /// campaign exists.
    #[func]
    fn settlements(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        data.settlements
            .iter()
            .filter_map(|(id, settlement)| {
                let live = state.settlement_state(id)?;
                let lonlat = Vector2::new(settlement.lonlat[0] as f32, settlement.lonlat[1] as f32);
                Some(
                    vdict! {
                        "id" => id.as_str(),
                        "province" => settlement.province.as_str(),
                        "kind" => settlement.kind.key(),
                        "name" => settlement.name.display.as_str(),
                        "lonlat" => lonlat,
                        "controller" => live.controller.as_str(),
                        "owner" => live.owner.as_str(),
                        "fortification_level" => i64::from(live.fortification_level),
                    }
                    .to_variant(),
                )
            })
            .collect()
    }

    /// Detail of one settlement (lot C4): `{id, province, kind, name, lonlat,
    /// owner, controller, fortification_level, weight_share, garrison[],
    /// buildings[], recruit_queue[] (unit ids; B7b: `recruit_queue_turns[]`
    /// gives each one's turns of training left), construction?, siege?, income,
    /// is_city}`. Lot C5 adds `buildings_info[]` (`{id, name, category,
    /// upkeep, …}` as in `get_province_city`), `recruit_slots`,
    /// `recruit_slots_free`, `garrison_strength` (sum of unit strengths),
    /// `port` and `province_name`. Lot C7d adds `garrison_cap` (max units a
    /// `garrison_units` order may hold here, -1 if `data/settlements/rules.json`
    /// sets none for this kind) and `garrison_free` (`garrison_cap` minus the
    /// current garrison size, -1 if uncapped). `income` is the settlement's tax
    /// share for its controller this season (0 while besieged). Empty for an
    /// unknown id.
    #[func]
    fn settlement_detail(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(id) = SettlementId::new(id.to_string()) else {
            return VarDictionary::new();
        };
        let (Some(entry), Some(live)) = (data.settlements.get(&id), state.settlement_state(&id))
        else {
            return VarDictionary::new();
        };
        let tax_rate = state
            .factions
            .get(&live.controller)
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        let tech =
            sim_campaign::research::faction_province_tech_effects(state, data, &live.controller);
        let income = if live.siege.is_some() {
            0
        } else {
            state.settlement_tax(data, &id, tax_rate, &tech).round() as i64
        };
        let lonlat = Vector2::new(entry.lonlat[0] as f32, entry.lonlat[1] as f32);
        let is_city = state.province_city_id(&live.province) == Some(&id);
        let garrison_cap: Option<usize> = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.garrison_cap.get(&live.kind))
            .copied();
        let garrison_free = garrison_cap
            .map(|cap| cap.saturating_sub(live.garrison.len()) as i64)
            .unwrap_or(-1);
        let mut dict = vdict! {
            "id" => id.as_str(),
            "province" => live.province.as_str(),
            "kind" => live.kind.key(),
            "name" => entry.name.display.as_str(),
            "lonlat" => lonlat,
            "owner" => live.owner.as_str(),
            "controller" => live.controller.as_str(),
            "fortification_level" => i64::from(state.fortification_level(data, &id)),
            "weight_share" => sim_campaign::settlements::weight_share(data, &id),
            "garrison" => &units_array(data, &live.garrison),
            "buildings" => &ids(live.buildings.iter()),
            "recruit_queue" => &ids(live.recruit_queue.iter().map(|r| &r.unit_type)),
            "recruit_queue_turns" => &live
                .recruit_queue
                .iter()
                .map(|r| r.turns_left as i32)
                .collect::<PackedInt32Array>(),
            "income" => income,
            "is_city" => is_city,
            "buildings_info" => &buildings_array(data, &live.buildings),
            "recruit_slots" => state.recruit_slots(data, &id) as i64,
            "recruit_slots_free" => state.recruit_slots_free(data, &id) as i64,
            "garrison_strength" => live.garrison.iter().map(|u| i64::from(u.strength)).sum::<i64>(),
            "port" => entry.port,
            "province_name" => data
                .provinces
                .get(&live.province)
                .map_or(live.province.as_str(), |p| p.name.display.as_str()),
            "garrison_cap" => garrison_cap.map_or(-1, |cap| cap as i64),
            "garrison_free" => garrison_free,
        };
        if let Some(construction) = &live.construction {
            dict.set("construction", &construction_dict(data, construction));
        }
        if let Some(siege) = &live.siege {
            let siege_dict = vdict! {
                "attacker" => siege.attacker.as_str(),
                "turns_left" => i64::from(siege.turns_left),
                "turns_elapsed" => i64::from(siege.turns_elapsed),
                "supplies" => i64::from(siege.supplies),
                "breach" => i64::from(siege.breach),
            };
            dict.set("siege", &siege_dict);
        }
        dict
    }

    /// Build options of a settlement for its controller (lot C5):
    /// `[{building, name, category, cost, turns, available, reason}]`, only
    /// the buildings allowed in its kind (`Building::allowed_in`). Empty for
    /// an unknown id. Recruitment options: `get_recruitable(settlement_id)`.
    #[func]
    fn settlement_buildable(&self, id: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(id) = SettlementId::new(id.to_string()) else {
            return VarArray::new();
        };
        buildable_array(data, &state.buildable(data, &id), Some((state, &id)))
    }

    /// RS-N: demolition preview of every built building of a settlement, for
    /// the « Raser » button (`Order::Demolish`): `[{building, name,
    /// can_demolish, reason, refund, upkeep_saved}]`, in the order of
    /// `settlement_detail`'s `buildings`. `reason` is French, empty when
    /// `can_demolish` is true. Empty for an unknown id.
    #[func]
    fn settlement_demolition_preview(&self, id: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(id) = SettlementId::new(id.to_string()) else {
            return VarArray::new();
        };
        let Some(live) = state.settlement_state(&id) else {
            return VarArray::new();
        };
        demolition_preview_array(state, data, &id, &live.buildings)
    }

    /// Settlement ids of a province, the city first then by id (lot C4).
    #[func]
    fn province_settlements(&self, province: GString) -> PackedStringArray {
        let Some(state) = &self.state else {
            return PackedStringArray::new();
        };
        let Some(entry) = ProvinceId::new(province.to_string())
            .ok()
            .and_then(|p| state.province_state(&p))
        else {
            return PackedStringArray::new();
        };
        let mut list: Vec<&SettlementId> = vec![&entry.city];
        list.extend(entry.settlements.iter().filter(|s| *s != &entry.city));
        list.into_iter()
            .map(|s| GString::from(s.as_str()))
            .collect()
    }

    /// French message of the last failed `load_from_string` (lot C4: a save
    /// older than the settlements is refused), empty otherwise.
    #[func]
    fn last_load_error(&self) -> GString {
        GString::from(self.last_load_error.as_str())
    }
}

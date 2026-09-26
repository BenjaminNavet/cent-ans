//! `CampaignSim` API of lot DP2 (ADR 0075), in a secondary `#[godot_api]`
//! block: right of passage (path warning, incidents), diplomatic stances of
//! the map, and readable negotiations (every weighted reason, the single
//! blocking point and its counter-offer). The rules live in
//! `sim_campaign::{passage, stance, treaty_explain}`.

use data_model::{FactionId, ProvinceId};
use godot::prelude::*;
use sim_campaign::passage::{self, trespass_along};
use sim_campaign::stance::diplomatic_stance;
use sim_campaign::treaty_explain::explain_treaty;
use sim_campaign::ArmyId;

use crate::campaign_sim::CampaignSim;
use crate::campaign_sim_treaty::{
    articles_array, error_dict, faction_label, parse, province_label,
};

#[godot_api(secondary)]
impl CampaignSim {
    /// Lands of factions at peace that the march of `army_id` to map pixel
    /// `(x, y)` would cross without right of passage:
    /// `{ok, crossings[{province, province_name, owner, owner_name, halt}],
    /// halts, warning}`. `halt`: a season of the march ends there (an
    /// incident). `warning` is "" when the march is lawful.
    #[func]
    fn find_path_trespass(&self, army_id: GString, x: f64, y: f64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return vdict! { "ok" => false };
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return vdict! { "ok" => false };
        };
        let Some(faction) = state.army(&army).map(|a| a.faction.clone()) else {
            return vdict! { "ok" => false };
        };
        let Some(plan) = state.plan_path(data, &army, [x as f32, y as f32]) else {
            return vdict! { "ok" => false };
        };
        let crossings = trespass_along(state, data, &faction, &plan.points, &plan.turn_ends);
        let halts = crossings.iter().filter(|c| c.halt).count();
        let list: VarArray = crossings
            .iter()
            .map(|c| {
                vdict! {
                    "province" => c.province.as_str(),
                    "province_name" => province_label(data, &c.province).as_str(),
                    "owner" => c.owner.as_str(),
                    "owner_name" => faction_label(data, &c.owner).as_str(),
                    "halt" => c.halt,
                }
                .to_variant()
            })
            .collect();
        let warning = if crossings.is_empty() {
            String::new()
        } else {
            let mut owners: Vec<String> = Vec::new();
            for c in &crossings {
                let name = faction_label(data, &c.owner);
                if !owners.contains(&name) {
                    owners.push(name);
                }
            }
            if halts > 0 {
                format!(
                    "Sans droit de passage chez {} : camper sur leurs terres crée un incident diplomatique",
                    owners.join(", ")
                )
            } else {
                format!(
                    "Traverse sans droit de passage les terres de {} (incident si l'armée s'y arrête)",
                    owners.join(", ")
                )
            }
        };
        vdict! {
            "ok" => true,
            "crossings" => &list,
            "halts" => halts as i64,
            "warning" => warning.as_str(),
        }
    }

    /// Diplomatic stance of the player towards the controller of each of
    /// `province_ids` (the « Diplomatie » map mode): "self", "ally",
    /// "agreement", "neutral", "tension", "war", "vassal"; "" when the
    /// province is unknown or held by rebels.
    #[func]
    fn get_province_stances(&self, province_ids: PackedStringArray) -> PackedStringArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return PackedStringArray::new();
        };
        let player = state.player_faction().clone();
        let mut cache: std::collections::BTreeMap<FactionId, &'static str> = Default::default();
        province_ids
            .as_slice()
            .iter()
            .map(|id| {
                let controller = ProvinceId::new(id.to_string())
                    .ok()
                    .and_then(|p| state.province_controller(&p).cloned())
                    .filter(|c| c.as_str() != sim_campaign::diplomacy::REBELS_FACTION);
                let key = controller.map_or("", |c| {
                    *cache
                        .entry(c.clone())
                        .or_insert_with(|| diplomatic_stance(state, data, &player, &c).key())
                });
                GString::from(key)
            })
            .collect()
    }

    /// Stance of the player towards `faction` (same keys as
    /// `get_province_stances`) with its French label: `{key, label}`.
    #[func]
    fn get_faction_stance(&self, faction: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        if !state.factions.contains_key(&faction) {
            return VarDictionary::new();
        }
        let stance = diplomatic_stance(state, data, state.player_faction(), &faction);
        vdict! { "key" => stance.key(), "label" => stance.label_fr() }
    }

    /// Trespass between the player and `faction`, both ways:
    /// `{theirs{seasons, total, grievance, provinces[names]}, ours{...},
    /// access_given, access_received}` (`theirs`: their armies on our lands;
    /// `grievance`: the victim holds a casus belli).
    #[func]
    fn get_trespass(&self, faction: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(other) = FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        let player = state.player_faction().clone();
        if !state.factions.contains_key(&other) {
            return VarDictionary::new();
        }
        let side = |victim: &FactionId, intruder: &FactionId| -> VarDictionary {
            let record = state.factions[victim].ledger.trespassers.get(intruder);
            let provinces: PackedStringArray = record
                .map(|t| {
                    t.provinces
                        .iter()
                        .map(|p| GString::from(province_label(data, p).as_str()))
                        .collect()
                })
                .unwrap_or_default();
            vdict! {
                "seasons" => i64::from(record.map_or(0, |t| t.seasons)),
                "total" => i64::from(record.map_or(0, |t| t.total)),
                "grievance" => passage::has_grievance(state, victim, intruder),
                "provinces" => &provinces,
            }
        };
        vdict! {
            "theirs" => &side(&player, &other),
            "ours" => &side(&other, &player),
            "access_given" => state.factions[&player].ledger.military_access.contains(&other),
            "access_received" => state.factions[&other].ledger.military_access.contains(&player),
        }
    }

    /// `target`'s verdict on `articles` proposed by the player, made
    /// readable: `{ok, chance, accept, score, summary, lines[{text, value,
    /// article}], blocker{text, value, article} (empty if none),
    /// counter[articles] (empty if none), counter_chance, counter_text}`.
    /// `article` is the index of the article a line belongs to, -1 for a
    /// general consideration.
    #[func]
    fn explain_treaty(&self, target: GString, articles: VarArray) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return error_dict("aucune campagne en cours");
        };
        let (target, articles) = match parse(state, &target, &articles) {
            Ok(parsed) => parsed,
            Err(error) => return error_dict(&error),
        };
        let player = state.player_faction().clone();
        let explanation = explain_treaty(state, data, &player, &target, &articles);
        let index = |i: Option<usize>| i.map_or(-1, |i| i as i64);
        let lines: VarArray = explanation
            .lines
            .iter()
            .map(|l| {
                vdict! {
                    "text" => l.text.as_str(),
                    "value" => i64::from(l.value),
                    "article" => index(l.article),
                }
                .to_variant()
            })
            .collect();
        let blocker = explanation
            .blocker
            .as_ref()
            .map_or_else(VarDictionary::new, |b| {
                vdict! {
                    "text" => b.text.as_str(),
                    "value" => i64::from(b.value),
                    "article" => index(b.article),
                }
            });
        let counter = explanation
            .counter
            .as_deref()
            .map_or_else(VarArray::new, articles_array);
        vdict! {
            "ok" => true,
            "chance" => i64::from(explanation.chance),
            "accept" => explanation.accept,
            "score" => i64::from(explanation.score),
            "summary" => explanation.summary.as_str(),
            "lines" => &lines,
            "blocker" => &blocker,
            "counter" => &counter,
            "counter_chance" => i64::from(explanation.counter_chance),
            "counter_text" => explanation.counter_text.as_str(),
        }
    }
}

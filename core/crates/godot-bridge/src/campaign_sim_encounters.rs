//! `CampaignSim` map encounters API (lot CV3-3, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 2), in a secondary
//! `#[godot_api]` block. Read-only views plus the choice; the UI is lot CV3-4.

use data_model::EncounterId;
use godot::prelude::*;
use sim_campaign::{ArmyId, Order};

use crate::campaign_sim::{CampaignSim, Ctx, CtxMut};

#[godot_api(secondary)]
impl CampaignSim {
    /// Encounter sites the player sees (their cell inside his vision), in
    /// creation order: `[{id, encounter, title, position: Vector2 (map
    /// pixels, like `get_army_*`'s `position`), province, province_name,
    /// expires_turn, expires_in, claimed}]`.
    #[func]
    fn get_encounter_sites(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        state
            .encounter_site_views(data, state.player_faction())
            .iter()
            .map(|site| {
                vdict! {
                    "id" => i64::from(site.id),
                    "encounter" => site.encounter.as_str(),
                    "title" => site.title.as_str(),
                    "position" => Vector2::new(site.point[0], site.point[1]),
                    "province" => site.province.as_str(),
                    "province_name" => site.province_name.as_str(),
                    "expires_turn" => i64::from(site.expires_turn),
                    "expires_in" => i64::from(site.expires_in),
                    "claimed" => site.claimed,
                }
                .to_variant()
            })
            .collect()
    }

    /// Encounters of the player awaiting a choice, oldest first:
    /// `[{site, encounter, army, army_name, title, text, province,
    /// province_name, options[{index, label, available, reason,
    /// effects_text, outcome ("", "battle", "join"), default}]}]`.
    #[func]
    fn get_pending_encounters(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        state
            .pending_encounter_views(data, state.player_faction())
            .iter()
            .map(|view| {
                let options: VarArray = view
                    .options
                    .iter()
                    .map(|option| {
                        vdict! {
                            "index" => option.index as i64,
                            "label" => option.label.as_str(),
                            "available" => option.available,
                            "reason" => option.reason.as_deref().unwrap_or(""),
                            "effects_text" => option.effects_text.as_str(),
                            "outcome" => option.outcome.as_str(),
                            "default" => option.default,
                        }
                        .to_variant()
                    })
                    .collect();
                vdict! {
                    "site" => i64::from(view.site),
                    "encounter" => view.encounter.as_str(),
                    "army" => view.army.as_str(),
                    "army_name" => view.army_name.as_str(),
                    "title" => view.title.as_str(),
                    "text" => view.text.as_str(),
                    "province" => view.province.as_str(),
                    "province_name" => view.province_name.as_str(),
                    "options" => &options,
                }
                .to_variant()
            })
            .collect()
    }

    /// Answers a pending encounter (same as the `choose_encounter_option`
    /// order): `{ok, error}`.
    #[func]
    fn choose_encounter_option(&mut self, army: GString, site: i64, option: i64) -> VarDictionary {
        self.run_order("choose_encounter_option", || {
            let Some(army) = ArmyId::parse(&army.to_string()) else {
                return Err(format!("armée inconnue : {army}"));
            };
            Ok(Order::ChooseEncounterOption {
                army,
                site: site.max(0) as u32,
                option: option.max(0) as usize,
            })
        })
    }

    /// Staging (UI tests, screenshots): puts a site of `encounter` at the map
    /// point (`x`, `y`) in pixels; returns its id, -1 if refused.
    #[func]
    fn debug_place_encounter(&mut self, encounter: GString, x: f64, y: f64) -> i64 {
        if self.refuse_while_turn_pending("debug_place_encounter") {
            return -1;
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return -1;
        };
        let Ok(id) = EncounterId::new(encounter.to_string()) else {
            return -1;
        };
        state
            .debug_put_encounter_site(data, &id, [x as f32, y as f32])
            .map_or(-1, i64::from)
    }
}

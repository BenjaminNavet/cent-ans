//! `CampaignSim` side of the battles: pending battles, setups, forecasts, auto-resolution and staging.

use godot::prelude::*;
use sim_battle::BattleOutcome;

use crate::campaign_sim::{events_array, CampaignSim, Ctx, CtxMut};
use crate::convert::{from_dict, resolve_reply, to_dict};

/// CV3: the last battle classification as a dictionary (see
/// `CampaignSim.get_last_battle_outcome`).
fn battle_outcome_dict(state: &sim_campaign::CampaignState) -> VarDictionary {
    let Some(report) = &state.last_battle_outcome else {
        return VarDictionary::new();
    };
    let mut dict = vdict! {
        "turn" => i64::from(report.turn),
        "province" => report.province.as_str(),
        "attacker_faction" => report.attacker_faction.as_str(),
        "defender_faction" => report.defender_faction.as_str(),
        "attacker_class" => report.attacker.key.as_str(),
        "attacker_label" => report.attacker.label.as_str(),
        "defender_class" => report.defender.key.as_str(),
        "defender_label" => report.defender.label.as_str(),
    };
    if let Some(view) = report.class_of(&state.player_faction) {
        dict.set("player_class", view.key.as_str());
        dict.set("player_label", view.label.as_str());
    }
    dict
}

#[godot_api(secondary)]
impl CampaignSim {
    /// `[{index, attacker, defender, province, province_name, attacker_name,
    /// defender_name, player_side, attacker_strength, defender_strength, seed}]`.
    #[func]
    fn get_pending_battles(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        state
            .pending_battle_views(data)
            .iter()
            .map(|view| {
                let strength = |id: &sim_campaign::ArmyId| {
                    state.army(id).map_or(0, |a| i64::from(a.total_strength()))
                };
                let settlement_state = state.settlement_state(&view.location);
                let defender_strength = if view.siege {
                    settlement_state.map_or(0, |s| {
                        s.garrison.iter().map(|u| i64::from(u.strength)).sum()
                    })
                } else {
                    strength(&view.defender)
                };
                let breach = settlement_state
                    .and_then(|s| s.siege.as_ref())
                    .map_or(0, |s| i64::from(s.breach));
                let settlement_name = data
                    .settlements
                    .get(&view.location)
                    .map_or_else(|| view.location.to_string(), |s| s.name.display.clone());
                let province_name = data
                    .provinces
                    .get(&view.province)
                    .map_or_else(|| view.province.to_string(), |p| p.name.display.clone());
                let seed = state.seed
                    ^ (u64::from(state.turn()) << 20)
                    ^ ((view.index as u64) << 8)
                    ^ 0xBA77;
                vdict! {
                    "index" => view.index as i64,
                    "attacker" => view.attacker.as_str(),
                    "defender" => view.defender.as_str(),
                    "province" => view.province.as_str(),
                    "province_name" => province_name.as_str(),
                    "attacker_name" => view.attacker_name.as_str(),
                    "defender_name" => view.defender_name.as_str(),
                    "player_side" => view.player_side.map_or("", |s| s.key()),
                    "attacker_strength" => strength(&view.attacker),
                    "defender_strength" => defender_strength,
                    "siege" => view.siege,
                    "fortification" => i64::from(state.fortification_level(data, &view.location)),
                    "location" => view.location.as_str(),
                    "settlement_name" => settlement_name.as_str(),
                    "settlement_kind" => state.settlement_kind(&view.location).key(),
                    "breach" => breach,
                    "seed" => (seed & 0x7FFF_FFFF_FFFF) as i64,
                }
                .to_variant()
            })
            .collect()
    }

    /// Setup of pending battle `index` for `BattleSim.setup` (empty if unknown).
    #[func]
    fn get_battle_setup(&self, index: i64) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        match state.battle_setup(data, index.max(0) as usize) {
            Ok(setup) => {
                let mut dict = to_dict(&setup);
                // EP7: in the province and years of a historical map, the
                // battle is fought on the real site.
                if let Some(dir) = crate::campaign_sim::loaded_data_dir() {
                    if let Some((map, text)) =
                        crate::historical_battles::campaign_site(&dir, &setup, state.year())
                    {
                        dict.set("historical_site", text.as_str());
                        dict.set("historical_site_id", map.id.as_str());
                        dict.set("historical_horizon", map.horizon_key().as_str());
                    }
                }
                // EP8: hour of the day drawn from the battle (turn, index,
                // province), no random stream consumed.
                let key = sim_battle::time_of_day::campaign_battle_key(
                    state.turn(),
                    index.max(0) as usize,
                    &setup.province,
                );
                dict.set(
                    "hour",
                    sim_battle::TimeOfDayRules::bundled().campaign_hour(key),
                );
                dict
            }
            Err(error) => {
                godot_warn!("CampaignSim.get_battle_setup({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// UB1: estimated balance of pending battle `index` for the pre-battle
    /// screen (`battle_forecast.rs`): `{attacker_power, defender_power,
    /// attacker_share, attacker_win_chance, attacker_soldiers,
    /// defender_soldiers, attacker_reinforcements, defender_reinforcements,
    /// modifiers, can_withdraw, siege}`; empty if unknown.
    #[func]
    fn get_battle_forecast(&self, index: i64) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        match state.battle_forecast(data, index.max(0) as usize) {
            Ok(forecast) => to_dict(&forecast),
            Err(error) => {
                godot_warn!("CampaignSim.get_battle_forecast({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// UB1: the player calls off pending battle `index` (attacker only; an
    /// assault is postponed, the siege goes on) → `{ok, error, events}`.
    #[func]
    fn withdraw_pending_battle(&mut self, index: i64) -> VarDictionary {
        if self.refuse_while_turn_pending("withdraw_pending_battle") {
            return resolve_reply(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return resolve_reply(Err("aucune campagne en cours".to_owned()));
        };
        match state.withdraw_pending_battle(data, index.max(0) as usize) {
            Ok(events) => {
                let mut dict = resolve_reply(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => resolve_reply(Err(error.to_string())),
        }
    }

    /// Applies a `BattleSim.get_outcome()` dictionary → `{ok, error, events}`.
    #[func]
    fn resolve_battle(&mut self, index: i64, outcome: VarDictionary) -> VarDictionary {
        if self.refuse_while_turn_pending("resolve_battle") {
            return resolve_reply(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return resolve_reply(Err("aucune campagne en cours".to_owned()));
        };
        let before = state.last_battle_outcome.clone();
        let result = from_dict::<BattleOutcome>(&outcome)
            .map_err(|e| format!("résultat invalide : {e}"))
            .and_then(|outcome| {
                state
                    .resolve_pending_battle(data, index.max(0) as usize, &outcome)
                    .map_err(|e| e.to_string())
            });
        match result {
            Ok(events) => {
                let mut dict = resolve_reply(Ok(()));
                dict.set("events", &events_array(&events));
                // CV3: class of the result (heroic, disaster...).
                if state.last_battle_outcome != before {
                    dict.set("outcome", &battle_outcome_dict(state));
                }
                dict
            }
            Err(error) => resolve_reply(Err(error)),
        }
    }

    /// CV3: class of the last field battle (auto-resolved or 3D) →
    /// `{turn, province, attacker_faction, defender_faction, attacker_class,
    /// attacker_label, defender_class, defender_label[, player_class,
    /// player_label]}`; classes: `heroic`, `decisive`, `pyrrhic`, `victory`,
    /// `honourable_defeat`, `disaster`, `defeat`. Empty before any battle.
    #[func]
    fn get_last_battle_outcome(&self) -> VarDictionary {
        self.state
            .as_ref()
            .map_or_else(VarDictionary::new, battle_outcome_dict)
    }

    /// Auto-resolves pending battle `index` now; returns its events.
    #[func]
    fn auto_resolve_battle(&mut self, index: i64) -> VarArray {
        if self.refuse_while_turn_pending("auto_resolve_battle") {
            return VarArray::new();
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return VarArray::new();
        };
        match state.auto_resolve_pending(data, index.max(0) as usize) {
            Ok(events) => events_array(&events),
            Err(error) => {
                godot_warn!("CampaignSim.auto_resolve_battle({index}): {error}");
                VarArray::new()
            }
        }
    }

    /// Player setting: fight own battles in 3D (`true`, default) or always
    /// auto-resolve them.
    #[func]
    fn set_interactive_battles(&mut self, enabled: bool) {
        if self.refuse_while_turn_pending("set_interactive_battles") {
            return;
        }
        if let Some(state) = &mut self.state {
            state.interactive_battles = enabled;
        }
    }

    /// Debug (smoke test, screenshots): puts `army` in siege of `province`
    /// (garrisoned if empty) and records a pending siege battle; returns its
    /// index or -1.
    #[func]
    fn debug_stage_siege(&mut self, army: GString, province: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_siege") {
            return -1;
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return -1;
        };
        let Some(army) = sim_campaign::ArmyId::parse(&army.to_string()) else {
            return -1;
        };
        let Ok(province) = data_model::ProvinceId::new(province.to_string().as_str()) else {
            return -1;
        };
        match state.debug_stage_siege(data, &army, &province) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_siege: {error}");
                -1
            }
        }
    }

    /// SG2 demo: `army` besieges the town drawn from landmark plan
    /// `landmark` (`data/landmarks/<id>.json`, e.g. `avignon`, `bruges`),
    /// at war with its holder if needed. Returns the battle index or -1.
    #[func]
    fn debug_stage_landmark_siege(&mut self, army: GString, landmark: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_landmark_siege") {
            return -1;
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return -1;
        };
        let Some(army) = sim_campaign::ArmyId::parse(&army.to_string()) else {
            return -1;
        };
        match state.debug_stage_landmark_siege(data, &army, &landmark.to_string()) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_landmark_siege: {error}");
                -1
            }
        }
    }

    /// Debug (smoke test, screenshots): brings `defender` to `attacker` and
    /// records a pending battle; returns its index or -1.
    #[func]
    fn debug_stage_battle(&mut self, attacker: GString, defender: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_battle") {
            return -1;
        }
        let Some(CtxMut { state, .. }) = self.ctx_mut() else {
            return -1;
        };
        let (Some(a), Some(d)) = (
            sim_campaign::ArmyId::parse(&attacker.to_string()),
            sim_campaign::ArmyId::parse(&defender.to_string()),
        ) else {
            return -1;
        };
        match state.debug_stage_battle(&a, &d) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_battle: {error}");
                -1
            }
        }
    }
}

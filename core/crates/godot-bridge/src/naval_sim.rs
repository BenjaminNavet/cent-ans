//! Naval battles on the GDExtension side: the naval methods of `CampaignSim`
//! (a secondary `#[godot_api]` block). Naval battles are auto-resolved only.

use crate::convert::{resolve_reply, to_dict};
use godot::prelude::*;

use crate::campaign_sim::{events_array, CampaignSim, Ctx, CtxMut};

#[godot_api(secondary)]
impl CampaignSim {
    /// Naval battles waiting for the player (lot NV1): `[{index, army, from,
    /// to, sea, sea_name, interceptor, interceptor_name, faction,
    /// faction_name, interceptor_ships, transport_ships, interceptor_men,
    /// army_men, win_chance, player_side, seed}]`.
    #[func]
    fn get_pending_naval_battles(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        state
            .pending_naval_views(data)
            .iter()
            .map(|view| {
                let mut dict = to_dict(view);
                let seed = state.naval_battle_seed(view.index).unwrap_or(1);
                dict.set("seed", (seed & 0x7FFF_FFFF_FFFF) as i64);
                dict.set("player_side", view.player_side.key());
                dict.to_variant()
            })
            .collect()
    }

    /// Setup of pending naval battle `index` for the pre-battle dialog.
    #[func]
    fn get_naval_battle_setup(&self, index: i64) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        match state.naval_battle_setup(data, index.max(0) as usize) {
            Ok(setup) => to_dict(&setup),
            Err(error) => {
                godot_warn!("CampaignSim.get_naval_battle_setup({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// Auto-resolves pending naval battle `index` → `{ok, error, events}`.
    #[func]
    fn auto_resolve_naval_battle(&mut self, index: i64) -> VarDictionary {
        if self.refuse_while_turn_pending("auto_resolve_naval_battle") {
            return resolve_reply(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return resolve_reply(Err("aucune campagne en cours".to_owned()));
        };
        match state.auto_resolve_naval_battle(data, index.max(0) as usize) {
            Ok(events) => {
                let mut dict = resolve_reply(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => resolve_reply(Err(error.to_string())),
        }
    }

    /// The intercepted fleet puts back into port → `{ok, error, events}`.
    #[func]
    fn withdraw_naval_battle(&mut self, index: i64) -> VarDictionary {
        if self.refuse_while_turn_pending("withdraw_naval_battle") {
            return resolve_reply(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return resolve_reply(Err("aucune campagne en cours".to_owned()));
        };
        match state.withdraw_naval_battle(data, index.max(0) as usize) {
            Ok(events) => {
                let mut dict = resolve_reply(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => resolve_reply(Err(error.to_string())),
        }
    }

    /// `{fleets: {faction: {class: count}}, control: {sea: {faction,
    /// level, name}}, blockaded: [settlement]}`.
    #[func]
    fn get_naval_state(&self) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let mut naval = state.naval.clone();
        naval.ensure(data);
        let mut control = VarDictionary::new();
        for (sea, c) in &naval.control {
            control.set(
                sea.as_str(),
                &vdict! {
                    "faction" => c.faction.as_str(),
                    "level" => i64::from(c.level),
                    "name" => data.naval.sea_name(sea).as_str(),
                },
            );
        }
        let mut dict = to_dict(&naval);
        dict.set("control", &control);
        dict
    }

    /// Debug (smoke test, screenshots): `interceptor` bars `army`'s crossing
    /// to `to_port`; returns the index of the pending naval battle or -1.
    #[func]
    fn debug_stage_naval(&mut self, army: GString, to_port: GString, interceptor: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_stage_naval") {
            return -1;
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return -1;
        };
        let (Some(army), Ok(to), Ok(by)) = (
            sim_campaign::ArmyId::parse(&army.to_string()),
            data_model::SettlementId::new(to_port.to_string().as_str()),
            data_model::FactionId::new(interceptor.to_string().as_str()),
        ) else {
            return -1;
        };
        state
            .debug_stage_naval(data, &army, &to, &by)
            .map_or(-1, |i| i as i64)
    }
}

//! `CampaignSim` army traditions (lot TW2-T5, ADR 0109): the traditions
//! panel of an army (experience, rank, choices), the choice order, the
//! armies of the player with a rank to spend (notification) and a staging
//! helper for the headless tests.

use godot::prelude::*;
use sim_campaign::{ArmyId, Order};

use crate::campaign_sim::{order_result, CampaignSim};

#[godot_api(secondary)]
impl CampaignSim {
    /// Traditions of `army_id`: `{xp, rank, max_rank, next_threshold (-1 at
    /// the last rank), pending, name (capitalised), named (the name is kept
    /// when the general changes), banner_house,
    /// effects_text, chosen: [names], options: [{id, branch, branch_name,
    /// tier, name, description, effects_text, chosen, allowed, reason}]}`.
    /// Empty for an unknown army.
    #[func]
    fn get_army_traditions(&self, army_id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return VarDictionary::new();
        };
        let Some(view) = state.army_tradition_view(data, &army) else {
            return VarDictionary::new();
        };
        let mut options = VarArray::new();
        let mut chosen = PackedStringArray::new();
        for option in &view.options {
            if option.chosen {
                chosen.push(option.name.as_str());
            }
            options.push(
                &vdict! {
                    "id" => option.id.as_str(),
                    "branch" => option.branch.key(),
                    "branch_name" => option.branch_name.as_str(),
                    "tier" => i64::from(option.tier),
                    "name" => option.name.as_str(),
                    "description" => option.description.as_str(),
                    "effects_text" => option.effects_text.as_str(),
                    "chosen" => option.chosen,
                    "allowed" => option.allowed,
                    "reason" => option.reason.as_str(),
                }
                .to_variant(),
            );
        }
        vdict! {
            "xp" => i64::from(view.xp),
            "rank" => i64::from(view.rank),
            "max_rank" => i64::from(view.max_rank),
            "next_threshold" => view.next_threshold.map_or(-1, i64::from),
            "pending" => i64::from(view.pending),
            "name" => sim_campaign::events::capitalize(&view.name).as_str(),
            "named" => view.named,
            "banner_house" => view.banner_house.as_deref().unwrap_or(""),
            "effects_text" => view.effects.text_fr().as_str(),
            "chosen" => &chosen,
            "options" => &options,
        }
    }

    /// Armies of the player with a rank to spend: `[{army, name, pending}]`
    /// (notification « une tradition est à choisir »).
    #[func]
    fn get_armies_with_pending_traditions(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let player = state.player_faction();
        state
            .armies
            .iter()
            .filter(|(_, a)| &a.faction == player)
            .filter_map(|(id, a)| {
                let pending = sim_campaign::traditions::pending_choices(data, a);
                (pending > 0).then(|| {
                    vdict! {
                        "army" => id.as_str(),
                        "name" => sim_campaign::events::capitalize(&state.army_name(data, id)).as_str(),
                        "pending" => i64::from(pending),
                    }
                    .to_variant()
                })
            })
            .collect()
    }

    /// `army_id` takes `tradition`; same as the `ChooseArmyTradition` order.
    /// `{ok, error}` (French reason when refused).
    #[func]
    fn choose_army_tradition(&mut self, army_id: GString, tradition: GString) -> VarDictionary {
        if self.refuse_while_turn_pending("choose_army_tradition") {
            return order_result(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return order_result(Err("aucune campagne en cours".to_owned()));
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return order_result(Err(format!("armée inconnue : {army_id}")));
        };
        let order = Order::ChooseArmyTradition {
            army,
            tradition: tradition.to_string(),
        };
        order_result(state.submit_order(data, order).map_err(|e| e.to_string()))
    }

    /// Staging (UI tests): `army_id` gains `xp` army experience at once;
    /// `false` for an unknown army.
    #[func]
    fn debug_grant_army_xp(&mut self, army_id: GString, xp: i64) -> bool {
        if self.refuse_while_turn_pending("debug_grant_army_xp") {
            return false;
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return false;
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return false;
        };
        if !state.armies.contains_key(&army) {
            return false;
        }
        let mut events = Vec::new();
        sim_campaign::traditions::grant_army_xp(
            state,
            data,
            &army,
            xp.clamp(0, i64::from(u32::MAX)) as u32,
            &mut events,
        );
        state.pending_events.extend(events);
        true
    }
}

//! `CampaignSim` API of H5 « Monnaie » and H6 « Rançons et ordres de
//! chevalerie », kept in its own secondary `#[godot_api]` block (ADR 0002).
//! Orders go through `submit_order` (`set_coinage`, `pay_ransom`,
//! `set_ransom_terms`, `release_on_parole`, `found_chivalric_order`).
//! Documented in `docs/design/h5-h6-api.md`.

use data_model::{CharacterId, FactionId};
use godot::prelude::*;
use sim_campaign::coinage::{self, CoinageLevel};
use sim_campaign::{chivalry, ransom, CampaignState, RansomTerms};

use crate::campaign_sim::CampaignSim;

fn terms_dict(terms: &RansomTerms) -> VarDictionary {
    let province = match terms {
        RansomTerms::Province { province } => province.as_str(),
        _ => "",
    };
    vdict! { "kind" => terms.key(), "province" => province }
}

fn captive_dict(
    state: &CampaignState,
    data: &data_model::GameData,
    id: &CharacterId,
) -> VarDictionary {
    let c = &state.characters[id];
    let amount = ransom::ransom_amount(state, data, id);
    let captor = c.captor.as_ref().map_or("", |f| f.as_str());
    let cedable: PackedStringArray = c
        .captor
        .as_ref()
        .map(|captor| ransom::cedable_provinces(state, data, &c.faction, captor))
        .unwrap_or_default()
        .iter()
        .map(|p| GString::from(p.as_str()))
        .collect();
    let mut plans = VarArray::new();
    for n in 1..=ransom::MAX_INSTALLMENTS {
        let (total, installment) = ransom::installment_plan(amount, n);
        plans.push(
            &vdict! { "installments" => n, "total" => total, "installment" => installment }
                .to_variant(),
        );
    }
    vdict! {
        "character" => id.as_str(),
        "name" => state.character_name(data, id).as_str(),
        "faction" => c.faction.as_str(),
        "captor" => captor,
        "rank" => ransom::captive_rank(state, id).key(),
        "rank_label" => ransom::captive_rank(state, id).label_fr(),
        "prestige" => c.prestige,
        "ransom" => amount,
        "terms" => &terms_dict(&c.ransom_terms.clone().unwrap_or_default()),
        "plans" => &plans,
        "cedable_provinces" => &cedable,
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Coinage of `faction` (empty: the player's): `{level, label,
    /// price_level, changed_this_year, seigniorage, recoinage,
    /// seigniorage_last_turn, recoinage_last_turn, options[{level, label,
    /// seigniorage, recoinage, inflation, deflation, burgher_unrest,
    /// prestige, current}]}`; `seigniorage`/`recoinage` are forecasts for
    /// this season in livres.
    #[func]
    fn get_coinage(&self, faction: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let id = if faction.is_empty() {
            state.player_faction().clone()
        } else {
            match FactionId::new(faction.to_string()) {
                Ok(id) => id,
                Err(_) => return VarDictionary::new(),
            }
        };
        let Some(f) = state.faction_state(&id) else {
            return VarDictionary::new();
        };
        let mut options = VarArray::new();
        for level in CoinageLevel::ALL {
            let params = level.params();
            options.push(
                &vdict! {
                    "level" => level.key(),
                    "label" => level.label_fr(),
                    "seigniorage" => coinage::seigniorage_for(state, data, &id, level),
                    "recoinage" => coinage::recoinage_for(state, data, &id, level),
                    "inflation" => params.inflation,
                    "deflation" => params.deflation,
                    "burgher_unrest" => params.burgher_unrest,
                    "prestige" => params.prestige,
                    "current" => level == f.coinage,
                }
                .to_variant(),
            );
        }
        vdict! {
            "level" => f.coinage.key(),
            "label" => f.coinage.label_fr(),
            "price_level" => f.price_level,
            "changed_this_year" => f.coinage_changed_year == Some(state.year()),
            "seigniorage" => coinage::seigniorage(state, data, &id),
            "recoinage" => coinage::recoinage(state, data, &id),
            "seigniorage_last_turn" => f.seigniorage_last_turn,
            "recoinage_last_turn" => f.recoinage_last_turn,
            "options" => &options,
        }
    }

    /// Price level of every faction: `{faction_id: price_level}`.
    #[func]
    fn get_price_levels(&self) -> VarDictionary {
        let mut dict = VarDictionary::new();
        if let Some(state) = &self.state {
            for (id, f) in &state.factions {
                dict.set(id.as_str(), f.price_level);
            }
        }
        dict
    }

    /// Captives of both sides for the player: `{ours: [...], held: [...],
    /// debts: [...]}`. A captive: `{character, name, faction, captor, rank,
    /// rank_label, prestige, ransom, terms{kind, province}, plans[
    /// {installments, total, installment}], cedable_provinces[]}`. A debt:
    /// `{character, name, creditor, remaining, installment, next_due_turn,
    /// missed}`.
    #[func]
    fn get_ransoms(&self) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let player = state.player_faction().clone();
        let mut ours = VarArray::new();
        let mut held = VarArray::new();
        for (id, c) in &state.characters {
            if !c.alive || !c.captive {
                continue;
            }
            if c.faction == player {
                ours.push(&captive_dict(state, data, id).to_variant());
            } else if c.captor.as_ref() == Some(&player) {
                held.push(&captive_dict(state, data, id).to_variant());
            }
        }
        let mut debts = VarArray::new();
        if let Some(f) = state.faction_state(&player) {
            for debt in &f.ransom_debts {
                debts.push(
                    &vdict! {
                        "character" => debt.character.as_str(),
                        "name" => state.character_name(data, &debt.character).as_str(),
                        "creditor" => debt.creditor.as_str(),
                        "remaining" => debt.remaining,
                        "installment" => debt.installment,
                        "next_due_turn" => debt.next_due_turn,
                        "missed" => debt.missed,
                    }
                    .to_variant(),
                );
            }
        }
        vdict! { "ours" => &ours, "held" => &held, "debts" => &debts }
    }

    /// Chivalric orders for the player: `{founded: {order, name,
    /// founded_turn, collapsed, members[{character, name}]} or empty,
    /// options[{id, name, description, cost, prestige_required, members,
    /// historical_members, member_loyalty, member_morale, founder_prestige,
    /// yearly_prestige, min_year, available, reason, founder}]}`.
    #[func]
    fn get_chivalric_orders(&self) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let player = state.player_faction().clone();
        let founded = state
            .faction_state(&player)
            .and_then(|f| f.chivalric_order.as_ref())
            .map(|o| {
                let members: VarArray = o
                    .members
                    .iter()
                    .map(|m| {
                        vdict! {
                            "character" => m.as_str(),
                            "name" => state.character_name(data, m).as_str(),
                        }
                        .to_variant()
                    })
                    .collect();
                vdict! {
                    "order" => o.order.as_str(),
                    "name" => data.chivalric_orders.get(&o.order).map_or(o.order.as_str(), |d| d.name.display.as_str()),
                    "founded_turn" => o.founded_turn,
                    "collapsed" => o.collapsed,
                    "members" => &members,
                }
            })
            .unwrap_or_default();
        let mut options = VarArray::new();
        for order in chivalry::orders_for(data, &player) {
            let blocker = chivalry::found_blocker(state, data, &player, &order.id);
            let founder = chivalry::founder_of(state, &order.id);
            options.push(
                &vdict! {
                    "id" => order.id.as_str(),
                    "name" => order.name.display.as_str(),
                    "description" => order.description.as_str(),
                    "cost" => chivalry::foundation_cost(state, &player, order),
                    "prestige_required" => order.prestige_required,
                    "members" => order.members,
                    "historical_members" => order.historical_members.as_str(),
                    "member_loyalty" => order.member_loyalty,
                    "member_morale" => order.member_morale,
                    "founder_prestige" => order.founder_prestige,
                    "yearly_prestige" => order.yearly_prestige,
                    "min_year" => order.min_year.unwrap_or(0),
                    "available" => blocker.is_none(),
                    "reason" => blocker.map(|e| e.to_string()).unwrap_or_default().as_str(),
                    "founder" => founder.as_ref().map_or("", |f| f.as_str()),
                }
                .to_variant(),
            );
        }
        vdict! { "founded" => &founded, "options" => &options }
    }
}

//! `CampaignSim` diplomacy and religion API (spec M5 § 3), in a secondary
//! `#[godot_api]` block so that each milestone keeps its own file.

use data_model::{FactionId, GameData, ProvinceId};
use godot::prelude::*;
use sim_campaign::diplomacy::{call_to_arms_forecast, CallForecast};
use sim_campaign::negotiation::{evaluate_treaty, Article};
use sim_campaign::religion::{faction_religion, is_excommunicated, religion_display};
use sim_campaign::{CampaignState, Order};

use crate::campaign_sim::{CampaignSim, Ctx};
use crate::convert::{reasons_array, variant_to_json};

#[godot_api(secondary)]
impl CampaignSim {
    /// Every other living faction seen from `faction` (attitude is theirs
    /// towards `faction`).
    #[func]
    fn get_diplomacy(&self, faction: GString) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarArray::new();
        };
        if !state.factions.contains_key(&faction) {
            return VarArray::new();
        }
        state
            .diplomacy_view(data, &faction)
            .iter()
            .map(|entry| {
                let static_faction = data.factions.get(&entry.faction);
                let religion = faction_religion(state, data, &entry.faction);
                let allies: PackedStringArray = state.factions[&entry.faction]
                    .allies
                    .iter()
                    .map(|a| GString::from(a.as_str()))
                    .collect();
                let claims: PackedStringArray =
                    entry.claims.iter().map(GString::from).collect();
                vdict! {
                    "id" => entry.faction.as_str(),
                    "name" => static_faction.map_or(entry.faction.as_str(), |f| f.short_or_display_name()),
                    "color" => static_faction.map_or("#888888", |f| f.heraldry.primary_color.as_str()),
                    "status" => entry.relation.key(),
                    "attitude" => i64::from(entry.attitude),
                    "attitude_band" => entry.attitude_band.as_str(),
                    "attitude_reasons" => &timed_reasons_array(&entry.attitude_reasons, &entry.reason_turns),
                    "allies_info" => &faction_refs(data, &entry.allies),
                    "enemies_info" => &faction_refs(data, &entry.enemies),
                    "vassals_info" => &faction_refs(data, &entry.vassals),
                    "truce_turns_left" => i64::from(entry.truce_turns_left),
                    "embargo_by_us" => entry.embargo_by_us,
                    "embargo_on_us" => entry.embargo_on_us,
                    "war_score" => i64::from(entry.war_score),
                    "casus_belli" => entry.casus_belli.as_deref().unwrap_or(""),
                    "claims" => &claims,
                    "religion" => religion.as_ref().map_or("", |r| r.as_str()),
                    "religion_name" => religion.as_ref().map_or(String::new(), |r| religion_display(state, data, r)).as_str(),
                    "loyalty" => entry.loyalty.map_or(-1, i64::from),
                    "power" => state.faction_power(&entry.faction).round() as i64,
                    "allies" => &allies,
                    // DP1: ruler, war weariness, standing treaties.
                    "ruler" => sim_campaign::negotiation::ruler_name(state, data, &entry.faction).as_str(),
                    "weariness" => i64::from(sim_campaign::negotiation::weariness(state, &entry.faction)),
                    "trade_agreement" => state.factions[&faction].ledger.trade_agreements.contains(&entry.faction),
                    "access_given" => state.factions[&faction].ledger.military_access.contains(&entry.faction),
                    "access_received" => state.factions[&entry.faction].ledger.military_access.contains(&faction),
                }
                .to_variant()
            })
            .collect()
    }

    /// Verdict of the recipient on a diplomatic order before sending it:
    /// `{accept, score, reasons[{text, value}]}`. For `declare_war`, `reasons`
    /// lists the motive and the reputation cost.
    #[func]
    fn evaluate_proposal(&self, order: VarDictionary) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return verdict(false, 0, &[("aucune campagne en cours".to_owned(), 0)]);
        };
        let parsed = variant_to_json(&order.to_variant())
            .and_then(|json| serde_json::from_value::<Order>(json).map_err(|e| e.to_string()));
        let order = match parsed {
            Ok(order) => order,
            Err(error) => {
                let message = crate::campaign_sim::invalid_order_message(&error);
                return verdict(false, 0, &[(message, 0)]);
            }
        };
        let player = state.player_faction().clone();
        let (target, articles) = match order {
            Order::DeclareWar { target } => return war_verdict(state, data, &player, &target),
            Order::RequestPapalMediation { target } => (
                target,
                vec![Article::Mediation {
                    turns: sim_campaign::diplomacy::MEDIATION_TRUCE_TURNS,
                }],
            ),
            other => match other.proposal(state, &player) {
                Some((target, treaty)) => (target, treaty.articles),
                None => return verdict(true, 0, &[]),
            },
        };
        let evaluation = evaluate_treaty(state, data, &player, &target, &articles);
        verdict(evaluation.accept, evaluation.score, &evaluation.reasons())
    }

    /// Offers waiting for the player's answer.
    #[func]
    fn get_offers(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        let Some(player) = state.factions.get(state.player_faction()) else {
            return VarArray::new();
        };
        player
            .offers
            .iter()
            .map(|offer| {
                let kind = offer.proposal.kind();
                vdict! {
                    "id" => i64::from(offer.id),
                    "from" => offer.from.as_str(),
                    "from_name" => data.factions.get(&offer.from).map_or(offer.from.as_str(), |f| f.short_or_display_name()),
                    "kind" => kind,
                    "text" => offer.text_fr.as_str(),
                    "expires_in" => i64::from(offer.expires_turn.saturating_sub(state.turn()).saturating_sub(1)),
                }
                .to_variant()
            })
            .collect()
    }

    /// `{religion, religion_name, papal_favor, excommunicated, turns_left,
    /// schism, obedience_choice_pending}`.
    #[func]
    fn get_religion_state(&self, faction: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        let Some(f) = state.factions.get(&faction) else {
            return VarDictionary::new();
        };
        let religion = faction_religion(state, data, &faction);
        let pending = f.offers.iter().any(|o| o.proposal.is_obedience());
        vdict! {
            "religion" => religion.as_ref().map_or("", |r| r.as_str()),
            "religion_name" => religion.as_ref().map_or(String::new(), |r| religion_display(state, data, r)).as_str(),
            "papal_favor" => i64::from(f.papal_favor),
            "excommunicated" => is_excommunicated(state, &faction),
            "turns_left" => f.excommunicated_until.map_or(0, |u| i64::from(u.saturating_sub(state.turn()))),
            "schism" => state.schism,
            "obedience_choice_pending" => pending,
        }
    }

    /// `{religion, religion_name, heresy, heresy_religion, heresy_name}` of a province.
    #[func]
    fn get_province_religion(&self, province: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Ok(id) = ProvinceId::new(province.to_string()) else {
            return VarDictionary::new();
        };
        let (Some(p), Some(pd)) = (state.province_state(&id), data.provinces.get(&id)) else {
            return VarDictionary::new();
        };
        let heresy_name = p
            .heresy_religion
            .as_ref()
            .and_then(|r| data.religions.get(r))
            .map_or("", |r| r.name.display.as_str());
        vdict! {
            "religion" => pd.religion.as_str(),
            "religion_name" => religion_display(state, data, &pd.religion).as_str(),
            "heresy" => i64::from(p.heresy),
            "heresy_religion" => p.heresy_religion.as_ref().map_or("", |r| r.as_str()),
            "heresy_name" => heresy_name,
        }
    }

    /// Relation of the player with each province's controller, in the order
    /// of `province_ids` (for the diplomacy map mode): "self", "war",
    /// "truce", "peace", "alliance", "vassal", "suzerain", "" if unknown.
    #[func]
    fn get_province_relations(&self, province_ids: PackedStringArray) -> PackedStringArray {
        let Some(Ctx { state, .. }) = self.ctx() else {
            return PackedStringArray::new();
        };
        let player = state.player_faction().clone();
        province_ids
            .as_slice()
            .iter()
            .map(|id| {
                let key = ProvinceId::new(id.to_string())
                    .ok()
                    .and_then(|p| state.province_controller(&p).cloned())
                    .map_or("", |controller| {
                        if controller == player {
                            "self"
                        } else {
                            state.relation(&player, &controller).key()
                        }
                    });
                GString::from(key)
            })
            .collect()
    }

    /// Answers an offer (same result shape as `submit_order`).
    #[func]
    fn answer_offer(&mut self, offer: i64, accept: bool) -> VarDictionary {
        self.run_order("answer_offer", || {
            Ok(Order::AnswerOffer {
                offer: offer.max(0) as u32,
                accept,
            })
        })
    }
}

/// `reasons_array` with `turns_left` (0: permanent or structural reason).
fn timed_reasons_array(
    reasons: &[(String, i32)],
    turns: &std::collections::BTreeMap<String, u32>,
) -> VarArray {
    reasons
        .iter()
        .map(|(text, value)| {
            vdict! {
                "text" => text.as_str(),
                "value" => i64::from(*value),
                "turns_left" => i64::from(turns.get(text).copied().unwrap_or(0)),
            }
            .to_variant()
        })
        .collect()
}

/// `[{id, name}]` of factions, for clickable names in the faction sheet.
fn faction_refs(data: &GameData, ids: &[FactionId]) -> VarArray {
    ids.iter()
        .map(|id| {
            vdict! { "id" => id.as_str(), "name" => data.faction_name(id).as_str() }.to_variant()
        })
        .collect()
}

fn verdict(accept: bool, score: i32, reasons: &[(String, i32)]) -> VarDictionary {
    vdict! {
        "accept" => accept,
        "score" => i64::from(score),
        "reasons" => &reasons_array(reasons),
    }
}

fn war_verdict(
    state: &CampaignState,
    data: &GameData,
    player: &FactionId,
    target: &FactionId,
) -> VarDictionary {
    let mut reasons = Vec::new();
    if state.is_at_war(player, target) {
        return verdict(false, 0, &[("Déjà en guerre".to_owned(), 0)]);
    }
    if state.has_truce(player, target) {
        reasons.push((
            "Trêve rompue : parjure (-40 auprès de tous)".to_owned(),
            -40,
        ));
    }
    match state.casus_belli(data, player, target) {
        Some(motive) => reasons.push((format!("Motif : {motive}"), 0)),
        None => reasons.push(("Sans motif : réputation -20 auprès de tous".to_owned(), -20)),
    }
    // WH diploa: how each ally of the target (and its suzerain) will answer.
    let defender = &state.factions[target];
    let mut called: Vec<FactionId> = defender.allies.iter().cloned().collect();
    called.extend(defender.suzerain.iter().cloned());
    called.sort();
    called.dedup();
    let mut forecast = VarArray::new();
    for ally in called
        .iter()
        .filter(|a| *a != player && state.factions.get(*a).is_some_and(|f| f.alive))
        .filter(|a| !state.is_at_war(a, player) && !state.is_allied(a, player))
    {
        let name = data.faction_name(ally);
        let (answer, line) = match call_to_arms_forecast(state, data, ally, target, player) {
            CallForecast::Joins => ("joins", format!("{name} viendra à son secours")),
            CallForecast::Hesitates => ("hesitates", format!("{name} hésite")),
            CallForecast::Refuses(why) => ("refuses", format!("{name} refusera : {why}")),
        };
        reasons.push((line.clone(), 0));
        forecast.push(
            &vdict! {
                "id" => ally.as_str(),
                "name" => name.as_str(),
                "answer" => answer,
                "text" => line.as_str(),
            }
            .to_variant(),
        );
    }
    let prestige = state.declaration_prestige_cost(data, player, target);
    reasons.push((
        if prestige == 0 {
            "Prestige du souverain : inchangé".to_owned()
        } else {
            format!("Prestige du souverain : {prestige}")
        },
        0, // already counted in the opinion lines above
    ));
    let score = reasons.iter().map(|(_, v)| v).sum();
    let mut result = verdict(true, score, &reasons);
    result.set("allies", &forecast);
    result
}

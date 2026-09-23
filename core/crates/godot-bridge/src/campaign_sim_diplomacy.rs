//! `CampaignSim` diplomacy and religion API (spec M5 § 3), in a secondary
//! `#[godot_api]` block so that each milestone keeps its own file.

use data_model::{FactionId, GameData, ProvinceId};
use godot::prelude::*;
use sim_campaign::diplomacy::{evaluate, Proposal, RelationKind};
use sim_campaign::religion::{faction_religion, is_excommunicated, religion_display};
use sim_campaign::{CampaignState, Order};

use crate::campaign_sim::{order_result, CampaignSim};
use crate::convert::variant_to_json;

#[godot_api(secondary)]
impl CampaignSim {
    /// Every other living faction seen from `faction` (attitude is theirs
    /// towards `faction`).
    #[func]
    fn get_diplomacy(&self, faction: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
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
                    "status" => relation_key(entry.relation),
                    "attitude" => i64::from(entry.attitude),
                    "attitude_reasons" => &reasons_array(&entry.attitude_reasons),
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
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return verdict(false, 0, &[("aucune campagne en cours".to_owned(), 0)]);
        };
        let parsed = variant_to_json(&order.to_variant())
            .and_then(|json| serde_json::from_value::<Order>(json).map_err(|e| e.to_string()));
        let order = match parsed {
            Ok(order) => order,
            Err(error) => return verdict(false, 0, &[(format!("ordre invalide : {error}"), 0)]),
        };
        let player = state.player_faction().clone();
        let (target, proposal) = match order {
            Order::ProposePeace {
                target,
                provinces,
                tribute,
            } => (target, Proposal::Peace { provinces, tribute }),
            Order::ProposeAlliance { target } => (target, Proposal::Alliance),
            Order::DemandVassalage { target } => (target, Proposal::Vassalage),
            Order::ProposeFactionMarriage {
                target,
                character,
                spouse,
            } => (target, Proposal::Marriage { character, spouse }),
            Order::RequestPapalMediation { target } => (
                target,
                Proposal::Truce {
                    turns: sim_campaign::diplomacy::MEDIATION_TRUCE_TURNS,
                },
            ),
            Order::DeclareWar { target } => return war_verdict(state, data, &player, &target),
            _ => return verdict(true, 0, &[]),
        };
        let evaluation = evaluate(state, data, &player, &target, &proposal);
        verdict(evaluation.accept, evaluation.score, &evaluation.reasons)
    }

    /// Offers waiting for the player's answer.
    #[func]
    fn get_offers(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Some(player) = state.factions.get(state.player_faction()) else {
            return VarArray::new();
        };
        player
            .offers
            .iter()
            .map(|offer| {
                let kind = serde_json::to_value(&offer.proposal)
                    .ok()
                    .and_then(|v| v.get("kind").and_then(|k| k.as_str().map(str::to_owned)))
                    .unwrap_or_default();
                vdict! {
                    "id" => i64::from(offer.id),
                    "from" => offer.from.as_str(),
                    "from_name" => data.factions.get(&offer.from).map_or(offer.from.as_str(), |f| f.short_or_display_name()),
                    "kind" => kind.as_str(),
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
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        let Some(f) = state.factions.get(&faction) else {
            return VarDictionary::new();
        };
        let religion = faction_religion(state, data, &faction);
        let pending = f
            .offers
            .iter()
            .any(|o| matches!(o.proposal, Proposal::Obedience { .. }));
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
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
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
        let Some(state) = &self.state else {
            return PackedStringArray::new();
        };
        let player = state.player_faction().clone();
        province_ids
            .as_slice()
            .iter()
            .map(|id| {
                let key = ProvinceId::new(id.to_string())
                    .ok()
                    .and_then(|p| state.province_state(&p))
                    .map_or("", |p| {
                        if p.controller == player {
                            "self"
                        } else {
                            relation_key(state.relation(&player, &p.controller))
                        }
                    });
                GString::from(key)
            })
            .collect()
    }

    /// Answers an offer (same result shape as `submit_order`).
    #[func]
    fn answer_offer(&mut self, offer: i64, accept: bool) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return order_result(Err("aucune campagne en cours".to_owned()));
        };
        let order = Order::AnswerOffer {
            offer: offer.max(0) as u32,
            accept,
        };
        order_result(state.submit_order(data, order).map_err(|e| e.to_string()))
    }
}

fn relation_key(relation: RelationKind) -> &'static str {
    match relation {
        RelationKind::War => "war",
        RelationKind::Truce => "truce",
        RelationKind::Peace => "peace",
        RelationKind::Alliance => "alliance",
        RelationKind::Vassal => "vassal",
        RelationKind::Suzerain => "suzerain",
    }
}

fn reasons_array(reasons: &[(String, i32)]) -> VarArray {
    reasons
        .iter()
        .map(|(text, value)| {
            vdict! { "text" => text.as_str(), "value" => i64::from(*value) }.to_variant()
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

fn war_verdict(state: &CampaignState, data: &GameData, player: &FactionId, target: &FactionId) -> VarDictionary {
    let mut reasons = Vec::new();
    if state.is_at_war(player, target) {
        return verdict(false, 0, &[("Déjà en guerre".to_owned(), 0)]);
    }
    if state.has_truce(player, target) {
        reasons.push(("Trêve rompue : parjure (-40 auprès de tous)".to_owned(), -40));
    }
    match state.casus_belli(data, player, target) {
        Some(motive) => reasons.push((format!("Motif : {motive}"), 0)),
        None => reasons.push(("Sans motif : réputation -20 auprès de tous".to_owned(), -20)),
    }
    let allies: Vec<String> = state.factions[target]
        .allies
        .iter()
        .filter(|a| *a != player)
        .filter_map(|a| data.factions.get(a).map(|f| f.short_or_display_name().to_owned()))
        .collect();
    if !allies.is_empty() {
        reasons.push((format!("Alliés appelés aux armes : {}", allies.join(", ")), 0));
    }
    let score = reasons.iter().map(|(_, v)| v).sum();
    verdict(true, score, &reasons)
}

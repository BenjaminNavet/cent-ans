//! `CampaignSim` negotiation API (lot DP1, ADR 0025), in a secondary
//! `#[godot_api]` block: multi-article treaties, their evaluation by the
//! recipient (chance of acceptance), counter-proposals, what each side can
//! put on the table, and the treaty history.
//!
//! Articles travel as dictionaries shaped like `negotiation::Article`'s
//! JSON: `{kind: "cede_province", giver: "recipient", province: "prov_x"}`.
//! A treaty is sent with `submit_order({type: "propose_treaty", target,
//! articles})`.

use data_model::{FactionId, GameData, SettlementKind};
use godot::prelude::*;
use serde_json::Value;
use sim_campaign::negotiation::{
    self, counter_proposal, evaluate_treaty, is_war_goal, ruler_name, Article, TreatyEvaluation,
    COUNTER_TARGET_CHANCE,
};
use sim_campaign::{CampaignState, Season};

use crate::campaign_sim::CampaignSim;
use crate::convert::variant_to_json;

#[godot_api(secondary)]
impl CampaignSim {
    /// Verdict of `target` on `articles` proposed by the player:
    /// `{ok, chance, accept, score, blocked, articles[{kind, label, value,
    /// reasons[{text, value}]}], context[{text, value}]}`.
    #[func]
    fn evaluate_treaty(&self, target: GString, articles: VarArray) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return error_dict("aucune campagne en cours");
        };
        let (target, articles) = match parse(state, &target, &articles) {
            Ok(parsed) => parsed,
            Err(error) => return error_dict(&error),
        };
        let player = state.player_faction().clone();
        let verdict = evaluate_treaty(state, data, &player, &target, &articles);
        verdict_dict(&verdict, &articles)
    }

    /// What `target` would need to accept: `{ok, articles[...], chance}`, or
    /// `{ok: false, error}` when nothing the player holds would do.
    #[func]
    fn counter_treaty(&self, target: GString, articles: VarArray) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return error_dict("aucune campagne en cours");
        };
        let (target, articles) = match parse(state, &target, &articles) {
            Ok(parsed) => parsed,
            Err(error) => return error_dict(&error),
        };
        let player = state.player_faction().clone();
        // Q8 : a draft already acceptable has no counter-proposal; say so instead of
        // « nothing would suffice ».
        let current = evaluate_treaty(state, data, &player, &target, &articles);
        if current.blocked.is_none() && current.chance >= COUNTER_TARGET_CHANCE {
            return error_dict(&format!(
                "Rien à ajouter : ils accepteraient déjà en l'état ({} % de chances).",
                current.chance
            ));
        }
        match counter_proposal(state, data, &player, &target, &articles) {
            Some(counter) => {
                let chance = evaluate_treaty(state, data, &player, &target, &counter).chance;
                vdict! {
                    "ok" => true,
                    "articles" => &articles_array(&counter),
                    "chance" => i64::from(chance),
                }
            }
            None => error_dict("Rien de ce que vous pouvez offrir ne suffirait."),
        }
    }

    /// What each side can put on the table: `{at_war, allied, trade,
    /// access_given, access_received, war_score, ours{...}, theirs{...}}`,
    /// each side `{id, name, ruler, treasury, income, weariness,
    /// provinces[{id, name, capital, occupied, war_goal}],
    /// settlements[{id, name, province, occupied}], captives[{id, name}],
    /// hostages[{id, name}], marriageable[{id, name}]}` (`captives`: the
    /// other side's men this side holds).
    #[func]
    fn treaty_options(&self, target: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(target) = FactionId::new(target.to_string()) else {
            return VarDictionary::new();
        };
        let player = state.player_faction().clone();
        if !state.factions.contains_key(&target) || !state.factions.contains_key(&player) {
            return VarDictionary::new();
        }
        let me = &state.factions[&player];
        vdict! {
            "at_war" => state.is_at_war(&player, &target),
            "allied" => state.is_allied(&player, &target),
            "trade" => me.ledger.trade_agreements.contains(&target),
            "access_given" => me.ledger.military_access.contains(&target),
            "access_received" => state.factions[&target].ledger.military_access.contains(&player),
            "war_score" => i64::from(if state.is_at_war(&player, &target) {
                state.war_score(data, &player, &target)
            } else {
                0
            }),
            "ours" => &side_dict(state, data, &player, &target),
            "theirs" => &side_dict(state, data, &target, &player),
        }
    }

    /// Treaties signed or refused by `faction`, newest first:
    /// `[{turn, date, with, with_name, proposed, accepted, articles[], text}]`.
    #[func]
    fn get_treaty_history(&self, faction: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarArray::new();
        };
        let Some(f) = state.factions.get(&faction) else {
            return VarArray::new();
        };
        f.ledger
            .history
            .iter()
            .rev()
            .map(|record| {
                let keys: PackedStringArray = record.articles.iter().map(GString::from).collect();
                vdict! {
                    "turn" => i64::from(record.turn),
                    "date" => date_of(state, record.turn).as_str(),
                    "with" => record.with.as_str(),
                    "with_name" => faction_label(data, &record.with).as_str(),
                    "proposed" => record.proposed,
                    "accepted" => record.accepted,
                    "articles" => &keys,
                    "text" => record.text_fr.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// War between the player and `enemy`: `{war_score, weariness_ours,
    /// weariness_theirs, goals_ours[{id, name, held}], goals_theirs[...]}`.
    #[func]
    fn get_war_summary(&self, enemy: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(enemy) = FactionId::new(enemy.to_string()) else {
            return VarDictionary::new();
        };
        let player = state.player_faction().clone();
        if !state.factions.contains_key(&enemy) {
            return VarDictionary::new();
        }
        let goals = |a: &FactionId, b: &FactionId| -> VarArray {
            state.factions[a]
                .ledger
                .war_goals
                .get(b)
                .map(|list| {
                    list.iter()
                        .map(|p| {
                            vdict! {
                                "id" => p.as_str(),
                                "name" => province_label(data, p).as_str(),
                                "held" => state.controls_province(a, p),
                            }
                            .to_variant()
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        vdict! {
            "war_score" => i64::from(state.war_score(data, &player, &enemy)),
            "weariness_ours" => i64::from(negotiation::weariness(state, &player)),
            "weariness_theirs" => i64::from(negotiation::weariness(state, &enemy)),
            "goals_ours" => &goals(&player, &enemy),
            "goals_theirs" => &goals(&enemy, &player),
        }
    }
}

pub(crate) fn error_dict(message: &str) -> VarDictionary {
    vdict! { "ok" => false, "error" => message, "chance" => 0 }
}

pub(crate) fn parse(
    state: &CampaignState,
    target: &GString,
    articles: &VarArray,
) -> Result<(FactionId, Vec<Article>), String> {
    let target = FactionId::new(target.to_string()).map_err(|e| e.to_string())?;
    if !state.factions.contains_key(&target) {
        return Err(format!("faction inconnue : {target}"));
    }
    let json = variant_to_json(&articles.to_variant())?;
    let articles: Vec<Article> =
        serde_json::from_value(json).map_err(|e| format!("article invalide : {e}"))?;
    Ok((target, articles))
}

fn reasons_array(reasons: &[(String, i32)]) -> VarArray {
    reasons
        .iter()
        .map(|(text, value)| {
            vdict! { "text" => text.as_str(), "value" => i64::from(*value) }.to_variant()
        })
        .collect()
}

fn verdict_dict(verdict: &TreatyEvaluation, articles: &[Article]) -> VarDictionary {
    let values: VarArray = verdict
        .articles
        .iter()
        .zip(articles)
        .map(|(value, article)| {
            vdict! {
                "kind" => article.key(),
                "label" => value.label.as_str(),
                "value" => i64::from(value.value),
                "reasons" => &reasons_array(&value.reasons),
                "blocked" => value.blocked.as_deref().unwrap_or(""),
            }
            .to_variant()
        })
        .collect();
    vdict! {
        "ok" => true,
        "chance" => i64::from(verdict.chance),
        "accept" => verdict.accept,
        "score" => i64::from(verdict.score),
        "blocked" => verdict.blocked.as_deref().unwrap_or(""),
        "articles" => &values,
        "context" => &reasons_array(&verdict.context),
    }
}

/// JSON value → Godot variant (objects become dictionaries).
fn json_to_variant(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => n.as_i64().map_or_else(
            || n.as_f64().unwrap_or(0.0).to_variant(),
            |i| i.to_variant(),
        ),
        Value::String(s) => s.to_variant(),
        Value::Array(list) => list
            .iter()
            .map(json_to_variant)
            .collect::<VarArray>()
            .to_variant(),
        Value::Object(map) => {
            let mut dict = VarDictionary::new();
            for (key, value) in map {
                dict.set(key.as_str(), &json_to_variant(value));
            }
            dict.to_variant()
        }
    }
}

pub(crate) fn articles_array(articles: &[Article]) -> VarArray {
    articles
        .iter()
        .map(|a| json_to_variant(&serde_json::to_value(a).unwrap_or(Value::Null)))
        .collect()
}

pub(crate) fn faction_label(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

pub(crate) fn province_label(data: &GameData, id: &data_model::ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

fn date_of(state: &CampaignState, turn: u32) -> String {
    let now = Season::ALL
        .iter()
        .position(|s| *s == state.season())
        .unwrap_or(0) as i64;
    let absolute = i64::from(state.year()) * 4 + now - i64::from(state.turn()) + i64::from(turn);
    let season = Season::ALL[absolute.rem_euclid(4) as usize];
    format!("{} {}", season.label_fr(), absolute.div_euclid(4))
}

/// What `side` can offer to (or be asked by) `other`.
fn side_dict(
    state: &CampaignState,
    data: &GameData,
    side: &FactionId,
    other: &FactionId,
) -> VarDictionary {
    let f = &state.factions[side];
    let named =
        |id: &str, name: String| vdict! { "id" => id, "name" => name.as_str() }.to_variant();
    let mut owned = state.owned_provinces(side);
    owned.sort();
    let provinces: VarArray = owned
        .iter()
        .map(|p| {
            vdict! {
                "id" => p.as_str(),
                "name" => province_label(data, p).as_str(),
                "capital" => &f.capital == p,
                "occupied" => state.controls_province(other, p),
                "war_goal" => is_war_goal(state, other, side, p),
            }
            .to_variant()
        })
        .collect();
    let settlements: VarArray = state
        .settlements
        .iter()
        .filter(|(_, s)| &s.owner == side && s.kind != SettlementKind::City)
        .map(|(id, s)| {
            let name = data
                .settlements
                .get(id)
                .map_or_else(|| id.to_string(), |d| d.name.display.clone());
            vdict! {
                "id" => id.as_str(),
                "name" => name.as_str(),
                "province" => province_label(data, &s.province).as_str(),
                "occupied" => &s.controller == other,
            }
            .to_variant()
        })
        .collect();
    let captives: VarArray = state
        .characters
        .iter()
        .filter(|(_, c)| {
            c.alive && c.captive && c.captor.as_ref() == Some(side) && &c.faction == other
        })
        .map(|(id, _)| named(id.as_str(), state.character_name(data, id)))
        .collect();
    let hostages: VarArray = state
        .characters
        .iter()
        .filter(|(id, c)| {
            c.alive
                && !c.captive
                && &c.faction == side
                && c.army.is_none()
                && f.ruler.as_ref() != Some(*id)
                && state.year() - c.birth_year >= 12
        })
        .take(12)
        .map(|(id, _)| named(id.as_str(), state.character_name(data, id)))
        .collect();
    let marriageable: VarArray = state
        .characters
        .iter()
        .filter(|(_, c)| {
            c.alive
                && !c.captive
                && &c.faction == side
                && c.spouse.is_none()
                && (14..=45).contains(&(state.year() - c.birth_year))
        })
        .take(12)
        .map(|(id, c)| {
            vdict! {
                "id" => id.as_str(),
                "name" => state.character_name(data, id).as_str(),
                "female" => c.sex == data_model::Sex::Female,
            }
            .to_variant()
        })
        .collect();
    vdict! {
        "id" => side.as_str(),
        "name" => faction_label(data, side).as_str(),
        "ruler" => ruler_name(state, data, side).as_str(),
        "treasury" => f.treasury,
        "income" => f.income_last_turn,
        "weariness" => i64::from(f.ledger.weariness),
        "provinces" => &provinces,
        "settlements" => &settlements,
        "captives" => &captives,
        "hostages" => &hostages,
        "marriageable" => &marriageable,
    }
}

//! `CampaignSim` feudal API (lot FE6, spec FE § 6): feudal tree, province
//! allegiance breadcrumb, obligations, war escalation preview, objectives,
//! faction sheets, and the player's feudal orders. Every rule lives in
//! `sim_campaign::feudal`; this module only converts to Godot values.

use data_model::{FactionId, GameData, ProvinceId, TitleId, TitleRank};
use godot::prelude::*;
use sim_campaign::diplomacy::Proposal;
use sim_campaign::feudal::{self, Arbitration, FeudalNode, Likelihood};
use sim_campaign::orders::Order;
use sim_campaign::state::CampaignState;

use crate::campaign_sim::{order_result, CampaignSim};
use crate::GameDataStore;

/// Short display name of a faction (its id when unknown: factions founded
/// in play by a grant may have no data).
pub(crate) fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions.get(id).map_or_else(
        || id.as_str().to_owned(),
        |f| f.short_or_display_name().to_owned(),
    )
}

fn title_name(data: &GameData, id: &TitleId) -> String {
    data.titles
        .get(id)
        .map_or_else(|| id.as_str().to_owned(), |t| t.name.display.clone())
}

fn rank_key(rank: Option<TitleRank>) -> &'static str {
    rank.map_or("", TitleRank::key)
}

/// French label of a title rank.
fn rank_label(rank: Option<TitleRank>) -> &'static str {
    match rank {
        Some(TitleRank::Kingdom) => "royaume",
        Some(TitleRank::Duchy) => "duché",
        Some(TitleRank::County) => "comté",
        None => "",
    }
}

fn likelihood_key(likelihood: Likelihood) -> &'static str {
    match likelihood {
        Likelihood::Likely => "likely",
        Likelihood::Uncertain => "uncertain",
        Likelihood::Unlikely => "unlikely",
    }
}

fn likelihood_label(likelihood: Likelihood) -> &'static str {
    match likelihood {
        Likelihood::Likely => "probable",
        Likelihood::Uncertain => "incertain",
        Likelihood::Unlikely => "improbable",
    }
}

fn faction_id(raw: &GString) -> Option<FactionId> {
    FactionId::new(raw.to_string()).ok()
}

fn titles_array(data: &GameData, titles: &[TitleId]) -> VarArray {
    titles
        .iter()
        .map(|t| {
            let rank = data.titles.get(t).map(|d| d.rank);
            vdict! {
                "id" => t.as_str(),
                "name" => title_name(data, t).as_str(),
                "rank" => rank_key(rank),
            }
            .to_variant()
        })
        .collect()
}

fn factions_array(data: &GameData, factions: &[FactionId]) -> VarArray {
    factions
        .iter()
        .map(|f| {
            vdict! { "id" => f.as_str(), "name" => faction_name(data, f).as_str() }.to_variant()
        })
        .collect()
}

/// Faction sheet (§ 6), shared by the campaign and the faction chooser.
pub(crate) fn sheet_dict(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> VarDictionary {
    let sheet = feudal::faction_sheet(state, data, faction);
    let objectives: VarArray = sheet
        .objectives
        .iter()
        .map(|o| {
            vdict! {
                "id" => o.id.as_str(),
                "title" => o.title.as_str(),
                "description" => o.description.as_str(),
                "met" => o.met,
            }
            .to_variant()
        })
        .collect();
    let ruler = state
        .factions
        .get(faction)
        .and_then(|f| f.ruler.as_ref())
        .map_or_else(String::new, |r| state.character_name(data, r));
    let rules = &data.feudal_rules;
    let text = |t: &Option<TitleId>| t.as_ref().map_or_else(String::new, |t| title_name(data, t));
    let liege_name = sheet
        .liege
        .as_ref()
        .map_or_else(String::new, |f| faction_name(data, f));
    vdict! {
        "id" => faction.as_str(),
        "name" => faction_name(data, faction).as_str(),
        "ruler" => ruler.as_str(),
        "primary" => sheet.primary.as_ref().map_or("", |t| t.as_str()),
        "primary_name" => text(&sheet.primary).as_str(),
        "rank" => rank_key(sheet.rank),
        "rank_label" => rank_label(sheet.rank),
        "titles" => &titles_array(data, &sheet.titles),
        "liege" => sheet.liege.as_ref().map_or("", |f| f.as_str()),
        "liege_name" => liege_name.as_str(),
        "liege_chain" => &factions_array(data, &sheet.liege_chain),
        "sovereign" => sheet.sovereign.as_str(),
        "sovereign_name" => faction_name(data, &sheet.sovereign).as_str(),
        "direct_vassals" => &factions_array(data, &sheet.direct_vassals),
        "title_vassals" => &factions_array(data, &sheet.title_vassals),
        "loyalty" => sheet.loyalty.map_or(-1, i64::from),
        "status" => sheet.status.map_or("", |s| s.key()),
        "status_label" => sheet.status.map_or("", |s| s.label_fr()),
        "objectives" => &objectives,
        "independent_turns" => i64::from(sheet.streaks.independent),
        "first_vassal_turns" => i64::from(sheet.streaks.first_vassal),
        "independence_turns" => i64::from(rules.independence_turns),
        "ascension_turns" => i64::from(rules.ascension_turns),
        "start_crown" => sheet.start_crown.as_ref().map_or("", |t| t.as_str()),
        "start_crown_name" => text(&sheet.start_crown).as_str(),
        "disloyal_threshold" => i64::from(rules.disloyal_threshold),
        "alive" => state.factions.get(faction).is_some_and(|f| f.alive),
    }
}

fn node_dict(state: &CampaignState, data: &GameData, node: &FeudalNode) -> VarDictionary {
    let status = feudal::vassal_status(state, data, &node.faction);
    let loyalty = feudal::liege_of(state, data, &node.faction)
        .and_then(|_| {
            state
                .factions
                .get(&node.faction)
                .map(|f| i64::from(f.loyalty))
        })
        .unwrap_or(-1);
    let vassals: VarArray = node
        .vassals
        .iter()
        .map(|v| node_dict(state, data, v).to_variant())
        .collect();
    vdict! {
        "faction" => node.faction.as_str(),
        "name" => faction_name(data, &node.faction).as_str(),
        "rank" => rank_key(feudal::primary_rank(state, data, &node.faction)),
        "titles" => &titles_array(data, &node.titles),
        "loyalty" => loyalty,
        "status" => status.map_or("", |s| s.key()),
        "status_label" => status.map_or("", |s| s.label_fr()),
        "player" => &node.faction == state.player_faction(),
        "vassals" => &vassals,
    }
}

fn felony_reason_label(reason: feudal::FelonyReason) -> &'static str {
    match reason {
        feudal::FelonyReason::RefusedHost => "refus d'ost",
        feudal::FelonyReason::AlliedWithEnemy => "alliance avec l'ennemi du suzerain",
        feudal::FelonyReason::Revolt => "révolte",
        feudal::FelonyReason::HarbouredFelon => "asile donné à un banni",
    }
}

fn felonies_array(
    state: &CampaignState,
    data: &GameData,
    cases: &[feudal::FelonyCase],
) -> VarArray {
    cases
        .iter()
        .map(|c| {
            vdict! {
                "vassal" => c.vassal.as_str(),
                "vassal_name" => faction_name(data, &c.vassal).as_str(),
                "liege" => c.liege.as_str(),
                "liege_name" => faction_name(data, &c.liege).as_str(),
                "reason" => felony_reason_label(c.reason),
                "turns_left" => i64::from(c.expires_turn.saturating_sub(state.turn)),
            }
            .to_variant()
        })
        .collect()
}

fn arbitration_from(verdict: &str, side: &str) -> Option<Arbitration> {
    match verdict {
        "impose_peace" => Some(Arbitration::ImposePeace),
        "let_be" => Some(Arbitration::LetBe),
        "take_side" => FactionId::new(side.to_owned())
            .ok()
            .map(|side| Arbitration::TakeSide { side }),
        _ => None,
    }
}

fn opt_id(f: Option<&FactionId>) -> String {
    f.map_or_else(String::new, |f| f.as_str().to_owned())
}

impl CampaignSim {
    /// Records a feudal order of the player (same result as `submit_order`).
    fn submit_feudal(&mut self, method: &str, order: Option<Order>) -> VarDictionary {
        if self.refuse_while_turn_pending(method) {
            return order_result(Err(crate::campaign_sim_turn::TURN_PENDING_FR.to_owned()));
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return order_result(Err("aucune campagne en cours".to_owned()));
        };
        let Some(order) = order else {
            return order_result(Err("ordre féodal invalide".to_owned()));
        };
        let result = state.submit_order(data, order).map_err(|e| e.to_string());
        self.revision += 1;
        order_result(result)
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Faction sheet: `{id, name, ruler, primary, primary_name, rank,
    /// rank_label, titles [{id, name, rank}], liege, liege_name, liege_chain
    /// [{id, name}], sovereign, sovereign_name, direct_vassals, title_vassals,
    /// loyalty (-1 when sovereign), status, status_label, objectives [{id,
    /// title, description, met}], independent_turns, first_vassal_turns,
    /// independence_turns, ascension_turns, start_crown, start_crown_name,
    /// disloyal_threshold, alive}`; empty for an unknown faction.
    #[func]
    fn get_feudal_sheet(&self, faction: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        match faction_id(&faction) {
            Some(id) if state.factions.contains_key(&id) => sheet_dict(state, data, &id),
            _ => VarDictionary::new(),
        }
    }

    /// Feudal tree below `root`: `{faction, name, rank, titles, loyalty,
    /// status, status_label, player, vassals [...]}` recursively.
    #[func]
    fn get_feudal_tree(&self, root: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        match faction_id(&root) {
            Some(id) if state.factions.contains_key(&id) => {
                node_dict(state, data, &feudal::feudal_tree(state, data, &id))
            }
            _ => VarDictionary::new(),
        }
    }

    /// Titles above `province`, the highest first: `[{title, title_name,
    /// rank, holder, holder_name}]` (« Royaume de France › Duché de
    /// Bourgogne › Comté de Charolais »).
    #[func]
    fn get_province_breadcrumb(&self, province: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(id) = ProvinceId::new(province.to_string()) else {
            return VarArray::new();
        };
        feudal::province_breadcrumb(state, data, &id)
            .iter()
            .map(|link| {
                let holder_name = link
                    .holder
                    .as_ref()
                    .map_or_else(String::new, |f| faction_name(data, f));
                vdict! {
                    "title" => link.title.as_str(),
                    "title_name" => title_name(data, &link.title).as_str(),
                    "rank" => rank_key(data.titles.get(&link.title).map(|t| t.rank)),
                    "holder" => opt_id(link.holder.as_ref()).as_str(),
                    "holder_name" => holder_name.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Allegiance chain of `province` (holder first, then its lords).
    #[func]
    fn get_province_lieges(&self, province: GString) -> PackedStringArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return PackedStringArray::new();
        };
        let Ok(id) = ProvinceId::new(province.to_string()) else {
            return PackedStringArray::new();
        };
        feudal::province_lieges(state, data, &id)
            .iter()
            .map(|f| GString::from(f.as_str()))
            .collect()
    }

    /// Obligations of `faction` (§ 4.1): `{liege, liege_name, tribute,
    /// tribute_percent, host_against [{id, name}], protect [{id, name,
    /// attackers [{id, name}]}], felons [{vassal, vassal_name, liege,
    /// liege_name, reason, turns_left}], own_felonies [...], grantable_titles
    /// [{id, name, rank}], grant_candidates [{id, name}], homage_candidates
    /// [{id, name}]}`.
    #[func]
    fn get_feudal_obligations(&self, faction: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(id) = faction_id(&faction).filter(|f| state.factions.contains_key(f)) else {
            return VarDictionary::new();
        };
        let o = feudal::obligations(state, data, &id);
        let protect: VarArray = o
            .protect
            .iter()
            .map(|p| {
                vdict! {
                    "id" => p.vassal.as_str(),
                    "name" => faction_name(data, &p.vassal).as_str(),
                    "attackers" => &factions_array(data, &p.attackers),
                }
                .to_variant()
            })
            .collect();
        let liege_name = o
            .liege
            .as_ref()
            .map_or_else(String::new, |f| faction_name(data, f));
        vdict! {
            "liege" => opt_id(o.liege.as_ref()).as_str(),
            "liege_name" => liege_name.as_str(),
            "tribute" => o.tribute,
            "tribute_percent" => o.tribute_percent,
            "host_against" => &factions_array(data, &o.host_against),
            "protect" => &protect,
            "felons" => &felonies_array(state, data, &o.felons),
            "own_felonies" => &felonies_array(state, data, &o.own_felonies),
            "grantable_titles" => &titles_array(data, &o.grantable_titles),
            "grant_candidates" => &factions_array(data, &feudal::direct_vassals(state, data, &id)),
            "homage_candidates" => &factions_array(data, &feudal::homage_candidates(state, data, &id)),
        }
    }

    /// Who may enter the war if `attacker` attacks `target` (§ 4.3):
    /// `[{faction, name, likelihood ("likely", "uncertain", "unlikely"),
    /// likelihood_label, reason}]`, link by link.
    #[func]
    fn get_war_escalation_preview(&self, attacker: GString, target: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let (Some(attacker), Some(target)) = (faction_id(&attacker), faction_id(&target)) else {
            return VarArray::new();
        };
        if !state.factions.contains_key(&attacker) || !state.factions.contains_key(&target) {
            return VarArray::new();
        }
        feudal::war_escalation_preview(state, data, &attacker, &target)
            .iter()
            .map(|step| {
                vdict! {
                    "faction" => step.faction.as_str(),
                    "name" => faction_name(data, &step.faction).as_str(),
                    "likelihood" => likelihood_key(step.likelihood),
                    "likelihood_label" => likelihood_label(step.likelihood),
                    "reason" => step.reason.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Feudal colouring of each id of `province_ids`, in order:
    /// `{sovereign, holder, second_lord}` ("" when none); empty for an
    /// unknown id.
    #[func]
    fn get_feudal_map(&self, province_ids: PackedStringArray) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let cells = feudal::feudal_map(state, data);
        province_ids
            .as_slice()
            .iter()
            .map(|id| {
                let Some(cell) = ProvinceId::new(id.to_string())
                    .ok()
                    .and_then(|p| cells.get(&p))
                else {
                    return VarDictionary::new().to_variant();
                };
                vdict! {
                    "sovereign" => opt_id(cell.sovereign.as_ref()).as_str(),
                    "holder" => opt_id(cell.holder.as_ref()).as_str(),
                    "second_lord" => opt_id(cell.second_lord.as_ref()).as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Feudal calls waiting for the player (§ 4.3): `[{id, kind
    /// ("protection", "arbitration", "summons" — ADR 0146), from, from_name, aggressor,
    /// aggressor_name, attacker, attacker_name, target, target_name, text,
    /// expires_in}]`.
    #[func]
    fn get_feudal_offers(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Some(player) = state.factions.get(state.player_faction()) else {
            return VarArray::new();
        };
        let name_of = |f: Option<&FactionId>| f.map_or_else(String::new, |f| faction_name(data, f));
        player
            .offers
            .iter()
            .filter_map(|offer| {
                let (kind, aggressor, attacker, target) = match &offer.proposal {
                    Proposal::Protection { aggressor } => {
                        ("protection", Some(aggressor), None, None)
                    }
                    Proposal::Arbitration { attacker, target } => {
                        ("arbitration", None, Some(attacker), Some(target))
                    }
                    Proposal::PeaceSummons { target } => ("summons", None, None, Some(target)),
                    _ => return None,
                };
                let expires_in = offer
                    .expires_turn
                    .saturating_sub(state.turn())
                    .saturating_sub(1);
                Some(
                    vdict! {
                        "id" => i64::from(offer.id),
                        "kind" => kind,
                        "from" => offer.from.as_str(),
                        "from_name" => name_of(Some(&offer.from)).as_str(),
                        "aggressor" => opt_id(aggressor).as_str(),
                        "aggressor_name" => name_of(aggressor).as_str(),
                        "attacker" => opt_id(attacker).as_str(),
                        "attacker_name" => name_of(attacker).as_str(),
                        "target" => opt_id(target).as_str(),
                        "target_name" => name_of(target).as_str(),
                        "text" => offer.text_fr.as_str(),
                        "expires_in" => i64::from(expires_in),
                    }
                    .to_variant(),
                )
            })
            .collect()
    }

    /// The player declares forfeiture against its felon vassal (§ 4.4).
    #[func]
    fn feudal_declare_commise(&mut self, vassal: GString) -> VarDictionary {
        let order = faction_id(&vassal).map(|vassal| Order::DeclareCommise { vassal });
        self.submit_feudal("feudal_declare_commise", order)
    }

    /// The player grants `title` (not its primary one) to `grantee`.
    #[func]
    fn feudal_grant_title(&mut self, title: GString, grantee: GString) -> VarDictionary {
        let order = TitleId::new(title.to_string())
            .ok()
            .zip(faction_id(&grantee))
            .map(|(title, grantee)| Order::GrantTitle { title, grantee });
        self.submit_feudal("feudal_grant_title", order)
    }

    /// The player revolts against its direct suzerain (§ 4.2).
    #[func]
    fn feudal_revolt(&mut self) -> VarDictionary {
        self.submit_feudal("feudal_revolt", Some(Order::Revolt))
    }

    /// The player pays homage to `lord` (§ 4.2).
    #[func]
    fn feudal_switch_allegiance(&mut self, lord: GString) -> VarDictionary {
        let order = faction_id(&lord).map(|lord| Order::SwitchAllegiance { lord });
        self.submit_feudal("feudal_switch_allegiance", order)
    }

    /// The player's verdict on a private war offer (§ 4.3.5): `verdict` is
    /// "impose_peace", "take_side" (with `side`) or "let_be".
    #[func]
    fn feudal_arbitrate(&mut self, offer: i64, verdict: GString, side: GString) -> VarDictionary {
        let order = arbitration_from(&verdict.to_string(), &side.to_string()).map(|verdict| {
            Order::ArbitratePrivateWar {
                offer: offer.max(0) as u32,
                verdict,
            }
        });
        self.submit_feudal("feudal_arbitrate", order)
    }
}

#[godot_api(secondary)]
impl GameDataStore {
    /// Spring 1337 sheets of the playable factions (faction chooser, § 6),
    /// sovereigns first then by name: same shape as
    /// `CampaignSim.get_feudal_sheet`, plus `provinces` (ids owned in 1337)
    /// and `kingdom` / `kingdom_name` (the sovereign's primary title).
    #[func]
    fn get_feudal_start_sheets(&self) -> VarArray {
        let Some(data) = &self.data else {
            return VarArray::new();
        };
        let Some(first) = data.factions.keys().next() else {
            return VarArray::new();
        };
        let mut state = match CampaignState::new_1337(data, first.clone(), 0) {
            Ok(state) => state,
            Err(error) => {
                godot_error!("GameDataStore.get_feudal_start_sheets: {error}");
                return VarArray::new();
            }
        };
        feudal::sync_suzerains(&mut state, data);
        let mut playable: Vec<&FactionId> = data
            .factions
            .iter()
            .filter(|(_, f)| f.playable)
            .map(|(id, _)| id)
            .collect();
        playable.sort_by_cached_key(|id| {
            (
                feudal::liege_of(&state, data, id).is_some(),
                faction_name(data, id),
            )
        });
        playable
            .into_iter()
            .map(|id| {
                let mut sheet = sheet_dict(&state, data, id);
                let provinces: PackedStringArray = data
                    .provinces
                    .values()
                    .filter(|p| &p.owner == id)
                    .map(|p| GString::from(p.id.as_str()))
                    .collect();
                sheet.set("provinces", &provinces);
                let sovereign = feudal::liege_chain(&state, data, id)
                    .last()
                    .cloned()
                    .unwrap_or_else(|| id.clone());
                let kingdom = state.feudal.primary.get(&sovereign).cloned();
                sheet.set("kingdom", kingdom.as_ref().map_or("", |t| t.as_str()));
                sheet.set(
                    "kingdom_name",
                    kingdom
                        .as_ref()
                        .map_or_else(String::new, |t| title_name(data, t))
                        .as_str(),
                );
                sheet.to_variant()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitration_verdicts_parse() {
        assert_eq!(
            arbitration_from("impose_peace", ""),
            Some(Arbitration::ImposePeace)
        );
        assert_eq!(arbitration_from("let_be", ""), Some(Arbitration::LetBe));
        assert_eq!(
            arbitration_from("take_side", "fac_blois"),
            Some(Arbitration::TakeSide {
                side: FactionId::new("fac_blois").unwrap()
            })
        );
        assert_eq!(arbitration_from("take_side", ""), None);
        assert_eq!(arbitration_from("nonsense", ""), None);
    }
}

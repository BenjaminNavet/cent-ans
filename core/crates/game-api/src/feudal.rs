//! Feudal panel views (ADR 0357): the faction's titles, suzerain chain, vassals, obligations,
//! objectives, possessions (provinces under each title), the feudal tree, the feudal calls
//! waiting for the player, and the player's feudal orders.
//!
//! Port of `godot-bridge` `campaign_sim_feudal.rs`. No rule here: everything comes from
//! `sim_campaign::feudal`.

use data_model::{FactionId, GameData, TitleId, TitleRank};
use serde::Serialize;
use sim_campaign::feudal::{self, Arbitration, FeudalNode};
use sim_campaign::negotiation::Article;
use sim_campaign::{CampaignState, Order};

use crate::common::{faction_refs, FactionRef};
pub use crate::diplomacy::OrderResult;

/// A held title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TitleView {
    pub id: String,
    pub name: String,
    /// `kingdom`, `duchy`, `county` or empty.
    pub rank: String,
    /// French rank label (`royaume`, `duché`, `comté`).
    pub rank_label: String,
    /// Provinces of the title's own domain: `{id, name}`.
    pub provinces: Vec<FactionRef>,
    /// Title the player may grant (held by a vassal, not primary): see `Obligations`.
    pub primary: bool,
}

/// One objective with its status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectiveRow {
    pub title: String,
    pub description: String,
    pub met: bool,
}

/// `{vassal, reason, turns_left}` felony case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FelonyRow {
    pub vassal: FactionRef,
    pub liege: FactionRef,
    pub reason: String,
    pub turns_left: u32,
}

/// A vassal owed protection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtectionRow {
    pub vassal: FactionRef,
    pub attackers: Vec<FactionRef>,
}

/// Duties of the faction (§ 4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DutiesView {
    pub tribute: i64,
    pub tribute_percent: i64,
    pub host_against: Vec<FactionRef>,
    pub protect: Vec<ProtectionRow>,
    pub felons: Vec<FelonyRow>,
    pub own_felonies: Vec<FelonyRow>,
    pub grantable_titles: Vec<TitleRef>,
    pub homage_candidates: Vec<FactionRef>,
}

/// `{id, name, rank}` reference to a title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TitleRef {
    pub id: String,
    pub name: String,
    pub rank: String,
}

/// Counters of the generic victory ways (§ 4.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StreaksView {
    pub independent_turns: u32,
    pub independence_turns: u32,
    pub first_vassal_turns: u32,
    pub ascension_turns: u32,
    pub start_crown_name: String,
}

/// Sheet of one faction (the « Féodalité » section and the head of the panel).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeudalSheet {
    pub id: String,
    pub name: String,
    pub ruler: String,
    /// Display name of the ruler's house (for the coat of arms), empty when unknown.
    pub house: String,
    pub primary_name: String,
    pub rank: String,
    pub rank_label: String,
    pub titles: Vec<TitleView>,
    pub liege: Option<FactionRef>,
    pub liege_chain: Vec<FactionRef>,
    pub sovereign: FactionRef,
    pub direct_vassals: Vec<FactionRef>,
    pub title_vassals: Vec<FactionRef>,
    pub loyalty: Option<u8>,
    /// `loyal`, `discontent`, `felon`, `in_revolt` or empty.
    pub status: String,
    pub status_label: String,
    pub objectives: Vec<ObjectiveRow>,
    pub streaks: StreaksView,
}

/// A node of the feudal tree.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TreeNodeView {
    pub id: String,
    pub name: String,
    pub rank: String,
    pub titles: Vec<String>,
    /// Loyalty towards its suzerain, `None` for a sovereign.
    pub loyalty: Option<u8>,
    pub status: String,
    pub status_label: String,
    pub player: bool,
    pub vassals: Vec<TreeNodeView>,
}

/// A feudal call waiting for the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeudalCall {
    pub id: u32,
    /// `protection`, `arbitration` or `peace_summons`.
    pub kind: String,
    pub text: String,
    pub expires_in: u32,
    pub aggressor: Option<FactionRef>,
    pub attacker: Option<FactionRef>,
    pub target: Option<FactionRef>,
}

/// Everything the feudal panel shows, seen from `focus` (the player by default).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeudalPanel {
    pub focus: FeudalSheet,
    /// Duties of the player (not of `focus`).
    pub duties: DutiesView,
    /// Tree below the sovereign of `focus`.
    pub tree: TreeNodeView,
    pub calls: Vec<FeudalCall>,
}

fn rank_key(rank: Option<TitleRank>) -> &'static str {
    rank.map_or("", TitleRank::key)
}

fn rank_label(rank: Option<TitleRank>) -> &'static str {
    match rank {
        Some(TitleRank::Kingdom) => "royaume",
        Some(TitleRank::Duchy) => "duché",
        Some(TitleRank::County) => "comté",
        None => "",
    }
}

fn title_name(data: &GameData, id: &TitleId) -> String {
    data.titles
        .get(id)
        .map_or_else(|| id.as_str().to_owned(), |t| t.name.display.clone())
}

fn title_ref(data: &GameData, id: &TitleId) -> TitleRef {
    TitleRef {
        id: id.as_str().to_owned(),
        name: title_name(data, id),
        rank: rank_key(data.titles.get(id).map(|t| t.rank)).to_owned(),
    }
}

fn faction_ref(data: &GameData, id: &FactionId) -> FactionRef {
    FactionRef {
        id: id.as_str().to_owned(),
        name: data.faction_name(id),
    }
}

fn felony_rows(
    state: &CampaignState,
    data: &GameData,
    cases: &[feudal::FelonyCase],
) -> Vec<FelonyRow> {
    cases
        .iter()
        .map(|c| FelonyRow {
            vassal: faction_ref(data, &c.vassal),
            liege: faction_ref(data, &c.liege),
            reason: felony_reason_label(c.reason).to_owned(),
            turns_left: c.expires_turn.saturating_sub(state.turn),
        })
        .collect()
}

fn felony_reason_label(reason: feudal::FelonyReason) -> &'static str {
    match reason {
        feudal::FelonyReason::RefusedHost => "refus d'ost",
        feudal::FelonyReason::AlliedWithEnemy => "alliance avec l'ennemi du suzerain",
        feudal::FelonyReason::Revolt => "révolte",
        feudal::FelonyReason::HarbouredFelon => "asile donné à un banni",
    }
}

/// Sheet of `faction`, `None` for an unknown faction.
pub fn feudal_sheet(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Option<FeudalSheet> {
    let live = state.factions.get(faction)?;
    let sheet = feudal::faction_sheet(state, data, faction);
    let ruler_id = live.ruler.clone();
    let ruler = ruler_id
        .as_ref()
        .map_or_else(String::new, |r| state.character_name(data, r));
    let house = ruler_id
        .as_ref()
        .and_then(|r| state.characters.get(r))
        .map_or_else(String::new, |c| c.house.clone());
    let rules = &data.feudal_rules;
    let titles = sheet
        .titles
        .iter()
        .map(|t| TitleView {
            id: t.as_str().to_owned(),
            name: title_name(data, t),
            rank: rank_key(data.titles.get(t).map(|d| d.rank)).to_owned(),
            rank_label: rank_label(data.titles.get(t).map(|d| d.rank)).to_owned(),
            provinces: data.titles.get(t).map_or_else(Vec::new, |d| {
                d.de_jure_provinces
                    .iter()
                    .map(|p| FactionRef {
                        id: p.as_str().to_owned(),
                        name: data.province_name(p),
                    })
                    .collect()
            }),
            primary: sheet.primary.as_ref() == Some(t),
        })
        .collect();
    Some(FeudalSheet {
        id: faction.as_str().to_owned(),
        name: data.faction_name(faction),
        ruler,
        house,
        primary_name: sheet
            .primary
            .as_ref()
            .map_or_else(String::new, |t| title_name(data, t)),
        rank: rank_key(sheet.rank).to_owned(),
        rank_label: rank_label(sheet.rank).to_owned(),
        titles,
        liege: sheet.liege.as_ref().map(|f| faction_ref(data, f)),
        liege_chain: faction_refs(data, &sheet.liege_chain),
        sovereign: faction_ref(data, &sheet.sovereign),
        direct_vassals: faction_refs(data, &sheet.direct_vassals),
        title_vassals: faction_refs(data, &sheet.title_vassals),
        loyalty: sheet.loyalty,
        status: sheet.status.map_or("", |s| s.key()).to_owned(),
        status_label: sheet.status.map_or("", |s| s.label_fr()).to_owned(),
        objectives: sheet
            .objectives
            .iter()
            .map(|o| ObjectiveRow {
                title: o.title.clone(),
                description: o.description.clone(),
                met: o.met,
            })
            .collect(),
        streaks: StreaksView {
            independent_turns: sheet.streaks.independent,
            independence_turns: rules.independence_turns,
            first_vassal_turns: sheet.streaks.first_vassal,
            ascension_turns: rules.ascension_turns,
            start_crown_name: sheet
                .start_crown
                .as_ref()
                .map_or_else(String::new, |t| title_name(data, t)),
        },
    })
}

/// Duties of `faction` (§ 4.1).
pub fn duties(data: &GameData, state: &CampaignState, faction: &FactionId) -> DutiesView {
    let o = feudal::obligations(state, data, faction);
    DutiesView {
        tribute: o.tribute,
        tribute_percent: o.tribute_percent,
        host_against: faction_refs(data, &o.host_against),
        protect: o
            .protect
            .iter()
            .map(|p| ProtectionRow {
                vassal: faction_ref(data, &p.vassal),
                attackers: faction_refs(data, &p.attackers),
            })
            .collect(),
        felons: felony_rows(state, data, &o.felons),
        own_felonies: felony_rows(state, data, &o.own_felonies),
        grantable_titles: o
            .grantable_titles
            .iter()
            .map(|t| title_ref(data, t))
            .collect(),
        homage_candidates: faction_refs(data, &feudal::homage_candidates(state, data, faction)),
    }
}

fn node_view(state: &CampaignState, data: &GameData, node: &FeudalNode) -> TreeNodeView {
    let status = feudal::vassal_status(state, data, &node.faction);
    let loyalty = feudal::liege_of(state, data, &node.faction)
        .and_then(|_| state.factions.get(&node.faction).map(|f| f.loyalty));
    TreeNodeView {
        id: node.faction.as_str().to_owned(),
        name: data.faction_name(&node.faction),
        rank: rank_key(feudal::primary_rank(state, data, &node.faction)).to_owned(),
        titles: node.titles.iter().map(|t| title_name(data, t)).collect(),
        loyalty,
        status: status.map_or("", |s| s.key()).to_owned(),
        status_label: status.map_or("", |s| s.label_fr()).to_owned(),
        player: &node.faction == state.player_faction(),
        vassals: node
            .vassals
            .iter()
            .map(|v| node_view(state, data, v))
            .collect(),
    }
}

/// Feudal tree below `root`.
pub fn feudal_tree(
    data: &GameData,
    state: &CampaignState,
    root: &FactionId,
) -> Option<TreeNodeView> {
    state
        .factions
        .contains_key(root)
        .then(|| node_view(state, data, &feudal::feudal_tree(state, data, root)))
}

/// Feudal calls waiting for the player (protection, private-war arbitration, peace summons).
pub fn calls(data: &GameData, state: &CampaignState) -> Vec<FeudalCall> {
    let Some(player) = state.factions.get(state.player_faction()) else {
        return Vec::new();
    };
    let reference = |f: &FactionId| Some(faction_ref(data, f));
    player
        .offers
        .iter()
        .filter_map(|offer| {
            let (kind, aggressor, attacker, target) = match offer.proposal.articles.as_slice() {
                [Article::Protection { aggressor }] => {
                    ("protection", reference(aggressor), None, None)
                }
                [Article::Arbitration { attacker, target }] => {
                    ("arbitration", None, reference(attacker), reference(target))
                }
                [Article::PeaceSummons { target }] => {
                    ("peace_summons", None, None, reference(target))
                }
                _ => return None,
            };
            Some(FeudalCall {
                id: offer.id,
                kind: kind.to_owned(),
                text: offer.text_fr.clone(),
                expires_in: offer
                    .expires_turn
                    .saturating_sub(state.turn())
                    .saturating_sub(1),
                aggressor,
                attacker,
                target,
            })
        })
        .collect()
}

/// The whole panel, seen from `focus` (the faction whose position is shown; usually the player).
pub fn feudal_panel(
    data: &GameData,
    state: &CampaignState,
    focus: &FactionId,
) -> Option<FeudalPanel> {
    let sheet = feudal_sheet(data, state, focus)?;
    let root = FactionId::new(sheet.sovereign.id.clone()).ok()?;
    let tree = feudal_tree(data, state, &root)?;
    Some(FeudalPanel {
        focus: sheet,
        duties: duties(data, state, state.player_faction()),
        tree,
        calls: calls(data, state),
    })
}

fn send(data: &GameData, state: &mut CampaignState, order: Option<Order>) -> OrderResult {
    match order {
        Some(order) => crate::diplomacy::submit_order(data, state, order),
        None => OrderResult {
            ok: false,
            message: "ordre féodal invalide".to_owned(),
        },
    }
}

/// The player declares forfeiture against its felon vassal (§ 4.4).
pub fn declare_commise(data: &GameData, state: &mut CampaignState, vassal: &str) -> OrderResult {
    let order = FactionId::new(vassal.to_owned())
        .ok()
        .map(|vassal| Order::DeclareCommise { vassal });
    send(data, state, order)
}

/// The player grants `title` (not its primary one) to `grantee`.
pub fn grant_title(
    data: &GameData,
    state: &mut CampaignState,
    title: &str,
    grantee: &str,
) -> OrderResult {
    let order = TitleId::new(title.to_owned())
        .ok()
        .zip(FactionId::new(grantee.to_owned()).ok())
        .map(|(title, grantee)| Order::GrantTitle { title, grantee });
    send(data, state, order)
}

/// The player revolts against its direct suzerain (§ 4.2).
pub fn revolt(data: &GameData, state: &mut CampaignState) -> OrderResult {
    send(data, state, Some(Order::Revolt))
}

/// The player pays homage to `lord` (§ 4.2).
pub fn switch_allegiance(data: &GameData, state: &mut CampaignState, lord: &str) -> OrderResult {
    let order = FactionId::new(lord.to_owned())
        .ok()
        .map(|lord| Order::SwitchAllegiance { lord });
    send(data, state, order)
}

/// Verdict on a private-war call: `impose_peace`, `take_side` (with `side`) or `let_be`.
pub fn arbitrate(
    data: &GameData,
    state: &mut CampaignState,
    offer: u32,
    verdict: &str,
    side: &str,
) -> OrderResult {
    let verdict = match verdict {
        "impose_peace" => Some(Arbitration::ImposePeace),
        "let_be" => Some(Arbitration::LetBe),
        "take_side" => FactionId::new(side.to_owned())
            .ok()
            .map(|side| Arbitration::TakeSide { side }),
        _ => None,
    };
    send(
        data,
        state,
        verdict.map(|verdict| Order::ArbitratePrivateWar { offer, verdict }),
    )
}

/// Answers a protection call or peace summons (`answer_offer`).
pub fn answer_call(
    data: &GameData,
    state: &mut CampaignState,
    offer: u32,
    accept: bool,
) -> OrderResult {
    crate::diplomacy::answer_offer(data, state, offer, accept)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(faction: &str) -> (&'static GameData, CampaignState, FactionId) {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, faction, 1337);
        feudal::sync_suzerains(&mut state, data);
        (data, state, FactionId::new(faction).unwrap())
    }

    #[test]
    fn france_sees_its_vassals_and_english_double_allegiance() {
        let (data, state, france) = setup("fac_france");
        let sheet = feudal_sheet(data, &state, &france).expect("france");
        assert!(sheet.liege.is_none());
        assert_eq!(sheet.sovereign.id, "fac_france");
        assert!(sheet.direct_vassals.len() + sheet.title_vassals.len() > 0);
        assert!(sheet.titles.iter().any(|t| t.primary));
        assert!(!sheet.titles[0].provinces.is_empty());
        let panel = feudal_panel(data, &state, &france).expect("panel");
        assert_eq!(panel.tree.id, "fac_france");
        assert!(!panel.tree.vassals.is_empty());
        assert!(serde_json::to_string(&panel).is_ok());
    }

    #[test]
    fn a_vassal_panel_is_rooted_at_its_sovereign() {
        let (data, state, france) = setup("fac_france");
        let sheet = feudal_sheet(data, &state, &france).expect("france");
        let vassal = sheet.direct_vassals.first().expect("a direct vassal");
        let id = FactionId::new(vassal.id.clone()).unwrap();
        let panel = feudal_panel(data, &state, &id).expect("panel");
        assert_eq!(panel.focus.liege.as_ref().unwrap().id, "fac_france");
        assert_eq!(panel.tree.id, "fac_france");
        assert!(panel.focus.loyalty.is_some());
        assert!(!panel.focus.status.is_empty());
    }

    #[test]
    fn france_has_a_ruler_house_and_objectives() {
        let (data, state, france) = setup("fac_france");
        let sheet = feudal_sheet(data, &state, &france).expect("sheet");
        assert!(!sheet.ruler.is_empty());
        assert!(!sheet.house.is_empty());
        assert!(sheet.loyalty.is_none());
        assert!(sheet.streaks.independence_turns > 0);
        let nowhere = FactionId::new("fac_nowhere").unwrap();
        assert!(feudal_sheet(data, &state, &nowhere).is_none());
        assert!(feudal_tree(data, &state, &nowhere).is_none());
        assert!(feudal_panel(data, &state, &nowhere).is_none());
    }

    #[test]
    fn a_vassal_player_sees_its_duties_and_can_revolt() {
        let (data, state, france) = setup("fac_france");
        let sheet = feudal_sheet(data, &state, &france).expect("france");
        let vassal = sheet
            .direct_vassals
            .first()
            .expect("a direct vassal")
            .id
            .clone();
        let (data, mut state, vassal) = setup(&vassal);
        let d = duties(data, &state, &vassal);
        assert!(d.tribute_percent > 0);
        let panel = feudal_panel(data, &state, &vassal).expect("panel");
        assert!(panel.focus.liege.is_some());
        let result = revolt(data, &mut state);
        assert!(result.ok, "{}", result.message);
    }

    #[test]
    fn invalid_orders_are_refused_in_french() {
        let (data, mut state, _) = setup("fac_france");
        assert!(!declare_commise(data, &mut state, "fac_nowhere").ok);
        assert!(!grant_title(data, &mut state, "tit_nowhere", "fac_france").ok);
        assert!(!arbitrate(data, &mut state, 0, "bogus", "").ok);
        assert!(!switch_allegiance(data, &mut state, "").message.is_empty());
        assert!(calls(data, &state).is_empty());
    }
}

//! Read-only views of the feudal state for the interface (lot FE6, spec
//! § 6): vassal status badges, faction sheets, obligations, the province
//! breadcrumb and the feudal map filter. Everything is deduced from the
//! title holdings; nothing here changes the state.

use data_model::key_enum;
use std::collections::BTreeMap;

use data_model::{FactionId, GameData, ProvinceId, TitleId, TitleRank};

use super::{
    direct_vassals, effective_liege, holder_of, liege_chain, liege_of, objective_status,
    primary_rank, province_lieges, title_of_province, title_vassals, titles_of, FelonyCase,
    VictoryStreaks, MAX_DEPTH,
};
use crate::state::CampaignState;

key_enum! {
/// Badge of a vassal in the feudal tree (§ 6: loyal, discontent, felon,
/// in revolt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VassalStatus {
    Loyal => "loyal",
    /// Loyalty below `feudal.json:disloyal_threshold`: may refuse the host,
    /// revolt or pay homage elsewhere (§ 4.2).
    Discontent => "discontent",
    /// Under an open felony case: its suzerain may declare forfeiture.
    Felon => "felon",
    /// At war with its own suzerain (forfeiture war, revolt under way).
    InRevolt => "in_revolt",
}
}

impl VassalStatus {
    /// French label.
    pub fn label_fr(self) -> &'static str {
        match self {
            VassalStatus::Loyal => "loyal",
            VassalStatus::Discontent => "mécontent",
            VassalStatus::Felon => "félon",
            VassalStatus::InRevolt => "en révolte",
        }
    }
}

/// Status of `vassal` towards its direct suzerain; `None` when sovereign.
pub fn vassal_status(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
) -> Option<VassalStatus> {
    let liege = liege_of(state, data, vassal)?;
    if state.is_at_war(vassal, &liege) {
        return Some(VassalStatus::InRevolt);
    }
    if state
        .feudal
        .felonies
        .iter()
        .any(|c| &c.vassal == vassal && c.liege == liege && c.expires_turn > state.turn)
    {
        return Some(VassalStatus::Felon);
    }
    let loyalty = state.factions.get(vassal).map_or(100, |f| f.loyalty);
    Some(if loyalty < data.feudal_rules.disloyal_threshold {
        VassalStatus::Discontent
    } else {
        VassalStatus::Loyal
    })
}

/// One historical objective with its texts and status (§ 4.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub met: bool,
}

/// Everything the faction sheet, the faction chooser and the tree show of
/// a faction.
#[derive(Debug, Clone, PartialEq)]
pub struct FactionSheet {
    pub faction: FactionId,
    pub primary: Option<TitleId>,
    pub rank: Option<TitleRank>,
    /// Titles held, primary first.
    pub titles: Vec<TitleId>,
    /// Direct suzerain.
    pub liege: Option<FactionId>,
    /// Suzerains, direct first, up to the sovereign.
    pub liege_chain: Vec<FactionId>,
    /// Top of the chain (the faction itself when sovereign).
    pub sovereign: FactionId,
    pub direct_vassals: Vec<FactionId>,
    /// Factions holding a title below one of ours without being direct
    /// vassals (double allegiance: England for France through Guyenne).
    pub title_vassals: Vec<FactionId>,
    /// Loyalty towards the direct suzerain (0-100), when there is one.
    pub loyalty: Option<u8>,
    pub status: Option<VassalStatus>,
    pub objectives: Vec<ObjectiveView>,
    pub streaks: VictoryStreaks,
    /// Crown of the generic victory (§ 4.8), for the factions that started as vassals.
    pub start_crown: Option<TitleId>,
}

/// Sheet of `faction` in the current state.
pub fn faction_sheet(state: &CampaignState, data: &GameData, faction: &FactionId) -> FactionSheet {
    let liege = liege_of(state, data, faction);
    let chain = liege_chain(state, data, faction);
    let sovereign = chain.last().cloned().unwrap_or_else(|| faction.clone());
    let direct = direct_vassals(state, data, faction);
    let title_only = title_vassals(state, data, faction)
        .into_iter()
        .filter(|v| !direct.contains(v))
        .collect();
    let objectives = objective_status(state, data, faction)
        .into_iter()
        .map(|progress| {
            let found = state
                .feudal
                .primary
                .get(faction)
                .and_then(|t| data.titles.get(t))
                .and_then(|t| t.objectives.iter().find(|o| o.id == progress.objective));
            ObjectiveView {
                title: found.map_or_else(|| progress.objective.clone(), |o| o.title.clone()),
                description: found.map_or_else(String::new, |o| o.description.clone()),
                id: progress.objective,
                met: progress.met,
            }
        })
        .collect();
    FactionSheet {
        faction: faction.clone(),
        primary: state.feudal.primary.get(faction).cloned(),
        rank: primary_rank(state, data, faction),
        titles: titles_of(state, faction),
        loyalty: liege
            .as_ref()
            .and_then(|_| state.factions.get(faction).map(|f| f.loyalty)),
        status: vassal_status(state, data, faction),
        liege,
        liege_chain: chain,
        sovereign,
        direct_vassals: direct,
        title_vassals: title_only,
        objectives,
        streaks: state
            .feudal
            .streaks
            .get(faction)
            .copied()
            .unwrap_or_default(),
        start_crown: state.feudal.start_crowns.get(faction).cloned(),
    }
}

/// A direct vassal under attack, owed protection (§ 4.1, 4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectionDue {
    pub vassal: FactionId,
    /// Its enemies other than ourselves.
    pub attackers: Vec<FactionId>,
}

/// Obligations of a faction (§ 4.1), shown in the faction panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligations {
    pub liege: Option<FactionId>,
    /// Tribute paid to the direct suzerain this turn.
    pub tribute: i64,
    /// Share of the income owed (`feudal.json:vassal_tribute_percent`).
    pub tribute_percent: i64,
    /// Enemies of the direct suzerain: the host is owed against them.
    pub host_against: Vec<FactionId>,
    /// Direct vassals at war, owed protection.
    pub protect: Vec<ProtectionDue>,
    /// Felony cases against our own vassals: forfeiture may be declared.
    pub felons: Vec<FelonyCase>,
    /// Felony cases opened against us.
    pub own_felonies: Vec<FelonyCase>,
    /// Our vassals holding titles we may still grant (non-primary titles).
    pub grantable_titles: Vec<TitleId>,
}

/// Obligations of `faction` in the current state.
pub fn obligations(state: &CampaignState, data: &GameData, faction: &FactionId) -> Obligations {
    let liege = liege_of(state, data, faction);
    let tribute = super::tribute_due(state, data, faction).map_or(0, |(_, amount)| amount);
    let host_against = liege
        .as_ref()
        .and_then(|l| state.factions.get(l))
        .map(|l| {
            l.at_war_with
                .iter()
                .filter(|enemy| *enemy != faction)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let protect = direct_vassals(state, data, faction)
        .into_iter()
        .filter_map(|vassal| {
            let attackers: Vec<FactionId> = state
                .factions
                .get(&vassal)?
                .at_war_with
                .iter()
                .filter(|enemy| *enemy != faction)
                .cloned()
                .collect();
            (!attackers.is_empty()).then_some(ProtectionDue { vassal, attackers })
        })
        .collect();
    let open = |c: &&FelonyCase| c.expires_turn > state.turn;
    let felons = state
        .feudal
        .felonies
        .iter()
        .filter(open)
        .filter(|c| &c.liege == faction && !super::has_forfeiture(state, faction, &c.vassal))
        .cloned()
        .collect();
    let own_felonies = state
        .feudal
        .felonies
        .iter()
        .filter(open)
        .filter(|c| &c.vassal == faction)
        .cloned()
        .collect();
    let primary = state.feudal.primary.get(faction);
    let grantable_titles = titles_of(state, faction)
        .into_iter()
        .filter(|t| Some(t) != primary)
        .collect();
    Obligations {
        liege,
        tribute,
        tribute_percent: data.feudal_rules.vassal_tribute_percent,
        host_against,
        protect,
        felons,
        own_felonies,
        grantable_titles,
    }
}

/// Lords `faction` may pay homage to (§ 4.2): living factions whose
/// primary title outranks its own, at peace with it, other than its
/// current suzerain and its own vassals. Strongest first.
pub fn homage_candidates(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<FactionId> {
    let Some(own) = primary_rank(state, data, faction) else {
        return Vec::new();
    };
    let liege = liege_of(state, data, faction);
    let mut lords: Vec<FactionId> = state
        .feudal
        .primary
        .keys()
        .filter(|f| *f != faction && Some(*f) != liege.as_ref())
        .filter(|f| state.factions.get(*f).is_some_and(|s| s.alive))
        .filter(|f| primary_rank(state, data, f).is_some_and(|r| r > own))
        .filter(|f| !state.is_at_war(faction, f))
        .filter(|f| !liege_chain(state, data, f).contains(faction))
        .cloned()
        .collect();
    lords.sort_by(|a, b| {
        state
            .faction_power(b)
            .total_cmp(&state.faction_power(a))
            .then_with(|| a.cmp(b))
    });
    lords
}

/// One link of a province's breadcrumb: a title and its holder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleLink {
    pub title: TitleId,
    pub holder: Option<FactionId>,
}

/// Titles above `province`, the highest first (« Royaume de France ›
/// Duché de Bourgogne › Comté de Charolais »), following the in-play
/// allegiances.
pub fn province_breadcrumb(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
) -> Vec<TitleLink> {
    let mut links = Vec::new();
    let mut title = title_of_province(data, province);
    for _ in 0..MAX_DEPTH {
        let Some(id) = title else { break };
        links.push(TitleLink {
            title: id.clone(),
            holder: holder_of(state, id).cloned(),
        });
        title = effective_liege(state, data, id);
    }
    links.reverse();
    links
}

/// Feudal colouring of a province (§ 6 map filter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeudalCell {
    /// Sovereign at the top of the province's chain (background colour).
    pub sovereign: Option<FactionId>,
    /// Holder of the province's own title (hatching when not the sovereign).
    pub holder: Option<FactionId>,
    /// Lord the holder answers to for this province when it differs from
    /// the holder's own suzerain (double allegiance: party shield).
    pub second_lord: Option<FactionId>,
}

/// Feudal colouring of every province with a title.
pub fn feudal_map(state: &CampaignState, data: &GameData) -> BTreeMap<ProvinceId, FeudalCell> {
    let mut lieges: BTreeMap<FactionId, Option<FactionId>> = BTreeMap::new();
    data.provinces
        .keys()
        .map(|province| {
            let chain = province_lieges(state, data, province);
            let holder = chain.first().cloned();
            let second_lord = holder.as_ref().and_then(|h| {
                let lord = chain.get(1)?;
                let own = lieges
                    .entry(h.clone())
                    .or_insert_with(|| liege_of(state, data, h));
                (own.as_ref() != Some(lord)).then(|| lord.clone())
            });
            (
                province.clone(),
                FeudalCell {
                    sovereign: chain.last().cloned(),
                    holder,
                    second_lord,
                },
            )
        })
        .collect()
}

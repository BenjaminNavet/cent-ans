//! Title transfers (lot F3, spec § 4.5-4.7): moving a holding with its
//! provinces, personal unions (a faction left without title vanishes into
//! the new holder), factions created by a grant to a courtier, vacant
//! titles freeing their vassals, titles demanded after a victory.

use std::collections::BTreeSet;

use data_model::{CharacterId, FactionId, GameData, TitleId, TitleRank};

use super::{direct_vassals, holder_of, titles_of, FeudalError};
use crate::events::{EventKind, GameEvent};
use crate::state::{CampaignState, FactionState};

/// Rank of `title` (`None` for an unknown id).
pub(super) fn rank_of(data: &GameData, title: &TitleId) -> Option<TitleRank> {
    data.titles.get(title).map(|t| t.rank)
}

fn title_name(data: &GameData, title: &TitleId) -> String {
    data.titles
        .get(title)
        .map_or_else(|| title.to_string(), |t| t.name.display.clone())
}

/// Highest-ranked title held by `faction`; the current primary wins ties.
fn best_primary(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<TitleId> {
    let current = state.feudal.primary.get(faction).cloned();
    let held: Vec<&TitleId> = state
        .feudal
        .holders
        .iter()
        .filter(|(_, h)| *h == faction)
        .map(|(t, _)| t)
        .collect();
    let current_rank = current
        .as_ref()
        .filter(|t| holder_of(state, t) == Some(faction))
        .and_then(|t| rank_of(data, t));
    let best = held
        .iter()
        .filter_map(|t| rank_of(data, t).map(|r| (r, t)))
        .max_by(|(ra, ta), (rb, tb)| ra.cmp(rb).then_with(|| tb.cmp(ta)))
        .map(|(r, t)| (r, (*t).clone()));
    match (current_rank, best) {
        (Some(rank), Some((best_rank, _))) if rank >= best_rank => current,
        (_, Some((_, best))) => Some(best),
        (_, None) => None,
    }
}

/// Recomputes the primary title of `faction` from what it holds.
fn refresh_primary(state: &mut CampaignState, data: &GameData, faction: &FactionId) {
    match best_primary(state, data, faction) {
        Some(title) => {
            state.feudal.primary.insert(faction.clone(), title);
        }
        None => {
            state.feudal.primary.remove(faction);
        }
    }
    // Every holding change goes through here: keep the cached suzerains
    // (`FactionState::suzerain`) in step with the titles.
    super::sync_suzerains(state, data);
}

/// Keeps the capital of `faction` among the provinces it owns.
fn fix_capital(state: &mut CampaignState, faction: &FactionId) {
    let Some(capital) = state.factions.get(faction).map(|f| f.capital.clone()) else {
        return;
    };
    if state.province_owner(&capital) == Some(faction) {
        return;
    }
    if let Some(first) = state.owned_provinces(faction).into_iter().next() {
        state.factions.get_mut(faction).expect("exists").capital = first;
    }
}

fn is_rebels(faction: &FactionId) -> bool {
    faction.as_str() == crate::diplomacy::REBELS_FACTION
}

/// Moves `title` to `to` (see [`super::transfer_title`]).
pub(crate) fn transfer(
    state: &mut CampaignState,
    data: &GameData,
    title: &TitleId,
    to: &FactionId,
    events: &mut Vec<GameEvent>,
) -> Result<(), FeudalError> {
    let Some(definition) = data.titles.get(title) else {
        return Err(FeudalError::UnknownTitle(title.clone()));
    };
    if !state.factions.get(to).is_some_and(|f| f.alive) {
        return Err(FeudalError::DeadFaction(to.clone()));
    }
    let from = holder_of(state, title).cloned();
    if from.as_ref() == Some(to) {
        return Ok(());
    }
    state.feudal.holders.insert(title.clone(), to.clone());
    if let Some(from) = &from {
        for province in &definition.de_jure_provinces {
            if state.province_owner(province) == Some(from) {
                crate::ransom::cede_province(state, from, to, province);
            }
        }
    }
    refresh_primary(state, data, to);
    fix_capital(state, to);
    events.push(
        GameEvent::new(
            EventKind::Vassalage,
            format!(
                "{} passe à {}.",
                title_name(data, title),
                crate::diplomacy::faction_name(data, to)
            ),
        )
        .faction(to),
    );
    let Some(from) = from else {
        return Ok(());
    };
    refresh_primary(state, data, &from);
    let alive = state.factions.get(&from).is_some_and(|f| f.alive);
    if !alive {
        return Ok(());
    }
    if titles_of(state, &from).is_empty() && !is_rebels(&from) {
        absorb(state, data, &from, to, events);
    } else {
        fix_capital(state, &from);
    }
    Ok(())
}

/// A faction left without any title vanishes into `into` (spec § 4.5
/// « fusion », § 2 « disparition d'une faction sans titre »): its lands,
/// occupations, armies, court and treasury pass to `into`.
fn absorb(
    state: &mut CampaignState,
    data: &GameData,
    from: &FactionId,
    into: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    for province in state.owned_provinces(from) {
        crate::ransom::cede_province(state, from, into, &province);
    }
    for settlement in state.settlements.values_mut() {
        if &settlement.owner == from {
            settlement.owner = into.clone();
        }
        if &settlement.controller == from {
            settlement.controller = into.clone();
        }
    }
    for army in state.armies.values_mut() {
        if &army.faction == from {
            army.faction = into.clone();
        }
    }
    for character in state.characters.values_mut() {
        if character.alive && &character.faction == from {
            character.faction = into.clone();
        }
    }
    let treasury = std::mem::take(&mut state.factions.get_mut(from).expect("alive").treasury);
    state.factions.get_mut(into).expect("alive").treasury += treasury;
    {
        let f = state.factions.get_mut(from).expect("alive");
        f.ruler = None;
        f.heir = None;
    }
    forget_faction(state, from);
    crate::characters::dissolve_faction(state, from);
    fix_capital(state, into);
    events.push(
        GameEvent::new(
            EventKind::FactionDestroyed,
            format!(
                "{} n'a plus de titre : ses terres sont réunies à celles de {}.",
                crate::diplomacy::faction_name(data, from),
                crate::diplomacy::faction_name(data, into)
            ),
        )
        .faction(from),
    );
}

/// Drops the feudal records of a vanished faction.
fn forget_faction(state: &mut CampaignState, faction: &FactionId) {
    state.feudal.primary.remove(faction);
    state
        .feudal
        .felonies
        .retain(|c| &c.vassal != faction && &c.liege != faction);
    state
        .feudal
        .forfeitures
        .retain(|f| &f.vassal != faction && &f.liege != faction);
    state.feudal.streaks.remove(faction);
}

/// `title` loses its holder (§ 4.7): its direct vassals become independent
/// (see [`super::liege_of`]).
pub fn vacate_title(state: &mut CampaignState, data: &GameData, title: &TitleId) {
    let Some(holder) = state.feudal.holders.remove(title) else {
        return;
    };
    refresh_primary(state, data, &holder);
}

/// Titles of a faction destroyed without heir (no land, no army): each
/// escheats to the holder of the title above when it is another living
/// faction, otherwise it falls vacant and frees its vassals (§ 4.5, § 4.7).
pub fn on_faction_destroyed(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let titles = titles_of(state, faction);
    for title in titles {
        // Effective liege: an homage or a release overrides the data.
        let liege = super::effective_liege(state, data, &title)
            .and_then(|l| holder_of(state, l))
            .filter(|h| *h != faction && state.factions.get(*h).is_some_and(|f| f.alive))
            .cloned();
        match liege {
            Some(liege) => {
                let _ = transfer(state, data, &title, &liege, events);
            }
            None => {
                vacate_title(state, data, &title);
                events.push(
                    GameEvent::new(
                        EventKind::Vassalage,
                        format!(
                            "{} est vacant : ses vassaux deviennent indépendants.",
                            title_name(data, &title)
                        ),
                    )
                    .faction(faction),
                );
            }
        }
    }
    forget_faction(state, faction);
}

/// Receiver of a granted title (§ 4.4 « concède à un fidèle »).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grantee {
    /// A living faction, usually a direct vassal.
    Faction(FactionId),
    /// A character of the grantor's court, who founds a new vassal faction.
    Character(CharacterId),
}

/// `grantor` grants one of its titles (not its primary one) to `grantee`.
/// Returns the faction now holding it (a new one for a courtier).
pub fn grant_title(
    state: &mut CampaignState,
    data: &GameData,
    grantor: &FactionId,
    title: &TitleId,
    grantee: Grantee,
) -> Result<FactionId, FeudalError> {
    if !data.titles.contains_key(title) {
        return Err(FeudalError::UnknownTitle(title.clone()));
    }
    if holder_of(state, title) != Some(grantor) {
        return Err(FeudalError::NotHolder(grantor.clone(), title.clone()));
    }
    if state.feudal.primary.get(grantor) == Some(title) {
        return Err(FeudalError::PrimaryTitle(title.clone()));
    }
    let to = match grantee {
        Grantee::Faction(faction) => {
            if &faction == grantor || !state.factions.get(&faction).is_some_and(|f| f.alive) {
                return Err(FeudalError::DeadFaction(faction));
            }
            faction
        }
        Grantee::Character(character) => {
            let valid = state
                .characters
                .get(&character)
                .is_some_and(|c| c.alive && !c.captive && &c.faction == grantor)
                && state.factions[grantor].ruler.as_ref() != Some(&character);
            if !valid {
                return Err(FeudalError::InvalidGrantee(character));
            }
            create_faction(state, data, grantor, &character, title)
        }
    };
    let mut events = Vec::new();
    transfer(state, data, title, &to, &mut events)?;
    super::record_title_grant(state, data, grantor, &to);
    for event in events {
        state.push_order_event(event);
    }
    Ok(to)
}

/// Id of the faction founded for `title`: a dead faction of the data whose
/// primary title it is (revived, so it keeps its name and arms), otherwise
/// `fac_<title>` made unique.
fn new_faction_id(state: &CampaignState, data: &GameData, title: &TitleId) -> FactionId {
    let revived = data.factions.iter().find(|(id, f)| {
        f.primary_title.as_ref() == Some(title) && state.factions.get(*id).is_none_or(|s| !s.alive)
    });
    if let Some((id, _)) = revived {
        return id.clone();
    }
    let stem = title.as_str().trim_start_matches("tit_");
    let taken: BTreeSet<&str> = state.factions.keys().map(FactionId::as_str).collect();
    let mut candidate = format!("fac_{stem}");
    let mut n = 2;
    while taken.contains(candidate.as_str())
        || data.factions.keys().any(|f| f.as_str() == candidate)
    {
        candidate = format!("fac_{stem}_{n}");
        n += 1;
    }
    FactionId::new(candidate).expect("built from a valid title id")
}

/// Founds a vassal faction of `grantor` ruled by `character` (spec § 4.4
/// « création d'une faction quand un titre est concédé à un personnage sans
/// faction »). The holding itself is moved by the caller.
fn create_faction(
    state: &mut CampaignState,
    data: &GameData,
    grantor: &FactionId,
    character: &CharacterId,
    title: &TitleId,
) -> FactionId {
    let id = new_faction_id(state, data, title);
    let template = state.factions[grantor].clone();
    let capital = data
        .titles
        .get(title)
        .and_then(|t| t.de_jure_provinces.first().cloned())
        .unwrap_or_else(|| template.capital.clone());
    // ADR 0114: the feudal tie stands for an alliance.
    let allies = BTreeSet::new();
    let faction = FactionState {
        treasury: 0,
        income_last_turn: 0,
        upkeep_last_turn: 0,
        at_war_with: BTreeSet::new(),
        allies,
        truces: Default::default(),
        alive: true,
        ruler: Some(character.clone()),
        heir: None,
        capital,
        technologies: template.technologies.clone(),
        tax_rate: template.tax_rate,
        goods: Default::default(),
        army_upkeep_last_turn: 0,
        building_upkeep_last_turn: 0,
        deficit_seasons: 0,
        projected_income: 0,
        regency: false,
        embargoes: BTreeSet::new(),
        suzerain: Some(grantor.clone()),
        loyalty: (data.feudal_rules.loyalty.base + data.feudal_rules.loyalty.title_granted)
            .clamp(0, 100) as u8,
        claims: Vec::new(),
        modifiers: Vec::new(),
        war_scores: Default::default(),
        war_started: Default::default(),
        religion: template.religion.clone(),
        papal_favor: template.papal_favor,
        excommunicated_until: None,
        offers: Vec::new(),
        last_offer_turn: Default::default(),
        last_war_declared: None,
        research: None,
        research_progress: 0,
        research_points_last_turn: 0,
        research_banked: Default::default(),
        table_upkeep_last_turn: 0,
        coinage: template.coinage,
        price_level: template.price_level,
        coinage_changed_year: None,
        seigniorage_last_turn: 0,
        recoinage_last_turn: 0,
        ransom_debts: Vec::new(),
        chivalric_order: None,
        trade_income_last_turn: 0,
        budget_history: Vec::new(),
        ledger: Default::default(),
    };
    state.factions.insert(id.clone(), faction);
    state.detach_general(character);
    let spouse = state
        .characters
        .get(character)
        .and_then(|c| c.spouse.clone());
    for member in std::iter::once(character.clone()).chain(spouse) {
        let ruler_of_grantor = state.factions[grantor].ruler.as_ref() == Some(&member);
        if let Some(c) = state.characters.get_mut(&member) {
            if c.alive && &c.faction == grantor && !ruler_of_grantor {
                c.faction = id.clone();
                c.governor_of = None;
            }
        }
    }
    if state.factions[grantor].heir.as_ref() == Some(character) {
        let ruler = state.factions[grantor].ruler.clone();
        let heir = ruler.and_then(|r| crate::dynasty::pick_heir_by_law(state, data, grantor, &r));
        state.factions.get_mut(grantor).expect("alive").heir = heir;
    }
    let heir = crate::dynasty::pick_heir_by_law(state, data, &id, character);
    state.factions.get_mut(&id).expect("created").heir = heir;
    state.push_order_event(
        GameEvent::new(
            EventKind::Vassalage,
            format!(
                "{} reçoit {} de {} et fonde sa propre maison vassale.",
                state.character_name(data, character),
                title_name(data, title),
                crate::diplomacy::faction_name(data, grantor)
            ),
        )
        .faction(&id),
    );
    id
}

/// What became of a title demanded after a victory (§ 4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleDemandOutcome {
    /// The winner took it: the title ranks at least as high as its own.
    Usurped,
    /// A lesser title, granted to the winner's strongest direct vassal.
    Granted(FactionId),
}

/// The winner of a war takes `title` (§ 4.6): it usurps a title of rank
/// equal to or above its primary title, and grants a lesser one to its
/// strongest direct vassal (or keeps it, without vassal). Only a vassal
/// whose primary title ranks at least as high is eligible: a lesser vassal
/// would take the conquered title as its primary one and pass under that
/// title's liege (Albret given Normandy would become a vassal of France).
pub fn conquer_title(
    state: &mut CampaignState,
    data: &GameData,
    winner: &FactionId,
    title: &TitleId,
) -> Result<TitleDemandOutcome, FeudalError> {
    let Some(rank) = rank_of(data, title) else {
        return Err(FeudalError::UnknownTitle(title.clone()));
    };
    let own_rank = state
        .feudal
        .primary
        .get(winner)
        .and_then(|t| rank_of(data, t));
    let vassal = if own_rank.is_some_and(|own| rank < own) {
        direct_vassals(state, data, winner)
            .into_iter()
            .filter(|v| holder_of(state, title) != Some(v))
            .filter(|v| {
                state
                    .feudal
                    .primary
                    .get(v)
                    .and_then(|t| rank_of(data, t))
                    .is_some_and(|own| own >= rank)
            })
            .max_by(|a, b| {
                state
                    .faction_power(a)
                    .total_cmp(&state.faction_power(b))
                    .then_with(|| b.cmp(a))
            })
    } else {
        None
    };
    let to = vassal.clone().unwrap_or_else(|| winner.clone());
    let mut events = Vec::new();
    transfer(state, data, title, &to, &mut events)?;
    for event in events {
        state.push_order_event(event);
    }
    Ok(match vassal {
        Some(v) => TitleDemandOutcome::Granted(v),
        None => TitleDemandOutcome::Usurped,
    })
}

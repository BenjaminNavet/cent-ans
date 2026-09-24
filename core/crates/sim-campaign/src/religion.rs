//! Religion (M5 spec § 2.4): papal favour, excommunication, papal mediation,
//! the Great Western Schism (dates from `data/religions`) and heresies.

use data_model::{BuildingCategory, FactionId, GameData, ProvinceId, ReligionId, ReligionKind};

use crate::diplomacy::{
    faction_name, DiplomacyError, Proposal, MEDIATION_COST, MEDIATION_MIN_FAVOR,
    MEDIATION_TRUCE_TURNS, PAPACY_FACTION,
};
use crate::events::{EventKind, GameEvent};
use crate::state::{CampaignState, Season};

/// Excommunication length (10 years).
pub const EXCOMMUNICATION_TURNS: u32 = 40;
/// Papal favour above which an excommunication is lifted early.
pub const EXCOMMUNICATION_LIFT_FAVOR: u8 = 40;
/// Heresy share above which a province may rise.
pub const HERESY_REVOLT_THRESHOLD: u8 = 70;
/// Heresy share above which it spreads to neighbours.
pub const HERESY_SPREAD_THRESHOLD: u8 = 30;

/// How two faiths relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaithRelation {
    Same,
    /// Two Catholic obediences during the Schism.
    RivalObedience,
    Different,
}

/// Current religion of a faction (state, or the static data before M5 saves).
pub fn faction_religion(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<ReligionId> {
    state
        .factions
        .get(faction)
        .and_then(|f| f.religion.clone())
        .or_else(|| data.factions.get(faction).map(|f| f.religion.clone()))
}

/// Root of a church or obedience (heresies and other faiths are their own root).
fn orthodox_root(data: &GameData, religion: &ReligionId) -> Option<ReligionId> {
    let r = data.religions.get(religion)?;
    match r.kind {
        ReligionKind::Church => Some(religion.clone()),
        ReligionKind::Obedience => r.parent.clone().or_else(|| Some(religion.clone())),
        ReligionKind::Heresy | ReligionKind::OtherFaith => None,
    }
}

/// Relation between two religions.
pub fn religions_relation(data: &GameData, a: &ReligionId, b: &ReligionId) -> FaithRelation {
    if a == b {
        return FaithRelation::Same;
    }
    match (orthodox_root(data, a), orthodox_root(data, b)) {
        (Some(ra), Some(rb)) if ra == rb => FaithRelation::RivalObedience,
        _ => FaithRelation::Different,
    }
}

/// `true` when `a` and `b` belong to the same faith (obediences included).
pub fn same_faith(data: &GameData, a: &ReligionId, b: &ReligionId) -> bool {
    religions_relation(data, a, b) != FaithRelation::Different
}

pub fn faith_relation(
    state: &CampaignState,
    data: &GameData,
    a: &FactionId,
    b: &FactionId,
) -> FaithRelation {
    match (
        faction_religion(state, data, a),
        faction_religion(state, data, b),
    ) {
        (Some(ra), Some(rb)) => religions_relation(data, &ra, &rb),
        _ => FaithRelation::Different,
    }
}

/// `true` for a faction of the Catholic church (any obedience).
pub fn is_catholic(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    faction_religion(state, data, faction)
        .and_then(|r| orthodox_root(data, &r))
        .is_some()
}

pub fn is_excommunicated(state: &CampaignState, faction: &FactionId) -> bool {
    state
        .factions
        .get(faction)
        .and_then(|f| f.excommunicated_until)
        .is_some_and(|until| until > state.turn)
}

pub(crate) fn change_favor(state: &mut CampaignState, faction: &FactionId, delta: i32) {
    if let Some(f) = state.factions.get_mut(faction) {
        f.papal_favor = (i32::from(f.papal_favor) + delta).clamp(0, 100) as u8;
    }
}

pub(crate) fn excommunicate(state: &mut CampaignState, data: &GameData, faction: &FactionId) {
    let until = state.turn + EXCOMMUNICATION_TURNS;
    if let Some(f) = state.factions.get_mut(faction) {
        f.excommunicated_until = Some(until);
    }
    let vassals: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.suzerain.as_ref() == Some(faction))
        .map(|(id, _)| id.clone())
        .collect();
    for vassal in vassals {
        let v = state.factions.get_mut(&vassal).expect("exists");
        v.loyalty = v.loyalty.saturating_sub(20);
    }
    let text = format!(
        "Le pape excommunie le souverain de {} : l'interdit frappe le royaume.",
        faction_name(data, faction)
    );
    state.push_order_event(GameEvent::new(EventKind::Excommunication, text).faction(faction));
}

/// Great Schism: `faction` joins the obedience `religion`.
pub(crate) fn set_obedience(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    religion: &ReligionId,
) -> Result<(), DiplomacyError> {
    if !state.schism {
        return Err(DiplomacyError::NoSchism);
    }
    let valid = data
        .religions
        .get(religion)
        .is_some_and(|r| matches!(r.kind, ReligionKind::Church | ReligionKind::Obedience));
    if !valid || !is_catholic(state, data, faction) {
        return Err(DiplomacyError::InvalidObedience);
    }
    state.factions.get_mut(faction).expect("exists").religion = Some(religion.clone());
    let text = format!(
        "{} se range derrière {}.",
        faction_name(data, faction),
        religion_display(state, data, religion)
    );
    state.push_order_event(GameEvent::new(EventKind::Schism, text).faction(faction));
    Ok(())
}

/// Display name of a religion, noting the Avignon obedience during the Schism.
pub fn religion_display(state: &CampaignState, data: &GameData, religion: &ReligionId) -> String {
    let name = data
        .religions
        .get(religion)
        .map_or_else(|| religion.to_string(), |r| r.name.display.clone());
    let is_church = data
        .religions
        .get(religion)
        .is_some_and(|r| r.kind == ReligionKind::Church);
    if state.schism && is_church {
        format!("{name} (obédience d'Avignon)")
    } else {
        name
    }
}

impl CampaignState {
    /// `donate_to_church { amount }`: favour +1 per 200 livres.
    pub fn donate_to_church(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        amount: i64,
    ) -> Result<(), DiplomacyError> {
        if amount <= 0 {
            return Err(DiplomacyError::InvalidAmount);
        }
        if !is_catholic(self, data, faction) {
            return Err(DiplomacyError::NotCatholic);
        }
        if self
            .factions
            .get(faction)
            .is_none_or(|f| f.treasury < amount)
        {
            return Err(DiplomacyError::InsufficientFunds);
        }
        self.factions.get_mut(faction).expect("checked").treasury -= amount;
        if let Some(papacy) = FactionId::new(PAPACY_FACTION)
            .ok()
            .and_then(|p| self.factions.get_mut(&p))
        {
            papacy.treasury += amount;
        }
        change_favor(self, faction, ((amount / 200) as i32).max(1));
        Ok(())
    }

    /// `request_papal_mediation { target }`: the Pope proposes a two-year truce.
    pub fn request_papal_mediation(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        target: &FactionId,
    ) -> Result<(), DiplomacyError> {
        if !is_catholic(self, data, faction) {
            return Err(DiplomacyError::NotCatholic);
        }
        let Some(me) = self.factions.get(faction) else {
            return Err(DiplomacyError::UnknownFaction(faction.clone()));
        };
        if me.papal_favor < MEDIATION_MIN_FAVOR || is_excommunicated(self, faction) {
            return Err(DiplomacyError::PapalFavorTooLow);
        }
        if me.treasury < MEDIATION_COST {
            return Err(DiplomacyError::InsufficientFunds);
        }
        self.propose(
            data,
            faction,
            target,
            Proposal::Truce {
                turns: MEDIATION_TRUCE_TURNS,
            },
        )?;
        self.factions.get_mut(faction).expect("checked").treasury -= MEDIATION_COST;
        if let Some(papacy) = FactionId::new(PAPACY_FACTION)
            .ok()
            .and_then(|p| self.factions.get_mut(&p))
        {
            papacy.treasury += MEDIATION_COST;
        }
        Ok(())
    }

    /// Extra unrest (flat) of a province from politics and faith: regency,
    /// excommunication, heresy, embargo.
    pub fn political_unrest(&self, province: &ProvinceId) -> f64 {
        let Some(p) = self.provinces.get(province) else {
            return 0.0;
        };
        let Some(controller) = self.province_controller(province) else {
            return 0.0;
        };
        let mut unrest = 0.0;
        if let Some(f) = self.factions.get(controller) {
            if f.regency {
                unrest += f64::from(crate::dynasty::REGENCY_UNREST_PENALTY);
            }
            if is_excommunicated(self, controller) {
                unrest += 10.0;
            }
        }
        let embargoed = self
            .factions
            .iter()
            .any(|(id, f)| id != controller && f.alive && f.embargoes.contains(controller));
        if embargoed {
            unrest += 3.0;
        }
        unrest += f64::from(p.heresy) / 5.0;
        unrest
    }
}

fn religious_buildings(data: &GameData, buildings: &[data_model::BuildingId]) -> u32 {
    buildings
        .iter()
        .filter(|b| {
            data.buildings
                .get(*b)
                .is_some_and(|d| d.category == BuildingCategory::Religious)
        })
        .count() as u32
}

/// G1: a character's piety as the rules read it — the stored value plus the
/// `Piety` effects of its traits and skills (pious +10, lustful −5,
/// excommunicated −20…), clamped to 0-100. The stored value only moves with
/// events, the table and the court's religious buildings.
pub fn effective_piety(
    state: &CampaignState,
    data: &GameData,
    character: &data_model::CharacterId,
) -> u8 {
    let Some(c) = state.characters.get(character) else {
        return 50;
    };
    let bonus = crate::skills::character_effects(state, data, character)
        .piety
        .apply(0.0);
    (f64::from(c.piety) + bonus).round().clamp(0.0, 100.0) as u8
}

fn ruler_piety(state: &CampaignState, data: &GameData, faction: &FactionId) -> u8 {
    state
        .factions
        .get(faction)
        .and_then(|f| f.ruler.as_ref())
        .filter(|r| state.characters.contains_key(*r))
        .map_or(50, |r| effective_piety(state, data, r))
}

/// Phase: papal favour, excommunications, the Schism, heresies.
pub(crate) fn resolve_religion(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    resolve_schism(state, data, events);
    resolve_favor(state, data, events);
    resolve_heresy(state, data, events);
}

fn schism_religion(data: &GameData) -> Option<(&ReligionId, i32, i32)> {
    data.religions
        .iter()
        .filter(|(_, r)| r.kind == ReligionKind::Obedience)
        .find_map(|(id, r)| {
            let from = r.available_from.as_ref()?.year()?;
            let until = r.available_until.as_ref()?.year()?;
            Some((id, from, until))
        })
}

fn resolve_schism(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let Some((rival, from, until)) = schism_religion(data) else {
        return;
    };
    let rival = rival.clone();
    let church = data.religions.get(&rival).and_then(|r| r.parent.clone());
    let Some(church) = church else {
        return;
    };
    let year = state.year;
    let autumn_or_later = matches!(state.season, Season::Autumn | Season::Winter);
    if !state.schism && year >= from && year < until && (year > from || autumn_or_later) {
        state.schism = true;
        let adherents = &data.religions[&rival].historical_adherents;
        let catholics: Vec<FactionId> = state
            .factions
            .keys()
            .filter(|f| is_catholic(state, data, f))
            .cloned()
            .collect();
        for faction in &catholics {
            if adherents.contains(faction) {
                state.factions.get_mut(faction).expect("exists").religion = Some(rival.clone());
            }
        }
        events.push(GameEvent::new(
            EventKind::Schism,
            "Grand Schisme d'Occident : deux papes se disputent la chrétienté, l'un à Rome, l'autre à Avignon.".to_owned(),
        ));
        let player = state.player_faction.clone();
        if catholics.contains(&player) {
            let current = faction_religion(state, data, &player);
            let other = if current.as_ref() == Some(&rival) {
                church.clone()
            } else {
                rival.clone()
            };
            if let Ok(papacy) = FactionId::new(PAPACY_FACTION) {
                state.create_obedience_offer(data, &papacy, other);
            }
        }
    } else if state.schism && year >= until && autumn_or_later {
        state.schism = false;
        let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
        for faction in ids {
            let on_rival = state.factions[&faction].religion.as_ref() == Some(&rival);
            if on_rival {
                state.factions.get_mut(&faction).expect("exists").religion = Some(church.clone());
            }
        }
        events.push(GameEvent::new(
            EventKind::Schism,
            "Le concile de Constance élit Martin V : fin du Grand Schisme, l'Église est réunifiée."
                .to_owned(),
        ));
    }
}

impl CampaignState {
    fn create_obedience_offer(&mut self, data: &GameData, from: &FactionId, religion: ReligionId) {
        let player = self.player_faction.clone();
        let id = self.next_offer_id;
        self.next_offer_id += 1;
        let text = format!(
            "Grand Schisme : rejoindre {} ?",
            data.religions
                .get(&religion)
                .map_or_else(|| religion.to_string(), |r| r.name.display.clone())
        );
        let expires_turn = self.turn + 5;
        if let Some(f) = self.factions.get_mut(&player) {
            f.offers.push(crate::diplomacy::Offer {
                id,
                from: from.clone(),
                proposal: Proposal::Obedience { religion },
                expires_turn,
                text_fr: text,
            });
        }
    }
}

fn resolve_favor(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let papacy = FactionId::new(PAPACY_FACTION).ok();
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for faction in ids {
        if !state.factions[&faction].alive || !is_catholic(state, data, &faction) {
            continue;
        }
        let buildings: u32 = state
            .settlements
            .values()
            .filter(|s| s.controller == faction)
            .map(|s| religious_buildings(data, &s.buildings))
            .sum();
        let mut target =
            40 + i32::from(ruler_piety(state, data, &faction)) / 4 + (buildings as i32 * 2).min(20);
        if papacy
            .as_ref()
            .is_some_and(|p| state.is_at_war(&faction, p))
        {
            target -= 30;
        }
        if is_excommunicated(state, &faction) {
            target -= 20;
        }
        let target = target.clamp(0, 100) as u8;
        let f = state.factions.get_mut(&faction).expect("exists");
        f.papal_favor = if f.papal_favor < target {
            f.papal_favor.saturating_add(2).min(target)
        } else {
            f.papal_favor.saturating_sub(2).max(target)
        };
        let lift = f
            .excommunicated_until
            .is_some_and(|until| until <= state.turn || f.papal_favor > EXCOMMUNICATION_LIFT_FAVOR);
        if lift {
            f.excommunicated_until = None;
            events.push(
                GameEvent::new(
                    EventKind::Excommunication,
                    format!(
                        "Le pape lève l'excommunication du souverain de {}.",
                        faction_name(data, &faction)
                    ),
                )
                .faction(&faction),
            );
        }
    }
}

fn resolve_heresy(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let year = state.year;
    // Appearance at the historical date.
    for (id, religion) in &data.religions {
        if religion.kind != ReligionKind::Heresy {
            continue;
        }
        let Some(from) = religion.available_from.as_ref().and_then(|d| d.year()) else {
            continue;
        };
        // Seeded during its historical year only: once stamped out, it does
        // not come back on its own (it can still spread from neighbours).
        if year != from {
            continue;
        }
        for province in &religion.origin_provinces {
            let Some(p) = state.provinces.get_mut(province) else {
                continue;
            };
            if p.heresy_religion.is_none() && p.heresy == 0 {
                p.heresy = 10;
                p.heresy_religion = Some(id.clone());
                let province_name = data
                    .provinces
                    .get(province)
                    .map_or_else(|| province.to_string(), |d| d.name.display.clone());
                events.push(
                    GameEvent::new(
                        EventKind::Heresy,
                        format!("{} apparaissent à {province_name}.", religion.name.display),
                    )
                    .province(province),
                );
            }
        }
    }
    // Growth, decline, spread, revolt.
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    let mut seeds: Vec<(ProvinceId, ReligionId)> = Vec::new();
    for id in &ids {
        let Some(heresy_religion) = state.provinces[id].heresy_religion.clone() else {
            continue;
        };
        let p = &state.provinces[id];
        let Some(controller) = state.province_controller(id).cloned() else {
            continue;
        };
        let province_buildings = state.province_buildings(id);
        let clergy = &p.population.clergy;
        let governor_piety = state
            .province_governor(id)
            .map(|g| effective_piety(state, data, g));
        let piety = governor_piety.unwrap_or_else(|| ruler_piety(state, data, &controller));
        let growth = f64::from(clergy.unrest) / 20.0
            + (100.0 - f64::from(clergy.goods_satisfaction)) / 40.0
            + 1.0
            - f64::from(religious_buildings(data, &province_buildings))
            - f64::from(piety) / 40.0;
        let value = (f64::from(p.heresy) + growth).round().clamp(0.0, 100.0) as u8;
        let p = state.provinces.get_mut(id).expect("exists");
        p.heresy = value;
        if value == 0 {
            p.heresy_religion = None;
            continue;
        }
        if value > HERESY_SPREAD_THRESHOLD {
            if let Some(pd) = data.provinces.get(id) {
                for neighbor in &pd.neighbors {
                    if state
                        .provinces
                        .get(neighbor)
                        .is_some_and(|n| n.heresy_religion.is_none())
                    {
                        seeds.push((neighbor.clone(), heresy_religion.clone()));
                    }
                }
            }
        }
        if value > HERESY_REVOLT_THRESHOLD && state.rng.chance_permille(100) {
            let p = state.provinces.get_mut(id).expect("exists");
            for entry in [
                &mut p.population.peasants,
                &mut p.population.burghers,
                &mut p.population.clergy,
            ] {
                entry.unrest = entry.unrest.saturating_add(15).min(100);
            }
            let province_name = data
                .provinces
                .get(id)
                .map_or_else(|| id.to_string(), |d| d.name.display.clone());
            events.push(
                GameEvent::new(
                    EventKind::Heresy,
                    format!("Soulèvement hérétique à {province_name} : les prédicateurs dressent le peuple contre l'Église."),
                )
                .province(id)
                .faction(&controller),
            );
        }
    }
    for (province, religion) in seeds {
        if state.rng.chance_permille(100) {
            if let Some(p) = state.provinces.get_mut(&province) {
                if p.heresy_religion.is_none() {
                    p.heresy = 5;
                    p.heresy_religion = Some(religion);
                }
            }
        }
    }
}

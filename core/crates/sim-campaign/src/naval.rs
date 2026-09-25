//! Naval war on the campaign map (lot NV1, ADR 0028).
//!
//! No fleet walks the sea: each faction keeps a pool of warships per class
//! (`data/naval/fleets.json`) and a control of each sea (0-100). A crossing
//! (`Embark`) may be intercepted by a faction at war that arms ships in that
//! sea; the naval battle follows ([`sim_battle::naval`]): fought in 3D or
//! auto-resolved when the player's army crosses (pending in
//! [`NavalState::pending`]), auto-resolved otherwise. Its consequences:
//! losses of the embarked regiments, ships sunk and taken (they change
//! pools), control of the sea to the victor, blockade of the enemy's ports
//! of a sea held (treasury toll each season).

use std::collections::BTreeMap;

use data_model::{
    FactionId, GameData, Marines, NavalRules, SeaZoneId, SettlementId, ShipClassId, UnitTypeId,
};
use serde::{Deserialize, Serialize};
use sim_battle::naval::{
    auto_resolve, CrewSetup, NavalOutcome, NavalSetup, NavalSideSetup, ShipFate, ShipSetup,
};
use sim_battle::{BattleSeason, SideId, UnitSetup};

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState, Season};

/// Holder and strength of the control of one sea.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeaControl {
    pub faction: FactionId,
    /// 0-100.
    pub level: u32,
}

/// A crossing intercepted at sea, awaiting the player's choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavalRequest {
    /// The crossing army (defender).
    pub army: ArmyId,
    pub from: SettlementId,
    pub to: SettlementId,
    /// Faction whose squadron intercepts (attacker).
    pub interceptor: FactionId,
    pub sea: SeaZoneId,
    /// Ships of the squadron per class.
    pub squadron: BTreeMap<ShipClassId, u32>,
    /// Seed of the battle.
    pub seed: u64,
}

/// Naval state of the campaign.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NavalState {
    /// The pools were filled from `data/naval/fleets.json`.
    #[serde(default)]
    pub initialised: bool,
    /// Warships per faction and class.
    #[serde(default)]
    pub fleets: BTreeMap<FactionId, BTreeMap<ShipClassId, u32>>,
    #[serde(default)]
    pub control: BTreeMap<SeaZoneId, SeaControl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<NavalRequest>,
    /// Ports blockaded during the last season.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockaded: Vec<SettlementId>,
}

/// What an interception check decided about a crossing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crossing {
    /// Nobody in the way.
    Clear,
    /// Intercepted; waits for the player (the army stays in port).
    Pending,
    /// Intercepted and fought: the crossing goes on (`true`) or the
    /// survivors are back in port (`false`).
    Fought(bool),
}

/// View of a pending naval battle for the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingNavalBattle {
    pub index: usize,
    pub army: ArmyId,
    pub from: SettlementId,
    pub to: SettlementId,
    pub sea: SeaZoneId,
    pub sea_name: String,
    pub interceptor: FactionId,
    pub interceptor_name: String,
    pub faction: FactionId,
    pub faction_name: String,
    pub interceptor_ships: u32,
    pub transport_ships: u32,
    pub interceptor_men: u32,
    pub army_men: u32,
    /// Share of auto-resolves the player wins (five seeds).
    pub win_chance: f64,
    pub player_side: SideId,
}

/// Why a pending naval battle cannot be resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NavalRequestError {
    #[error("aucune bataille navale en attente n°{0}")]
    Unknown(usize),
    #[error("la bataille navale n°{0} n'a plus lieu d'être (armée disparue)")]
    Stale(usize),
    #[error("résultat invalide : {0}")]
    BadOutcome(String),
}

fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

fn battle_season(season: Season) -> BattleSeason {
    match season {
        Season::Spring => BattleSeason::Spring,
        Season::Summer => BattleSeason::Summer,
        Season::Autumn => BattleSeason::Autumn,
        Season::Winter => BattleSeason::Winter,
    }
}

/// Order in which ships are drawn from a pool for a transport.
const TRANSPORT_ORDER: [&str; 4] = ["ship_nef", "ship_cog", "ship_barge", "ship_galley"];
/// Order in which ships are drawn for a squadron.
const SQUADRON_ORDER: [&str; 4] = ["ship_galley", "ship_nef", "ship_cog", "ship_barge"];

impl NavalState {
    /// Fills the pools from the data on first use.
    pub fn ensure(&mut self, data: &GameData) {
        if self.initialised {
            return;
        }
        self.initialised = true;
        for fleet in &data.naval.fleets.fleets {
            self.fleets
                .insert(fleet.faction.clone(), fleet.ships.clone());
        }
    }

    pub fn ships_of(&self, faction: &FactionId) -> u32 {
        self.fleets.get(faction).map_or(0, |f| f.values().sum())
    }

    /// Control of `sea` held by `faction` (0 if another holds it).
    pub fn control_of(&self, sea: &SeaZoneId, faction: &FactionId) -> u32 {
        self.control
            .get(sea)
            .filter(|c| &c.faction == faction)
            .map_or(0, |c| c.level)
    }

    fn remove_ship(&mut self, faction: &FactionId, class: &str) {
        if let Some(count) = self.fleets.get_mut(faction).and_then(|f| f.get_mut(class)) {
            *count = count.saturating_sub(1);
        }
    }

    fn add_ship(&mut self, faction: &FactionId, class: &str) {
        if let Ok(id) = ShipClassId::new(class) {
            *self
                .fleets
                .entry(faction.clone())
                .or_default()
                .entry(id)
                .or_insert(0) += 1;
        }
    }

    /// The victor of a sea fight gains control of the sea.
    fn win_sea(&mut self, sea: &SeaZoneId, winner: &FactionId, amount: u32) {
        let entry = self.control.entry(sea.clone()).or_insert(SeaControl {
            faction: winner.clone(),
            level: 0,
        });
        if &entry.faction == winner {
            entry.level = (entry.level + amount).min(100);
        } else if entry.level > amount {
            entry.level -= amount;
        } else {
            entry.level = amount - entry.level;
            entry.faction = winner.clone();
        }
    }
}

/// Sea crossed between two ports: a sea both provinces touch, else the
/// first sea of the departure.
pub fn crossing_sea(
    state: &CampaignState,
    data: &GameData,
    from: &SettlementId,
    to: &SettlementId,
) -> Option<SeaZoneId> {
    let seas = |s: &SettlementId| {
        state
            .settlement_province(s)
            .and_then(|p| data.provinces.get(p))
            .map(|p| p.sea_zones.clone())
            .unwrap_or_default()
    };
    let (a, b) = (seas(from), seas(to));
    a.iter()
        .find(|s| b.contains(s))
        .or_else(|| a.first())
        .or_else(|| b.first())
        .cloned()
}

/// The hostile faction most likely to intercept a crossing of `sea` by
/// `faction`, with its chance.
fn interceptor(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    sea: &SeaZoneId,
) -> Option<(FactionId, f64)> {
    let rules = &data.naval.rules;
    let own = f64::from(state.naval.control_of(sea, faction)) / 100.0;
    state
        .factions
        .iter()
        .filter(|(id, f)| f.alive && state.is_at_war(faction, id))
        .filter(|(id, _)| state.naval.ships_of(id) > 0)
        .filter_map(|(id, _)| {
            let control = f64::from(state.naval.control_of(sea, id)) / 100.0;
            let musters = data
                .naval
                .fleet_of(id)
                .is_some_and(|f| f.seas.contains(sea));
            if !musters && control <= 0.0 {
                return None;
            }
            let chance = (rules.intercept_base + control * rules.intercept_per_control
                - own * rules.intercept_per_control * 0.5)
                .clamp(0.0, 0.9);
            Some((id.clone(), chance))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
}

/// Checks whether `army`'s crossing from `from` to `to` is intercepted
/// (called by `Embark`), and fights it when nobody has to choose.
pub(crate) fn intercept(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    from: &SettlementId,
    to: &SettlementId,
    events: &mut Vec<GameEvent>,
) -> Crossing {
    if data.naval.ship_classes.is_empty() {
        return Crossing::Clear;
    }
    state.naval.ensure(data);
    let Some(faction) = state.armies.get(army).map(|a| a.faction.clone()) else {
        return Crossing::Clear;
    };
    let Some(sea) = crossing_sea(state, data, from, to) else {
        return Crossing::Clear;
    };
    let Some((hostile, chance)) = interceptor(state, data, &faction, &sea) else {
        return Crossing::Clear;
    };
    if chance <= 0.0 || state.rng.unit_f64() >= chance {
        return Crossing::Clear;
    }
    let squadron = squadron(state, &hostile, &data.naval.rules);
    if squadron.is_empty() {
        return Crossing::Clear;
    }
    let request = NavalRequest {
        army: army.clone(),
        from: from.clone(),
        to: to.clone(),
        interceptor: hostile.clone(),
        sea,
        squadron,
        seed: state.rng.next_u64(),
    };
    let player = state.player_faction.clone();
    let player_involved = faction == player || hostile == player;
    if player_involved && state.interactive_battles && state.ai_turn.is_none() {
        state.naval.pending.push(request);
        events.push(
            GameEvent::new(
                EventKind::Battle,
                format!(
                    "En mer, une escadre {} barre la route de l'armée {army}.",
                    crate::events::de(&faction_name(data, &hostile))
                ),
            )
            .army(army)
            .faction(&faction),
        );
        return Crossing::Pending;
    }
    let setup = naval_setup(state, data, &request);
    let outcome = auto_resolve(&setup, request.seed);
    let through = apply_outcome(state, data, &request, &setup, &outcome, events);
    Crossing::Fought(through)
}

/// Ships a faction sends to intercept: a share of its pool, strongest
/// classes first.
fn squadron(
    state: &CampaignState,
    faction: &FactionId,
    rules: &NavalRules,
) -> BTreeMap<ShipClassId, u32> {
    let Some(pool) = state.naval.fleets.get(faction) else {
        return BTreeMap::new();
    };
    let total: u32 = pool.values().sum();
    let mut wanted = ((f64::from(total) * rules.squadron_share).round() as u32)
        .clamp(1, rules.squadron_max.max(1))
        .min(total);
    let mut out = BTreeMap::new();
    // Proportional draw, then top up in order.
    for class in SQUADRON_ORDER {
        let Some((id, &count)) = pool.get_key_value(class) else {
            continue;
        };
        let share =
            ((f64::from(count) / f64::from(total.max(1))) * f64::from(wanted)).floor() as u32;
        let take = share.min(count);
        if take > 0 {
            out.insert(id.clone(), take);
        }
    }
    let taken: u32 = out.values().sum();
    wanted = wanted.saturating_sub(taken);
    for class in SQUADRON_ORDER
        .iter()
        .chain(pool.keys().map(|k| k.as_str()).collect::<Vec<_>>().iter())
    {
        if wanted == 0 {
            break;
        }
        let Some((id, &count)) = pool.get_key_value(*class) else {
            continue;
        };
        let have = out.get(id).copied().unwrap_or(0);
        let take = (count - have).min(wanted);
        if take > 0 {
            *out.entry(id.clone()).or_insert(0) += take;
            wanted -= take;
        }
    }
    out
}

fn unit_setup(data: &GameData, id: &UnitTypeId, men: u32) -> Option<UnitSetup> {
    let unit_type = data.unit_types.get(id)?;
    let mut setup = UnitSetup::from_unit_type(unit_type, men, 65, 2);
    setup.max_soldiers = setup.max_soldiers.max(men);
    Some(setup)
}

fn ship_name(data: &GameData, class: &ShipClassId, n: usize) -> String {
    let display = data
        .naval
        .ship(class.as_str())
        .map_or_else(|| class.to_string(), |c| c.name.display.clone());
    format!("{display} n°{}", n + 1)
}

/// The battle of a request: the interceptor's squadron with its marines
/// (attacker) against the crossing army in its transports (defender).
pub fn naval_setup(state: &CampaignState, data: &GameData, request: &NavalRequest) -> NavalSetup {
    let rules = data.naval.rules.clone();
    // Attacker: the squadron and its marines.
    let marines: Marines = data
        .naval
        .fleet_of(&request.interceptor)
        .and_then(|f| f.marines.clone())
        .or_else(|| data.naval.fleets.default_marines.clone())
        .unwrap_or(Marines {
            archers: UnitTypeId::new("unit_crossbowmen").expect("id"),
            soldiers: UnitTypeId::new("unit_men_at_arms_foot").expect("id"),
        });
    let mut units = vec![
        unit_setup(data, &marines.archers, 0),
        unit_setup(data, &marines.soldiers, 0),
    ];
    let mut ships = Vec::new();
    let mut n = 0;
    for (class_id, &count) in &request.squadron {
        let Some(class) = data.naval.ship(class_id.as_str()) else {
            continue;
        };
        for _ in 0..count {
            let men = (f64::from(class.soldiers) * 0.8).round() as u32;
            let archers = (f64::from(men) * rules.marine_archer_share).round() as u32;
            let mut crew = Vec::new();
            for (k, share) in [(0usize, archers), (1usize, men - archers)] {
                if share == 0 {
                    continue;
                }
                if let Some(unit) = units[k].as_mut() {
                    unit.soldiers += share;
                    unit.max_soldiers = unit.max_soldiers.max(unit.soldiers);
                    crew.push(CrewSetup {
                        unit: k,
                        men: share,
                    });
                }
            }
            ships.push(ShipSetup {
                name: ship_name(data, class_id, n),
                class: class.clone(),
                crew,
                fireship: false,
                chain: None,
                fire_arrows: false,
                position: None,
                heading_deg: None,
                flagship: n == 0,
            });
            n += 1;
        }
    }
    // Missing unit types: an empty regiment keeps the indices stable.
    let units: Vec<UnitSetup> = units
        .into_iter()
        .map(|u| {
            u.unwrap_or_else(|| UnitSetup {
                unit_type: "unit_marines".to_owned(),
                name: "Gens de mer".to_owned(),
                category: data_model::UnitCategory::Infantry,
                mounted: false,
                soldiers: 0,
                max_soldiers: 0,
                morale: 60,
                experience: 0,
                stats: crate::battle_request::fallback_stats(),
                abilities: Vec::new(),
                missile: None,
            })
        })
        .collect();
    let attacker = NavalSideSetup {
        faction: request.interceptor.to_string(),
        faction_name: faction_name(data, &request.interceptor),
        army: String::new(),
        admiral: String::new(),
        units,
        ships,
        hold: false,
    };
    let defender = transport_side(state, data, &request.army);
    let player = &state.player_faction;
    let player_side = if &request.interceptor == player {
        Some(SideId::Attacker)
    } else if state
        .armies
        .get(&request.army)
        .is_some_and(|a| &a.faction == player)
    {
        Some(SideId::Defender)
    } else {
        None
    };
    NavalSetup {
        sea_zone: request.sea.to_string(),
        place_name: data.naval.sea_name(&request.sea),
        season: battle_season(state.season),
        rain: state.season == Season::Autumn || state.season == Season::Winter,
        wind_to_deg: None,
        wind_strength: None,
        gauge: None,
        shore: false,
        attacker,
        defender,
        player_side,
        rules,
    }
}

/// The crossing army aboard its transports: warships of its faction's pool
/// first (nefs, cogs, barges, galleys), hired cogs for the rest.
fn transport_side(state: &CampaignState, data: &GameData, army_id: &ArmyId) -> NavalSideSetup {
    let Some(army) = state.armies.get(army_id) else {
        return NavalSideSetup {
            faction: String::new(),
            faction_name: String::new(),
            army: army_id.to_string(),
            admiral: String::new(),
            units: Vec::new(),
            ships: Vec::new(),
            hold: false,
        };
    };
    let side = crate::battle_request::side_setup(state, data, army_id, army);
    let units = side.units;
    let mut men_left: Vec<u32> = units.iter().map(|u| u.soldiers).collect();
    let total: u32 = men_left.iter().sum();
    let pool = state
        .naval
        .fleets
        .get(&army.faction)
        .cloned()
        .unwrap_or_default();
    let mut classes: Vec<ShipClassId> = Vec::new();
    let mut capacity = 0;
    for class in TRANSPORT_ORDER {
        let (Some((id, &count)), Some(ship)) = (pool.get_key_value(class), data.naval.ship(class))
        else {
            continue;
        };
        for _ in 0..count {
            if capacity >= total {
                break;
            }
            classes.push(id.clone());
            capacity += ship.soldiers;
        }
    }
    let hired = ShipClassId::new("ship_cog").expect("id");
    if let Some(cog) = data.naval.ship(hired.as_str()) {
        while capacity < total && cog.soldiers > 0 {
            classes.push(hired.clone());
            capacity += cog.soldiers;
        }
    }
    let mut ships = Vec::new();
    let mut unit = 0;
    for (n, class_id) in classes.iter().enumerate() {
        let Some(class) = data.naval.ship(class_id.as_str()) else {
            continue;
        };
        let mut room = class.soldiers;
        let mut crew = Vec::new();
        while room > 0 && unit < men_left.len() {
            if men_left[unit] == 0 {
                unit += 1;
                continue;
            }
            let take = men_left[unit].min(room);
            crew.push(CrewSetup { unit, men: take });
            men_left[unit] -= take;
            room -= take;
        }
        ships.push(ShipSetup {
            name: ship_name(data, class_id, n),
            class: class.clone(),
            crew,
            fireship: false,
            chain: None,
            fire_arrows: false,
            position: None,
            heading_deg: None,
            flagship: n == 0,
        });
    }
    NavalSideSetup {
        faction: army.faction.to_string(),
        faction_name: faction_name(data, &army.faction),
        army: army_id.to_string(),
        admiral: side.general.map(|g| g.name).unwrap_or_default(),
        units,
        ships,
        hold: false,
    }
}

/// Applies a naval battle to the campaign. Returns whether the crossing
/// goes on (the army survived and its side was not beaten).
pub(crate) fn apply_outcome(
    state: &mut CampaignState,
    data: &GameData,
    request: &NavalRequest,
    setup: &NavalSetup,
    outcome: &NavalOutcome,
    events: &mut Vec<GameEvent>,
) -> bool {
    let Some(army_faction) = state.armies.get(&request.army).map(|a| a.faction.clone()) else {
        return false;
    };
    // Regiments of the crossing army.
    if let Some(army) = state.armies.get_mut(&request.army) {
        for (unit, lost) in army.units.iter_mut().zip(&outcome.defender.unit_losses) {
            unit.strength = unit.strength.saturating_sub(*lost);
            if *lost > 0 {
                unit.morale = unit.morale.saturating_sub(10);
            }
        }
        army.units.retain(|u| u.strength > 0);
    }
    // Ships lost and taken.
    let sides = [
        (&request.interceptor, &outcome.attacker, &setup.attacker),
        (&army_faction, &outcome.defender, &setup.defender),
    ];
    for (faction, result, _) in sides {
        for ship in &result.ships {
            if matches!(ship.fate, ShipFate::Captured | ShipFate::Sunk) {
                state.naval.remove_ship(faction, &ship.class);
            }
        }
        for prize in &result.prizes {
            state.naval.add_ship(faction, prize);
        }
    }
    let winner = outcome.winner.map(|w| match w {
        SideId::Attacker => request.interceptor.clone(),
        SideId::Defender => army_faction.clone(),
    });
    if let Some(winner) = &winner {
        let amount = data.naval.rules.control_victory;
        state.naval.win_sea(&request.sea, winner, amount);
    }
    let destroyed = state
        .armies
        .get(&request.army)
        .is_none_or(|a| a.units.is_empty());
    let sea = data.naval.sea_name(&request.sea);
    let text = format!(
        "Bataille navale dans {sea} : l'escadre {} contre l'armée {} ({}). {} Navires pris : {} ; coulés : {} ({} contre {}). Pertes : {} hommes contre {}.",
        crate::events::de(&faction_name(data, &request.interceptor)),
        request.army,
        faction_name(data, &army_faction),
        match &winner {
            Some(w) if w == &request.interceptor => "La traversée est brisée.".to_owned(),
            Some(_) => "L'escadre est repoussée, la traversée continue.".to_owned(),
            None => "Les deux flottes se séparent.".to_owned(),
        },
        outcome.attacker.prizes.len() + outcome.defender.prizes.len(),
        outcome.attacker.count(ShipFate::Sunk) + outcome.defender.count(ShipFate::Sunk),
        faction_name(data, &request.interceptor),
        faction_name(data, &army_faction),
        outcome.attacker.men_lost,
        outcome.defender.men_lost,
    );
    events.push(
        GameEvent::new(EventKind::Battle, text)
            .army(&request.army)
            .faction(&army_faction),
    );
    if destroyed {
        if let Some(general) = state
            .armies
            .get(&request.army)
            .and_then(|a| a.general.clone())
        {
            state.detach_general(&general);
        }
        state.armies.remove(&request.army);
        events.push(
            GameEvent::new(
                EventKind::ArmyDestroyed,
                format!("L'armée {} a péri en mer.", request.army),
            )
            .faction(&army_faction),
        );
        return false;
    }
    winner.as_ref().is_none_or(|w| w == &army_faction)
}

impl CampaignState {
    /// Pending naval battles of the player (lot NV1).
    pub fn pending_naval_views(&self, data: &GameData) -> Vec<PendingNavalBattle> {
        self.naval
            .pending
            .iter()
            .enumerate()
            .filter_map(|(index, request)| {
                let army = self.armies.get(&request.army)?;
                let setup = naval_setup(self, data, request);
                let player_side = setup.player_side.unwrap_or(SideId::Defender);
                let wins = (0..5u64)
                    .filter(|k| {
                        auto_resolve(&setup, request.seed.wrapping_add(k * 7919)).winner
                            == Some(player_side)
                    })
                    .count();
                Some(PendingNavalBattle {
                    index,
                    army: request.army.clone(),
                    from: request.from.clone(),
                    to: request.to.clone(),
                    sea: request.sea.clone(),
                    sea_name: data.naval.sea_name(&request.sea),
                    interceptor: request.interceptor.clone(),
                    interceptor_name: faction_name(data, &request.interceptor),
                    faction: army.faction.clone(),
                    faction_name: faction_name(data, &army.faction),
                    interceptor_ships: setup.attacker.ships.len() as u32,
                    transport_ships: setup.defender.ships.len() as u32,
                    interceptor_men: setup.attacker.men(),
                    army_men: setup.defender.men(),
                    win_chance: wins as f64 / 5.0,
                    player_side,
                })
            })
            .collect()
    }

    /// Setup of pending naval battle `index` for the 3D battle.
    pub fn naval_battle_setup(
        &self,
        data: &GameData,
        index: usize,
    ) -> Result<NavalSetup, NavalRequestError> {
        let request = self
            .naval
            .pending
            .get(index)
            .ok_or(NavalRequestError::Unknown(index))?;
        if !self.armies.contains_key(&request.army) {
            return Err(NavalRequestError::Stale(index));
        }
        Ok(naval_setup(self, data, request))
    }

    /// Seed of pending naval battle `index`.
    pub fn naval_battle_seed(&self, index: usize) -> Option<u64> {
        self.naval.pending.get(index).map(|r| r.seed)
    }

    /// Applies the result of the 3D naval battle `index`; the crossing goes
    /// on if the army holds the sea.
    pub fn resolve_naval_battle(
        &mut self,
        data: &GameData,
        index: usize,
        outcome: &NavalOutcome,
    ) -> Result<Vec<GameEvent>, NavalRequestError> {
        let request = self.take_naval(index)?;
        let setup = naval_setup(self, data, &request);
        if outcome.defender.unit_losses.len() != setup.defender.units.len() {
            return Err(NavalRequestError::BadOutcome(format!(
                "{} régiments attendus, {} pertes fournies",
                setup.defender.units.len(),
                outcome.defender.unit_losses.len()
            )));
        }
        let mut events = Vec::new();
        let through = apply_outcome(self, data, &request, &setup, outcome, &mut events);
        if through {
            self.land_crossing(data, &request.army, &request.to, &mut events);
        }
        self.pending_events.extend(events.iter().cloned());
        Ok(events)
    }

    /// Auto-resolves pending naval battle `index`.
    pub fn auto_resolve_naval_battle(
        &mut self,
        data: &GameData,
        index: usize,
    ) -> Result<Vec<GameEvent>, NavalRequestError> {
        let request = self
            .naval
            .pending
            .get(index)
            .cloned()
            .ok_or(NavalRequestError::Unknown(index))?;
        if !self.armies.contains_key(&request.army) {
            self.naval.pending.remove(index);
            return Err(NavalRequestError::Stale(index));
        }
        let setup = naval_setup(self, data, &request);
        let outcome = auto_resolve(&setup, request.seed);
        self.resolve_naval_battle(data, index, &outcome)
    }

    /// The crossing fleet puts back into port: no battle, the turn is spent.
    pub fn withdraw_naval_battle(
        &mut self,
        data: &GameData,
        index: usize,
    ) -> Result<Vec<GameEvent>, NavalRequestError> {
        let request = self.take_naval(index)?;
        let text = format!(
            "La flotte de l'armée {} rentre au port de {} sans combattre.",
            request.army,
            crate::siege::settlement_name(data, &request.from)
        );
        let mut event = GameEvent::new(EventKind::Battle, text).army(&request.army);
        if let Some(army) = self.armies.get(&request.army) {
            event = event.faction(&army.faction);
        }
        self.pending_events.push(event.clone());
        Ok(vec![event])
    }

    fn take_naval(&mut self, index: usize) -> Result<NavalRequest, NavalRequestError> {
        if index >= self.naval.pending.len() {
            return Err(NavalRequestError::Unknown(index));
        }
        let request = self.naval.pending.remove(index);
        if !self.armies.contains_key(&request.army) {
            return Err(NavalRequestError::Stale(index));
        }
        Ok(request)
    }

    /// Stages a pending interception of `army` crossing to `to_port` by
    /// `interceptor` (tests, screenshots); returns its index.
    pub fn debug_stage_naval(
        &mut self,
        data: &GameData,
        army: &ArmyId,
        to_port: &SettlementId,
        interceptor: &FactionId,
    ) -> Option<usize> {
        self.naval.ensure(data);
        let mut from = self.armies.get(army)?.settlement().cloned();
        if from
            .as_ref()
            .and_then(|f| crossing_sea(self, data, f, to_port))
            .is_none()
        {
            // Debug staging: move the army to a friendly port facing `to_port`.
            let faction = self.armies.get(army)?.faction.clone();
            let port = data
                .movement_graph
                .adjacency
                .iter()
                .find(|(f, edges)| {
                    self.is_friendly_settlement(&faction, f)
                        && edges.iter().any(|e| e.sea && &e.to == to_port)
                })
                .map(|(f, _)| f.clone())?;
            self.armies.get_mut(army)?.position =
                crate::state::ArmyPosition::Settlement(port.clone());
            from = Some(port);
        }
        let from = from?;
        let sea = crossing_sea(self, data, &from, to_port)?;
        let squadron = squadron(self, interceptor, &data.naval.rules);
        if squadron.is_empty() {
            return None;
        }
        let seed = self.rng.next_u64();
        self.naval.pending.push(NavalRequest {
            army: army.clone(),
            from,
            to: to_port.clone(),
            interceptor: interceptor.clone(),
            sea,
            squadron,
            seed,
        });
        Some(self.naval.pending.len() - 1)
    }
}

/// End of turn: pending naval battles of the player are auto-resolved.
pub(crate) fn auto_resolve_all_pending(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    while !state.naval.pending.is_empty() {
        match state.auto_resolve_naval_battle(data, 0) {
            Ok(mut fought) => events.append(&mut fought),
            Err(_) => {
                if !state.naval.pending.is_empty() {
                    state.naval.pending.remove(0);
                }
            }
        }
    }
}

/// End of season: sea control fades, the ports of a sea held by an enemy
/// are blockaded (treasury toll), fleets are slowly rebuilt towards their
/// 1337 strength.
pub(crate) fn resolve_season(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    if data.naval.ship_classes.is_empty() {
        return;
    }
    state.naval.ensure(data);
    let rules = data.naval.rules.clone();
    // Blockades.
    let mut blockaded = Vec::new();
    let mut tolls: BTreeMap<FactionId, (u32, i64)> = BTreeMap::new();
    for (sea, control) in &state.naval.control {
        if control.level < rules.blockade_control {
            continue;
        }
        for (province_id, province) in &data.provinces {
            if !province.has_port() || !province.sea_zones.contains(sea) {
                continue;
            }
            let Some(owner) = state.province_owner(province_id).cloned() else {
                continue;
            };
            if !state.is_at_war(&control.faction, &owner) {
                continue;
            }
            if let Some(city) = state.provinces.get(province_id).map(|p| p.city.clone()) {
                blockaded.push(city);
            }
            let entry = tolls.entry(owner).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += i64::from(rules.blockade_toll);
        }
    }
    for (faction, (ports, toll)) in tolls {
        if let Some(f) = state.factions.get_mut(&faction) {
            f.treasury -= toll;
        }
        events.push(
            GameEvent::new(
                EventKind::Income,
                format!("Blocus : {ports} port(s) fermé(s) par l'ennemi, {toll} livres de commerce perdues."),
            )
            .faction(&faction),
        );
    }
    blockaded.sort();
    blockaded.dedup();
    state.naval.blockaded = blockaded;
    // Control fades.
    state.naval.control.retain(|_, c| {
        c.level = c.level.saturating_sub(rules.control_decay);
        c.level > 0
    });
    // Shipyards and requisitions.
    for fleet in &data.naval.fleets.fleets {
        let alive = state.factions.get(&fleet.faction).is_some_and(|f| f.alive);
        if !alive {
            continue;
        }
        let pool = state.naval.fleets.entry(fleet.faction.clone()).or_default();
        for (class, &target) in &fleet.ships {
            let count = pool.entry(class.clone()).or_insert(0);
            if *count < target {
                *count = (*count + (target / 10).max(1)).min(target);
            }
        }
    }
}

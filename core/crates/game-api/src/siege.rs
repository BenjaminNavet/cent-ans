//! Siege sheet (campaign siege panel).
//!
//! Replaces `CampaignSim.get_assault_odds` and the siege part of `get_province_state`
//! (`siege_controller.gd`). Pure view: orders go through `submit_order_outcome`
//! (`Order::Assault`, `Order::DemandSurrender`, `Order::Sortie`, `Order::SetStance`).

use data_model::{GameData, SettlementId};
use serde::Serialize;
use sim_campaign::{ArmyId, CampaignState, Stance};

use crate::common::{unit_views, UnitView};

/// One siege engine of the besiegers (`data/rules/siege_engines.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineView {
    pub id: String,
    pub name: String,
    /// `ladders`, `ram`, `tower` (serialised `BuiltEngineKind`).
    pub kind: String,
    pub ready: bool,
    pub turns_left: u32,
}

/// An army besieging the place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BesiegerView {
    pub id: String,
    pub name: String,
    pub faction: String,
    pub faction_name: String,
    pub men: u32,
    pub regiments: usize,
    /// In `siege` stance (an army that can be told to lift the siege).
    pub sieging: bool,
}

/// Role of the player at this siege.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SiegeRole {
    Attacker,
    Defender,
    Observer,
}

/// Everything the siege panel shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SiegeSheet {
    pub settlement: String,
    pub settlement_name: String,
    pub province: String,
    pub province_name: String,
    pub attacker: String,
    pub attacker_name: String,
    pub defender: String,
    pub defender_name: String,
    pub role: SiegeRole,
    pub turns_elapsed: u32,
    /// Estimated turns before the garrison starves.
    pub turns_left: u32,
    /// Food left in the town (0-100).
    pub supplies: u8,
    /// Wall damage (0-100); from 50 the wall penalty disappears.
    pub breach: u8,
    pub fortification_level: u32,
    /// Walls still count against an assault (fortified, breach under 50, no tower yet).
    pub walls_stand: bool,
    pub garrison: Vec<UnitView>,
    pub garrison_men: u32,
    pub engines: Vec<EngineView>,
    pub besiegers: Vec<BesiegerView>,
    pub besieger_men: u32,
    /// Player attacker only: army that would storm (the player's most numerous besieger).
    pub assault_army: Option<String>,
    /// Estimated assault success (0-100); player attacker only.
    pub assault_odds: Option<u32>,
    /// French reason an assault is refused now (empty: allowed).
    pub assault_blocker: String,
    /// Chance (%) the garrison yields to a summons now.
    pub surrender_chance: u32,
    /// The defender may sally out (garrison not empty).
    pub can_sortie: bool,
    /// Player's besieging armies still in `siege` stance (target of « Lever le siège »).
    pub liftable_armies: Vec<String>,
}

/// Settlement under siege in `place` (a settlement id, or a province id: its first besieged
/// settlement, the city first).
pub fn besieged_settlement(state: &CampaignState, place: &str) -> Option<SettlementId> {
    let besieged = |id: &SettlementId| {
        state
            .settlement_state(id)
            .is_some_and(|s| s.siege.is_some())
    };
    if let Ok(id) = SettlementId::new(place) {
        return besieged(&id).then_some(id);
    }
    let province = data_model::ProvinceId::new(place).ok()?;
    if let Some(city) = state.province_city_id(&province).filter(|c| besieged(c)) {
        return Some(city.clone());
    }
    state
        .settlements
        .iter()
        .find(|(_, s)| s.province == province && s.siege.is_some())
        .map(|(id, _)| id.clone())
}

/// Sheet of the siege at `place` (settlement or province id); `None` without a siege.
pub fn siege_sheet(data: &GameData, state: &CampaignState, place: &str) -> Option<SiegeSheet> {
    let id = besieged_settlement(state, place)?;
    let settlement = state.settlement_state(&id)?;
    let siege = settlement.siege.as_ref()?;
    let player = state.player_faction();
    let besiegers: Vec<BesiegerView> = state
        .armies_at(&id)
        .into_iter()
        .filter_map(|army_id| {
            let army = state.army(&army_id)?;
            let hostile = army.faction == siege.attacker
                || (army.faction != settlement.controller
                    && state.is_at_war(&army.faction, &settlement.controller)
                    && army.stance == Stance::Siege);
            hostile.then(|| BesiegerView {
                id: army_id.to_string(),
                name: state.army_name(data, &army_id),
                faction: army.faction.as_str().to_owned(),
                faction_name: data.faction_name(&army.faction),
                men: army.units.iter().map(|u| u.strength).sum(),
                regiments: army.units.len(),
                sieging: army.stance == Stance::Siege,
            })
        })
        .collect();
    let role = if &siege.attacker == player {
        SiegeRole::Attacker
    } else if &settlement.controller == player {
        SiegeRole::Defender
    } else {
        SiegeRole::Observer
    };
    let assault_army: Option<ArmyId> = (role == SiegeRole::Attacker)
        .then(|| {
            besiegers
                .iter()
                .filter(|b| b.faction == player.as_str())
                .max_by_key(|b| b.men)
                .and_then(|b| ArmyId::parse(&b.id))
        })
        .flatten();
    let fortification_level = state.fortification_level(data, &id);
    let odds = assault_army
        .as_ref()
        .and_then(|army| state.assault_odds(data, army));
    let walls_stand = odds.map_or(fortification_level > 0 && siege.breach < 50, |(_, w)| w);
    let engines = state
        .siege_engines(data, &id)
        .into_iter()
        .map(|e| EngineView {
            id: e.id,
            name: e.name,
            kind: serde_json::to_value(e.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            ready: e.ready,
            turns_left: e.turns_left,
        })
        .collect();
    let liftable_armies = if role == SiegeRole::Attacker {
        besiegers
            .iter()
            .filter(|b| b.sieging && b.faction == player.as_str())
            .map(|b| b.id.clone())
            .collect()
    } else {
        Vec::new()
    };
    let province = settlement.province.clone();
    Some(SiegeSheet {
        settlement: id.as_str().to_owned(),
        settlement_name: data.settlement_name(&id),
        province: province.as_str().to_owned(),
        province_name: data.province_name(&province),
        attacker: siege.attacker.as_str().to_owned(),
        attacker_name: data.faction_name(&siege.attacker),
        defender: settlement.controller.as_str().to_owned(),
        defender_name: data.faction_name(&settlement.controller),
        role,
        turns_elapsed: siege.turns_elapsed,
        turns_left: siege.turns_left,
        supplies: siege.supplies,
        breach: siege.breach,
        fortification_level,
        walls_stand,
        garrison: unit_views(data, &settlement.garrison),
        garrison_men: settlement.garrison.iter().map(|u| u.strength).sum(),
        engines,
        besieger_men: besiegers.iter().map(|b| b.men).sum(),
        besiegers,
        assault_odds: assault_army.as_ref().map(|army| {
            state
                .assault_win_chance(data, army)
                .map_or(0, |c| (c * 100.0).round() as u32)
        }),
        assault_blocker: assault_army
            .as_ref()
            .and_then(|army| state.assault_blocker(data, army))
            .unwrap_or_default(),
        assault_army: assault_army.map(|a| a.to_string()),
        surrender_chance: sim_campaign::siege::surrender_chance(state, data, &id).unwrap_or(0),
        can_sortie: role == SiegeRole::Defender && !settlement.garrison.is_empty(),
        liftable_armies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_siege_no_sheet() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        assert!(siege_sheet(data, &state, "prov_ile_de_france").is_none());
        assert!(besieged_settlement(&state, "nonsense").is_none());
    }

    #[test]
    fn staged_siege_sheet() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let army = sim_campaign::test_support::main_army(&state, "fac_france");
        let target = state
            .settlements
            .iter()
            .find(|(_, s)| &s.controller != state.player_faction() && !s.garrison.is_empty())
            .map(|(id, _)| id.clone())
            .expect("an enemy-held settlement");
        let attacker = state.player_faction().clone();
        {
            let a = state.armies.get_mut(&army).unwrap();
            a.position = sim_campaign::ArmyPosition::Settlement(target.clone());
            a.stance = Stance::Siege;
        }
        state.settlements.get_mut(&target).unwrap().siege = Some(sim_campaign::SiegeState {
            attacker,
            turns_left: 3,
            turns_elapsed: 1,
            supplies: 40,
            breach: 10,
            started_turn: 0,
            engine_work: 0,
        });
        let sheet = siege_sheet(data, &state, target.as_str()).expect("sheet");
        assert_eq!(sheet.role, SiegeRole::Attacker);
        assert_eq!(sheet.supplies, 40);
        assert_eq!(
            sheet.assault_army.as_deref(),
            Some(army.to_string().as_str())
        );
        assert!(sheet.assault_odds.is_some());
        assert!(!sheet.garrison.is_empty());
        assert_eq!(sheet.liftable_armies.len(), 1);
        assert!(serde_json::to_string(&sheet).is_ok());
        // Province id resolves to the same siege.
        let by_province = siege_sheet(data, &state, sheet.province.as_str()).expect("by province");
        assert_eq!(by_province.settlement, sheet.settlement);
    }
}

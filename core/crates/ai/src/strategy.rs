//! Strategic doctrines and difficulty tiers as the planner reads them (ADR 0782, 0783).
//!
//! The doctrine of a faction (`sim_campaign::doctrines`) and the behaviour tiers of the
//! difficulty level (prudence, anticipation, coordination, fog of war) are gathered in one
//! [`Strategy`], built once per faction and turn. Every factor is neutral for the `balanced`
//! doctrine at the `normal` level: the planner then plays exactly as before the doctrines.

use std::collections::BTreeSet;

use data_model::{
    DifficultyModifiers, DoctrineObjectives, FactionId, GameData, ProvinceId, StrategicDoctrines,
};
use sim_campaign::doctrines::{self, ActiveDoctrine};
use sim_campaign::{CampaignState, Order};

/// What shapes the strategic choices of one faction this turn.
#[derive(Clone, Copy)]
pub struct Strategy {
    pub doctrine: ActiveDoctrine,
    pub difficulty: DifficultyModifiers,
}

impl Strategy {
    /// The strategy of `faction`: its doctrine, and the difficulty levers of an AI faction
    /// (neutral for the player's own).
    pub fn of(state: &CampaignState, data: &GameData, faction: &FactionId) -> Self {
        let difficulty = if faction == &state.player_faction {
            DifficultyModifiers::NEUTRAL
        } else {
            state.difficulty_modifiers(data)
        };
        Strategy {
            doctrine: doctrines::active(state, faction),
            difficulty,
        }
    }

    /// Multiplier of the superiority an army demands before attacking an enemy army:
    /// the doctrine's taste for battle and the prudence of the level.
    pub fn engage_factor(&self) -> f64 {
        f64::from(self.doctrine.doctrine.battle.engage_ratio_percent) / 100.0
            * f64::from(self.difficulty.ai_prudence_percent)
            / 100.0
    }

    /// Multiplier of the share of strength under which an army falls back.
    pub fn retreat_factor(&self) -> f64 {
        f64::from(self.doctrine.doctrine.battle.retreat_percent) / 100.0
            * f64::from(self.difficulty.ai_prudence_percent)
            / 100.0
    }

    /// Multiplier of the superiority demanded over a place before besieging it.
    pub fn siege_factor(&self) -> f64 {
        f64::from(self.doctrine.doctrine.battle.siege_superiority_percent) / 100.0
    }

    /// Multiplier of the value of a kind of target (see [`DoctrineObjectives`]).
    pub fn conquest(&self) -> f64 {
        DoctrineObjectives::factor(self.doctrine.doctrine.objectives.conquest)
    }
    pub fn revenge(&self) -> f64 {
        DoctrineObjectives::factor(self.doctrine.doctrine.objectives.revenge)
    }
    pub fn security(&self) -> f64 {
        DoctrineObjectives::factor(self.doctrine.doctrine.objectives.security)
    }
    pub fn treasury(&self) -> f64 {
        DoctrineObjectives::factor(self.doctrine.doctrine.objectives.treasury)
    }
    pub fn prestige(&self) -> f64 {
        DoctrineObjectives::factor(self.doctrine.doctrine.objectives.prestige)
    }

    /// Value added to a siege target held by a rival the faction wants to avenge itself on.
    pub fn revenge_bonus(
        &self,
        state: &CampaignState,
        faction: &FactionId,
        holder: &FactionId,
    ) -> f64 {
        if doctrines::revenge_on(state, faction, holder).is_none() {
            return 0.0;
        }
        StrategicDoctrines::bundled().tuning.revenge_target_bonus * self.revenge()
    }

    /// Whether an own army marching to the same target counts for a siege or a defence.
    pub fn coordinated(&self) -> bool {
        self.difficulty.ai_coordination >= 1
    }

    /// Whether the AI reads the destination of the hostile armies.
    pub fn anticipates(&self) -> bool {
        self.difficulty.ai_anticipation >= 1
    }

    /// Provinces whose hostile armies the AI sees (`None`: all of them, no fog).
    pub fn visible_provinces(
        &self,
        state: &CampaignState,
        data: &GameData,
        faction: &FactionId,
    ) -> Option<BTreeSet<ProvinceId>> {
        let depth = self.difficulty.ai_vision_provinces;
        if depth == 0 {
            return None;
        }
        let mut seen: BTreeSet<ProvinceId> = state
            .provinces
            .keys()
            .filter(|p| state.holds_province(faction, p))
            .cloned()
            .collect();
        seen.extend(
            state
                .armies
                .values()
                .filter(|a| &a.faction == faction)
                .filter_map(|a| state.army_province(data, a)),
        );
        let mut frontier: Vec<ProvinceId> = seen.iter().cloned().collect();
        for _ in 0..depth {
            let mut next = Vec::new();
            for province in &frontier {
                for neighbour in data.province_land_neighbors(province).iter() {
                    if seen.insert(neighbour.clone()) {
                        next.push(neighbour.clone());
                    }
                }
            }
            frontier = next;
        }
        Some(seen)
    }
}

/// ADR 0782: a doctrine reluctant to borrow (or to levy) keeps the finance order only when its
/// treasury is empty; the others take the order of the finance planner as it comes.
pub fn filter_finance_orders(
    state: &CampaignState,
    faction: &FactionId,
    orders: Vec<Order>,
) -> Vec<Order> {
    let finance = doctrines::active(state, faction).doctrine.finance;
    let broke = state.factions.get(faction).is_none_or(|f| f.treasury < 0);
    orders
        .into_iter()
        .filter(|order| match order {
            Order::Borrow { .. } => broke || finance.borrow_percent >= 50,
            Order::LevyExtraordinary { .. } | Order::ConveneAssembly { .. } => {
                broke || finance.levy_percent >= 50
            }
            _ => true,
        })
        .collect()
}

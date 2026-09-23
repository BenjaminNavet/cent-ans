//! End-of-turn resolution in the fixed order of spec § 1.3.

use data_model::{FactionId, GameData};

use crate::events::GameEvent;
use crate::orders::Order;
use crate::state::CampaignState;
use crate::{
    ai_minimal, buildings, characters, diplomacy, dynasty, economy, movement, population, religion,
    research, siege,
};

impl CampaignState {
    /// Resolves the turn with the built-in minimal AI for every non-player faction.
    ///
    /// Returns the turn's events (also kept in [`CampaignState::events`]).
    pub fn end_turn(&mut self, data: &GameData) -> Vec<GameEvent> {
        self.end_turn_with(data, ai_minimal::plan_turn)
    }

    /// Resolves the turn, asking `planner` for the orders of each AI faction.
    pub fn end_turn_with<P>(&mut self, data: &GameData, planner: P) -> Vec<GameEvent>
    where
        P: Fn(&CampaignState, &GameData, &FactionId) -> Vec<Order>,
    {
        let mut events = Vec::new();

        // 1. AI orders (invalid ones are silently dropped: the planner is advisory).
        let ai_factions: Vec<FactionId> = self
            .factions
            .iter()
            .filter(|(id, f)| f.alive && **id != self.player_faction)
            .map(|(id, _)| id.clone())
            .collect();
        for faction in ai_factions {
            for order in planner(self, data, &faction) {
                let _ = self.apply_order(data, &faction, order);
            }
        }

        // 2-4. Movement and battles, sieges, chevauchées.
        movement::resolve_movement(self, data, &mut events);
        siege::resolve_sieges(self, data, &mut events);
        siege::resolve_raids(self, data, &mut events);

        // 5. Buildings and goods, ahead of the economy that reads them (M3).
        buildings::resolve_construction(self, data, &mut events);
        economy::resolve_goods(self, data);

        // 6-8. Economy, attrition, recovery.
        economy::resolve_economy(self, data, &mut events);
        research::resolve_research(self, data, &mut events);
        economy::resolve_attrition(self, data, &mut events);
        economy::resolve_decay(self);

        // 8b. Diplomacy (vassal tribute after the economy, expiries,
        // rebellions) and religion (favour, Schism, heresy) before the
        // population reads their unrest (M5).
        diplomacy::resolve_diplomacy(self, data, &mut events);
        religion::resolve_religion(self, data, &mut events);

        // 9. Population dynamics: growth, health, wealth, goods
        // satisfaction, unrest, revolt, plague, famine (M3).
        population::resolve_population(self, data, &mut events);

        // 10. Characters: governance XP, deaths, winter births, regencies
        // (M4), then dead factions.
        dynasty::resolve_governance(self);
        characters::resolve_characters(self, data, &mut events);
        dynasty::resolve_births(self, data, &mut events);
        dynasty::resolve_regencies(self, data, &mut events);
        characters::resolve_faction_deaths(self, data, &mut events);

        // 11. New season.
        self.advance_date();
        let movement_points = self.season.movement_points();
        for army in self.armies.values_mut() {
            army.movement_points = movement_points;
        }
        let turn = self.turn;
        for faction in self.factions.values_mut() {
            faction.truces.retain(|_, until| *until > turn);
        }
        self.pending_battles.clear();
        self.events = events.clone();
        events
    }
}

//! End-of-turn resolution in the fixed order of spec § 1.3.

use data_model::{FactionId, GameData};

use crate::events::GameEvent;
use crate::orders::Order;
use crate::state::CampaignState;
use crate::{
    ai_minimal, battle_request, buildings, characters, chronicle, diplomacy, dynasty, economy,
    movement, population, religion, research, siege, table,
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

        // 0. Player battles left pending from the previous turn are
        // auto-resolved first (M7).
        battle_request::auto_resolve_all_pending(self, data, &mut events);

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
        // H3: diets whose requirements no longer hold fall back to the default.
        table::resolve_requirements(self, data, &mut events);

        // 6-8. Economy, attrition, recovery.
        economy::resolve_economy(self, data, &mut events);
        // C5: trade routes and agreements, after the treasury's tax income.
        crate::trade::resolve_trade(self, data, &mut events);
        // H5: prices follow the coinage.
        crate::coinage::resolve_coinage(self, &mut events);
        // H6: ransom installments, captive rulers.
        crate::ransom::resolve_ransoms(self, data, &mut events);
        // H6: chivalric orders (collapse, new members, yearly prestige).
        crate::chivalry::resolve_chivalry(self, data, &mut events);
        research::resolve_research(self, data, &mut events);
        economy::resolve_attrition(self, data, &mut events);
        economy::resolve_decay(self, data);

        // 8b. Diplomacy (vassal tribute after the economy, expiries,
        // rebellions) and religion (favour, Schism, heresy) before the
        // population reads their unrest (M5).
        diplomacy::resolve_diplomacy(self, data, &mut events);
        religion::resolve_religion(self, data, &mut events);
        // 8b'. Agents: upkeep, counter-espionage, stale intelligence (C6).
        crate::agents::resolve_agents(self, data, &mut events);

        // 8c. Chronicle: historical and random events (M10).
        chronicle::resolve_chronicle(self, data, &mut events);

        // 8d. The table: ruler piety/prestige and Lent in spring (H3).
        table::resolve_lent(self, data, &mut events);

        // 9. Population dynamics: growth, health, wealth, goods
        // satisfaction, unrest, revolt, plague, famine (M3).
        population::resolve_population(self, data, &mut events);

        // 10. Characters: governance XP, deaths, winter births, regencies
        // (M4), then dead factions.
        dynasty::resolve_governance(self);
        dynasty::resolve_court_prestige(self, data);
        crate::retinue::resolve_retinue(self, data, &mut events);
        characters::resolve_characters(self, data, &mut events);
        dynasty::resolve_births(self, data, &mut events);
        dynasty::resolve_regencies(self, data, &mut events);
        characters::resolve_faction_deaths(self, data, &mut events);

        // 11. New season.
        self.advance_date();
        let allowances: Vec<(crate::state::ArmyId, u32)> = self
            .armies
            .iter()
            .map(|(id, army)| (id.clone(), self.army_movement_allowance(data, army)))
            .collect();
        for (id, points) in allowances {
            if let Some(army) = self.armies.get_mut(&id) {
                army.movement_points = points;
            }
        }
        // C6: agents get their points back and resume their march.
        crate::agents::start_season(self, data);
        let turn = self.turn;
        for faction in self.factions.values_mut() {
            faction.truces.retain(|_, until| *until > turn);
        }
        // M10: victory, defeat or end of the campaign for the player.
        crate::victory::resolve_victory(self, data, &mut events);
        self.events = events.clone();
        events
    }
}

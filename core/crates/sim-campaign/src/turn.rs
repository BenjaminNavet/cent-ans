//! End-of-turn resolution in the fixed order of spec § 1.3.

use data_model::{FactionId, GameData};

use crate::events::GameEvent;
use crate::orders::Order;
use crate::state::CampaignState;
use crate::{
    ai_minimal, battle_request, buildings, characters, chronicle, diplomacy, dynasty, economy,
    edicts, march, population, religion, research, siege, table,
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
        self.end_turn_profiled(data, planner).0
    }

    /// [`CampaignState::end_turn_with`], also returning the time each AI
    /// faction took to play its turn (marches, planning, orders), in the
    /// order they played (lot M3 performance probe).
    pub fn end_turn_profiled<P>(
        &mut self,
        data: &GameData,
        planner: P,
    ) -> (Vec<GameEvent>, Vec<(FactionId, std::time::Duration)>)
    where
        P: Fn(&CampaignState, &GameData, &FactionId) -> Vec<Order>,
    {
        let mut events = Vec::new();
        // UI audit A3 (U3): purses before the turn, for the budget history.
        let purses = self.purses();
        let resolved_turn = self.turn;

        // 1. The player's turn is over. His battles still pending are
        // auto-resolved first (M7).
        // CV3-3: encounters left unanswered take their default option.
        crate::encounter::resolve_unanswered(self, data, &mut events);
        battle_request::auto_resolve_all_pending(self, data, &mut events);
        crate::naval::auto_resolve_all_pending(self, data, &mut events);

        // 2. Each AI faction plays in id order (spec § 3.4).
        let ai_factions: Vec<FactionId> = self
            .factions
            .iter()
            .filter(|(id, f)| f.alive && **id != self.player_faction)
            .map(|(id, _)| id.clone())
            .collect();
        let mut timings = Vec::with_capacity(ai_factions.len());
        // CT1: the record of the AI moves (for the replay) starts afresh.
        self.ai_replay_begin(data);
        for faction in ai_factions {
            let started = std::time::Instant::now();
            self.play_ai_turn(data, &faction, &planner, &mut events);
            timings.push((faction, started.elapsed()));
        }

        self.resolve_end_of_turn(data, &mut events);
        self.ai_replay_finish(data);
        self.record_budget_history(&purses, resolved_turn);
        (events, timings)
    }

    /// The turn of AI faction `faction` (spec § 3.4, step 2): its multi-turn
    /// marches resume, then `planner`'s orders are executed at once, one by
    /// one (moves, attacks, embarkations and sieges included). Invalid orders
    /// are dropped: the planner is advisory. Battles against the player are
    /// auto-resolved, with a notice in the season report.
    pub fn play_ai_turn<P>(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        planner: &P,
        events: &mut Vec<GameEvent>,
    ) where
        P: Fn(&CampaignState, &GameData, &FactionId) -> Vec<Order>,
    {
        if !self.factions.get(faction).is_some_and(|f| f.alive) {
            return;
        }
        self.ai_turn = Some(faction.clone());
        // CT1: both record the moves when the replay record is on.
        self.continue_ai_marches(data, faction, events);
        for order in planner(self, data, faction) {
            self.apply_ai_order(data, faction, order);
        }
        self.ai_turn = None;
    }

    /// Steps 3 and 4 of the sequential turn (spec § 3.4): the end-of-turn
    /// phases of spec § 1.3 (without a movement phase: marches and battles
    /// are immediate), then the new season, whose movement points are
    /// refilled before the player's marches resume. Public for probes that
    /// play each AI faction themselves ([`CampaignState::play_ai_turn`]).
    pub fn resolve_end_of_turn(&mut self, data: &GameData, events: &mut Vec<GameEvent>) {
        // Numbers below: the phases of spec § 1.3 (1, movement, is gone).
        // 2-4. Sieges, chevauchées.
        siege::resolve_sieges(self, data, events);
        siege::resolve_raids(self, data, events);

        // 5. Buildings and goods, ahead of the economy that reads them (M3).
        buildings::resolve_construction(self, data, events);
        economy::resolve_goods(self, data);
        // H3: diets whose requirements no longer hold fall back to the default.
        table::resolve_requirements(self, data, events);
        // Lot C4: edicts of a province no longer fully held fall back to the
        // default, and a change becomes active once its delay has elapsed.
        edicts::resolve_requirements(self, data, events);

        // 6-8. Economy, attrition, recovery.
        economy::resolve_economy(self, data, events);
        // C5: trade routes and agreements, after the treasury's tax income.
        crate::trade::resolve_trade(self, data, events);
        // H5: prices follow the coinage.
        crate::coinage::resolve_coinage(self, events);
        // H6: ransom installments, captive rulers.
        crate::ransom::resolve_ransoms(self, data, events);
        // H6: chivalric orders (collapse, new members, yearly prestige).
        crate::chivalry::resolve_chivalry(self, data, events);
        research::resolve_research(self, data, events);
        economy::resolve_attrition(self, data, events);
        // CV3: forced marches pay their supply, morale modifiers wear off.
        crate::posture::end_of_turn(self, data);
        economy::resolve_decay(self, data);
        // NV1: blockades, sea control, shipyards.
        crate::naval::resolve_season(self, data, events);

        // 8b. Diplomacy (vassal tribute after the economy, expiries,
        // rebellions) and religion (favour, Schism, heresy) before the
        // population reads their unrest (M5).
        // DP2: armies camping on the lands of a faction at peace without
        // right of passage (incident, casus belli), before the diplomacy.
        crate::passage::resolve_trespass(self, data, events);
        diplomacy::resolve_diplomacy(self, data, events);
        religion::resolve_religion(self, data, events);
        // 8b'. Agents: upkeep, counter-espionage, stale intelligence (C6).
        crate::agents::resolve_agents(self, data, events);

        // 8c. Chronicle: historical and random events (M10).
        chronicle::resolve_chronicle(self, data, events);

        // 8d. The table: ruler piety/prestige and Lent in spring (H3).
        table::resolve_lent(self, data, events);

        // 9. Population dynamics: growth, health, wealth, goods
        // satisfaction, unrest, revolt, plague, famine (M3).
        population::resolve_population(self, data, events);

        // 10. Characters: governance XP, deaths, winter births, regencies
        // (M4), then dead factions.
        dynasty::resolve_governance(self);
        dynasty::resolve_court_prestige(self, data);
        crate::retinue::resolve_retinue(self, data, events);
        characters::resolve_characters(self, data, events);
        dynasty::resolve_births(self, data, events);
        dynasty::resolve_regencies(self, data, events);
        characters::resolve_faction_deaths(self, data, events);
        // FE (F3): felony cases, generic victory streaks, objectives.
        crate::feudal::resolve_feudal(self, data, events);

        // 11. New season (step 4 of § 3.4): movement points are refilled,
        // then the player plays.
        self.advance_date();
        // CV3: forced marches end before the movement points are refilled.
        crate::posture::start_of_turn(self);
        let allowances: Vec<(crate::state::ArmyId, u32)> = self
            .armies
            .iter()
            .map(|(id, army)| (id.clone(), self.army_grid_allowance(data, army)))
            .collect();
        for (id, points) in allowances {
            if let Some(army) = self.armies.get_mut(&id) {
                army.movement_left = points;
            }
        }
        // CV3-3: encounter sites expire, new ones appear.
        crate::encounter::start_season(self, data, events);
        // The player's multi-turn marches resume at the start of
        // his turn.
        let player = self.player_faction.clone();
        march::continue_marches(self, data, Some(&player), events);
        // C6: agents get their points back and resume their march.
        crate::agents::start_season(self, data);
        let turn = self.turn;
        for faction in self.factions.values_mut() {
            faction.truces.retain(|_, until| *until > turn);
        }
        // M10: victory, defeat or end of the campaign for the player.
        crate::victory::resolve_victory(self, data, events);
        self.events = events.clone();
    }
}

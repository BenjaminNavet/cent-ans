//! Staging helpers for headless tests, screenshots and demos: they put armies
//! in a siege or a battle, or drop an encounter site, without going through
//! the normal campaign flow (SC CB10).

use data_model::{EncounterId, GameData, ProvinceId};

use crate::battle_request::BattleRequestError;
use crate::encounter::EncounterSite;
use crate::navigation::Cell;
use crate::state::{ArmyId, BattleRequest, CampaignState};

impl CampaignState {
    /// Debug helper for headless tests and screenshots: puts `army` in
    /// siege of the city of the enemy province `province` (moving it there,
    /// giving the town a garrison if it has none) and records a pending
    /// siege battle. Returns the battle index.
    pub fn debug_stage_siege(
        &mut self,
        data: &GameData,
        army: &ArmyId,
        province: &ProvinceId,
    ) -> Result<usize, BattleRequestError> {
        let index = self.pending_battles.len();
        let Some(a) = self.armies.get(army) else {
            return Err(BattleRequestError::Stale(index));
        };
        let faction = a.faction.clone();
        let Some(city) = self.province_city_id(province).cloned() else {
            return Err(BattleRequestError::Stale(index));
        };
        let controller = self.settlements[&city].controller.clone();
        if !self.is_at_war(&faction, &controller) {
            return Err(BattleRequestError::Stale(index));
        }
        Ok(self.stage_siege_at(data, army, &city, province))
    }

    /// SG2 demo: puts `army` in siege of the settlement drawn by landmark
    /// plan `landmark` (`data/landmarks/<id>.json`: Avignon, Bruges...),
    /// declaring war on its holder first when they are at peace (a demo
    /// campaign, thrown away after the battle). Returns the battle index.
    pub fn debug_stage_landmark_siege(
        &mut self,
        data: &GameData,
        army: &ArmyId,
        landmark: &str,
    ) -> Result<usize, BattleRequestError> {
        let index = self.pending_battles.len();
        let Some(faction) = self.armies.get(army).map(|a| a.faction.clone()) else {
            return Err(BattleRequestError::Stale(index));
        };
        let Some(city) = data
            .landmarks
            .values()
            .find(|l| l.id == landmark)
            .and_then(|l| data_model::SettlementId::new(l.settlement.as_str()).ok())
            .filter(|id| self.settlements.contains_key(id))
        else {
            return Err(BattleRequestError::Stale(index));
        };
        let province = self.settlements[&city].province.clone();
        let controller = self.settlements[&city].controller.clone();
        if controller == faction || !self.factions.contains_key(&controller) {
            return Err(BattleRequestError::Stale(index));
        }
        if !self.is_at_war(&faction, &controller) {
            self.start_war(&faction, &controller);
        }
        Ok(self.stage_siege_at(data, army, &city, &province))
    }

    /// Moves `army` before settlement `city` of `province`, gives the town a
    /// garrison if it has none, opens the siege and records the battle.
    fn stage_siege_at(
        &mut self,
        data: &GameData,
        army: &ArmyId,
        city: &data_model::SettlementId,
        province: &ProvinceId,
    ) -> usize {
        let index = self.pending_battles.len();
        let city = city.clone();
        let faction = self.armies[army].faction.clone();
        let army_state = self.armies.get_mut(army).expect("exists");
        army_state.position = crate::state::ArmyPosition::Settlement(city.clone());
        army_state.clear_plan();
        army_state.stance = crate::state::Stance::Siege;
        if let Some(general) = army_state.general.clone() {
            if let Some(c) = self.characters.get_mut(&general) {
                c.location = Some(province.clone());
            }
        }
        let walls = self.fortification_level(data, &city);
        let p = self.settlements.get_mut(&city).expect("exists");
        if p.garrison.is_empty() {
            if let Some(unit_type) = data
                .unit_types
                .values()
                .find(|t| t.id.as_str() == "unit_urban_militia")
            {
                for _ in 0..3 {
                    p.garrison.push(crate::state::Unit {
                        unit_type: unit_type.id.clone(),
                        strength: unit_type.soldiers,
                        max_strength: unit_type.soldiers,
                        morale: unit_type.stats.morale,
                        experience: 0,
                        levy_armor: 0,
                        levy_ranged: 0,
                        experience_residue: 0,
                    });
                }
            }
        }
        if p.siege.is_none() {
            p.siege = Some(crate::state::SiegeState {
                attacker: faction,
                turns_left: 4,
                turns_elapsed: 1,
                supplies: 80,
                breach: 0,
                started_turn: 0,
                engine_work: 0,
            });
        }
        // NT5 (N7): a staged siege brings its ladders and ram (the demos'
        // assault as before the engines were built on the spot), no tower.
        let staged_work: u32 = data
            .siege_engine_rules
            .engines
            .iter()
            .take_while(|e| e.kind != data_model::BuiltEngineKind::Tower)
            .map(|e| e.cost(data.siege_engine_rules.scaling_min_wall_level, walls))
            .sum();
        if let Some(siege) = p.siege.as_mut() {
            siege.engine_work = siege.engine_work.max(staged_work);
        }
        self.pending_battles.push(BattleRequest {
            attacker: army.clone(),
            defender: army.clone(),
            location: city,
            province: province.clone(),
            siege: true,
            opening: Default::default(),
        });
        index
    }

    /// Debug helper for headless tests and screenshots: moves `defender` to
    /// `attacker`'s position and records a pending battle between them
    /// (they must be at war; the attacker must stand in a settlement).
    /// Returns the battle index.
    pub fn debug_stage_battle(
        &mut self,
        attacker: &ArmyId,
        defender: &ArmyId,
    ) -> Result<usize, BattleRequestError> {
        let index = self.pending_battles.len();
        let (Some(a), Some(d)) = (self.armies.get(attacker), self.armies.get(defender)) else {
            return Err(BattleRequestError::Stale(index));
        };
        if !self.is_at_war(&a.faction, &d.faction) {
            return Err(BattleRequestError::Stale(index));
        }
        let Some(location) = a.settlement().cloned() else {
            return Err(BattleRequestError::Stale(index));
        };
        let Some(province) = self.settlement_province(&location).cloned() else {
            return Err(BattleRequestError::Stale(index));
        };
        let army = self.armies.get_mut(defender).expect("exists");
        army.position = crate::state::ArmyPosition::Settlement(location.clone());
        army.clear_plan();
        if let Some(general) = army.general.clone() {
            if let Some(c) = self.characters.get_mut(&general) {
                c.location = Some(province.clone());
            }
        }
        self.pending_battles.push(BattleRequest {
            attacker: attacker.clone(),
            defender: defender.clone(),
            location,
            province,
            siege: false,
            opening: Default::default(),
        });
        Ok(index)
    }

    /// Puts a site of `encounter` on the cell of `point` (map pixels), lasting
    /// three seasons; returns its id, or `None` for an unknown encounter or a
    /// point outside every province. Staging only: no spawn rule applies.
    pub fn debug_put_encounter_site(
        &mut self,
        data: &GameData,
        encounter: &EncounterId,
        point: [f32; 2],
    ) -> Option<u32> {
        if !data.encounters.contains_key(encounter) {
            return None;
        }
        let province = data.province_at_point(point[0], point[1])?.clone();
        let cell = Cell::of_point(data.navgrid(), point);
        let id = self.encounters.next_site_id;
        self.encounters.next_site_id += 1;
        self.encounters.sites.push(EncounterSite {
            id,
            encounter: encounter.clone(),
            cell,
            province,
            expires_turn: self.turn + 3,
            claimed_by: None,
        });
        Some(id)
    }
}

//! Army orders: raise, merge, split, garrison, disband, assign a general.

use data_model::{CharacterId, CharacterStatus, FactionId, GameData, ProvinceId, SettlementId};

use super::OrderError;
use crate::state::{Army, ArmyId, ArmyPosition, CampaignState, Unit};

/// NT5 (N6): units `army` may still receive under `armies.json`
/// `max_units` (0 when full or unknown).
pub fn army_room(state: &CampaignState, data: &GameData, army: &ArmyId) -> usize {
    state
        .armies
        .get(army)
        .map_or(0, |a| data.army_rules.cap().saturating_sub(a.units.len()))
}

impl CampaignState {
    pub(super) fn order_create_army(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        indices: &[usize],
        general: Option<CharacterId>,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        // A friendly army on the place would lift the siege for free
        // (`resolve_sieges`): the garrison cannot march out while besieged.
        if self.settlements[settlement].siege.is_some() {
            return Err(OrderError::SettlementBesieged);
        }
        let garrison_len = self.settlements[settlement].garrison.len();
        let indices = unique_sorted(indices, garrison_len)?;
        // NT5 (N6): no army above the unit cap.
        let cap = data.army_rules.cap();
        if indices.len() > cap {
            return Err(OrderError::ArmyFull { cap });
        }
        let province = self.settlements[settlement].province.clone();
        if let Some(character) = &general {
            self.check_general(faction, character, &province)?;
        }
        let units = take_indices(
            &mut self
                .settlements
                .get_mut(settlement)
                .expect("checked")
                .garrison,
            &indices,
        );
        let id = self.allocate_army_id();
        self.armies.insert(
            id.clone(),
            Army::new(
                faction.clone(),
                ArmyPosition::Settlement(settlement.clone()),
                units,
            ),
        );
        if let Some(character) = general {
            self.attach_general(&id, &character);
        }
        // F1: siege trains and movement effects set the pace from the start.
        let allowance = self.army_grid_allowance(data, &self.armies[&id]);
        self.armies
            .get_mut(&id)
            .expect("just created")
            .movement_left = allowance;
        Ok(())
    }

    pub(super) fn order_merge(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        source: &ArmyId,
        target: &ArmyId,
    ) -> Result<(), OrderError> {
        let source_army = self.own_army(faction, source)?;
        let target_army = self.own_army(faction, target)?;
        if source == target {
            return Err(OrderError::NotSameProvince);
        }
        // Lot M2: the same settlement, or two armies within reach of an
        // engagement.
        if !self.armies_together(data, source_army, target_army) {
            return Err(OrderError::NotSameProvince);
        }
        // NT5 (N6): the merged army stays within the unit cap.
        let cap = data.army_rules.cap();
        if source_army.units.len() + target_army.units.len() > cap {
            return Err(OrderError::ArmyFull { cap });
        }
        let source_army = self.armies.remove(source).expect("checked");
        let general = source_army.general.clone();
        let target_mut = self.armies.get_mut(target).expect("checked");
        target_mut.units.extend(source_army.units);
        target_mut.movement_left = target_mut.movement_left.min(source_army.movement_left);
        target_mut.clear_plan();
        if target_mut.general.is_none() {
            if let Some(general) = general {
                self.attach_general(target, &general);
            }
        } else if let Some(general) = general {
            self.detach_general(&general);
        }
        Ok(())
    }

    pub(super) fn order_split(
        &mut self,
        faction: &FactionId,
        army_id: &ArmyId,
        indices: &[usize],
    ) -> Result<(), OrderError> {
        let army = self.own_army(faction, army_id)?;
        let indices = unique_sorted(indices, army.units.len())?;
        if indices.len() == army.units.len() {
            return Err(OrderError::WouldEmptyArmy);
        }
        let units = take_indices(
            &mut self.armies.get_mut(army_id).expect("checked").units,
            &indices,
        );
        // The detachment inherits the rest of the army, without a general
        // or a plan, and (TW2-T5) without traditions.
        let template = self.armies[army_id].clone();
        let id = self.allocate_army_id();
        self.armies.insert(
            id,
            Army {
                general: None,
                units,
                planned_path: Vec::new(),
                destination: None,
                traditions: Default::default(),
                ..template
            },
        );
        Ok(())
    }

    /// Lot C7a: units of an army become the garrison of the place it holds.
    pub(super) fn order_garrison(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army_id: &ArmyId,
        indices: &[usize],
    ) -> Result<(), OrderError> {
        let army = self.own_army(faction, army_id)?;
        let location = army
            .settlement()
            .cloned()
            .ok_or(OrderError::NotInSettlement)?;
        let indices = unique_sorted(indices, army.units.len())?;
        self.own_settlement(faction, &location)?;
        let settlement = &self.settlements[&location];
        if settlement.siege.is_some() {
            return Err(OrderError::SettlementBesieged);
        }
        if let Some(cap) = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.garrison_cap.get(&settlement.kind))
        {
            if settlement.garrison.len() + indices.len() > *cap {
                return Err(OrderError::GarrisonFull { cap: *cap });
            }
        }
        let units = take_indices(
            &mut self.armies.get_mut(army_id).expect("checked").units,
            &indices,
        );
        self.settlements
            .get_mut(&location)
            .expect("checked")
            .garrison
            .extend(units);
        if self.armies[army_id].units.is_empty() {
            if let Some(general) = self.armies[army_id].general.clone() {
                self.detach_general(&general);
            }
            self.armies.remove(army_id);
        }
        Ok(())
    }

    pub(super) fn order_disband(
        &mut self,
        faction: &FactionId,
        army: Option<&ArmyId>,
        settlement: Option<&SettlementId>,
        unit_index: usize,
    ) -> Result<(), OrderError> {
        match (army, settlement) {
            (Some(army_id), None) => {
                let army = self.own_army(faction, army_id)?;
                if unit_index >= army.units.len() {
                    return Err(OrderError::InvalidUnitIndex(unit_index));
                }
                if army.units.len() == 1 {
                    // Dismissing the last unit disbands the army (M10), unless
                    // it is about to fight.
                    let fighting = self
                        .pending_battles
                        .iter()
                        .any(|b| &b.attacker == army_id || &b.defender == army_id);
                    if fighting {
                        return Err(OrderError::WouldEmptyArmy);
                    }
                    if let Some(general) = army.general.clone() {
                        self.detach_general(&general);
                    }
                    self.armies.remove(army_id);
                    return Ok(());
                }
                self.armies
                    .get_mut(army_id)
                    .expect("checked")
                    .units
                    .remove(unit_index);
                Ok(())
            }
            (None, Some(settlement_id)) => {
                self.own_settlement(faction, settlement_id)?;
                let garrison = &mut self
                    .settlements
                    .get_mut(settlement_id)
                    .expect("checked")
                    .garrison;
                if unit_index >= garrison.len() {
                    return Err(OrderError::InvalidUnitIndex(unit_index));
                }
                garrison.remove(unit_index);
                Ok(())
            }
            _ => Err(OrderError::AmbiguousTarget),
        }
    }

    pub(super) fn order_assign_general(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army_id: &ArmyId,
        character: &CharacterId,
    ) -> Result<(), OrderError> {
        let army = self.own_army(faction, army_id)?;
        let location = self
            .army_province(data, army)
            .ok_or(OrderError::NotInSettlement)?;
        self.check_general(faction, character, &location)?;
        if let Some(previous) = self.armies[army_id].general.clone() {
            self.detach_general(&previous);
        }
        self.attach_general(army_id, character);
        Ok(())
    }

    fn check_general(
        &self,
        faction: &FactionId,
        character: &CharacterId,
        province: &ProvinceId,
    ) -> Result<(), OrderError> {
        let state = self
            .characters
            .get(character)
            .ok_or_else(|| OrderError::UnknownCharacter(character.clone()))?;
        if !state.alive || state.captive || &state.faction != faction || !state.is_major(self.year)
        {
            return Err(OrderError::CharacterUnavailable);
        }
        if state.governor_of.is_some() {
            return Err(OrderError::AlreadyGoverning);
        }
        let in_province = state.location.as_ref() == Some(province)
            || state
                .army
                .as_ref()
                .and_then(|a| self.armies.get(a))
                .and_then(|a| a.settlement())
                .is_some_and(|s| self.settlement_province(s) == Some(province));
        if !in_province {
            return Err(OrderError::CharacterElsewhere);
        }
        Ok(())
    }

    /// Makes `character` the general of `army`, detaching it from its previous army.
    pub(crate) fn attach_general(&mut self, army_id: &ArmyId, character: &CharacterId) {
        if let Some(previous_army) = self.characters.get(character).and_then(|c| c.army.clone()) {
            if let Some(previous) = self.armies.get_mut(&previous_army) {
                if previous.general.as_ref() == Some(character) {
                    previous.general = None;
                }
            }
        }
        if let Some(army) = self.armies.get_mut(army_id) {
            army.general = Some(character.clone());
            // A field army's province is kept up to date by
            // `movement::move_general` (it needs the province raster).
            let location = army
                .settlement()
                .and_then(|id| self.settlements.get(id))
                .map(|s| s.province.clone());
            if let Some(state) = self.characters.get_mut(character) {
                state.army = Some(army_id.clone());
                if location.is_some() {
                    state.location = location;
                }
            }
        }
    }

    /// Removes `character` from the army it commands (stays in the province).
    pub(crate) fn detach_general(&mut self, character: &CharacterId) {
        let Some(state) = self.characters.get_mut(character) else {
            return;
        };
        if let Some(army_id) = state.army.take() {
            if let Some(province) = self
                .armies
                .get(&army_id)
                .and_then(|army| army.settlement())
                .and_then(|id| self.settlements.get(id))
            {
                state.location = Some(province.province.clone());
            }
            if let Some(army) = self.armies.get_mut(&army_id) {
                if army.general.as_ref() == Some(character) {
                    army.general = None;
                }
            }
        }
    }
}

/// `true` when a character with this static status may command at the start.
pub(crate) fn status_allows_command(status: Option<CharacterStatus>) -> bool {
    !matches!(
        status,
        Some(CharacterStatus::Minor) | Some(CharacterStatus::Captive)
    )
}

fn unique_sorted(indices: &[usize], len: usize) -> Result<Vec<usize>, OrderError> {
    if indices.is_empty() {
        return Err(OrderError::NoUnitsSelected);
    }
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if let Some(&bad) = sorted.iter().find(|&&i| i >= len) {
        return Err(OrderError::InvalidUnitIndex(bad));
    }
    Ok(sorted)
}

/// Removes the units at `sorted_indices` (ascending, unique) and returns them in order.
fn take_indices(units: &mut Vec<Unit>, sorted_indices: &[usize]) -> Vec<Unit> {
    let mut taken = Vec::with_capacity(sorted_indices.len());
    for &index in sorted_indices.iter().rev() {
        taken.push(units.remove(index));
    }
    taken.reverse();
    taken
}

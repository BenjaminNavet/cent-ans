//! Validation of chronicle events (M10): structural checks are errors,
//! unknown ids are warnings (the simulation ignores such effects and
//! conditions, spec `m10-events.md` § 2).

use std::collections::BTreeMap;
use std::fmt;

use crate::entities::event::{
    CharacterRef, Condition, EventCategory, EventEffect, EventScope, ProvinceRef,
};
use crate::entities::faction::ClaimKind;
use crate::ids::{CharacterId, FactionId, ProvinceId};
use crate::load::{DataError, GameData, Warning};

/// Checks every loaded event.
pub(crate) fn validate_events(
    data: &GameData,
    warnings: &mut Vec<Warning>,
) -> Result<(), DataError> {
    for (id, event) in &data.events {
        let invalid = |message: &str| DataError::InvalidEvent {
            id: id.to_string(),
            message: message.to_owned(),
        };
        if event.options.is_empty() || event.options.len() > 3 {
            return Err(invalid("an event needs 1 to 3 options"));
        }
        match event.kind {
            EventCategory::Historical if event.trigger.date.is_none() => {
                return Err(invalid("a historical event needs trigger.date"));
            }
            EventCategory::Random
                if event.trigger.mean_time_to_happen.is_none()
                    && event.trigger.chance_permille.is_none() =>
            {
                return Err(invalid(
                    "a random event needs trigger.mean_time_to_happen or chance_permille",
                ));
            }
            _ => {}
        }
        if event.trigger.mean_time_to_happen == Some(0) {
            return Err(invalid("mean_time_to_happen must be positive"));
        }
        for option in &event.options {
            for effect in &option.effects {
                if let EventEffect::ScheduleEvent {
                    event: target,
                    delay,
                } = effect
                {
                    if *delay == 0 {
                        return Err(invalid("schedule_event.delay must be at least 1 turn"));
                    }
                    if target == id {
                        return Err(invalid("an event cannot schedule itself"));
                    }
                }
            }
        }
        if event.kind == EventCategory::Chained && !is_scheduled(data, id) {
            warnings.push(Warning {
                entity: id.to_string(),
                field: "kind".to_owned(),
                message: "chained event never scheduled by another event (never fires)".to_owned(),
            });
        }
        let mut refs = EventRefs {
            data,
            entity: id.to_string(),
            warnings: &mut *warnings,
        };
        match &event.scope {
            EventScope::Global => {}
            EventScope::Faction { faction } => refs.faction("scope.faction", faction.as_ref()),
            EventScope::Province { faction, province } => {
                refs.faction("scope.faction", faction.as_ref());
                refs.province("scope.province", province.as_ref());
            }
        }
        for condition in &event.trigger.conditions {
            refs.condition(condition);
        }
        for option in &event.options {
            for effect in &option.effects {
                refs.effect(effect);
            }
        }
    }
    Ok(())
}

/// `true` when some event option schedules `target` (F1 chains).
fn is_scheduled(data: &GameData, target: &crate::ids::EventId) -> bool {
    data.events
        .values()
        .flat_map(|e| e.options.iter())
        .flat_map(|o| o.effects.iter())
        .any(|effect| matches!(effect, EventEffect::ScheduleEvent { event, .. } if event == target))
}

/// Reports unknown ids referenced by an event as warnings.
struct EventRefs<'a> {
    data: &'a GameData,
    entity: String,
    warnings: &'a mut Vec<Warning>,
}

impl EventRefs<'_> {
    fn warn(&mut self, field: &str, target: &dyn fmt::Display) {
        self.warnings.push(Warning {
            entity: self.entity.clone(),
            field: field.to_owned(),
            message: format!("unknown id {target} (ignored by the simulation)"),
        });
    }

    fn check<K: Ord + fmt::Display, V>(
        &mut self,
        field: &str,
        target: Option<&K>,
        known: &BTreeMap<K, V>,
    ) {
        if let Some(target) = target {
            if !known.contains_key(target) {
                self.warn(field, target);
            }
        }
    }

    fn faction(&mut self, field: &str, id: Option<&FactionId>) {
        let known = &self.data.factions;
        self.check(field, id, known);
    }

    fn province(&mut self, field: &str, id: Option<&ProvinceId>) {
        let known = &self.data.provinces;
        self.check(field, id, known);
    }

    fn character(&mut self, field: &str, id: Option<&CharacterId>) {
        let known = &self.data.characters;
        self.check(field, id, known);
    }

    fn character_ref(&mut self, field: &str, id: &CharacterRef) {
        if let CharacterRef::Id(id) = id {
            self.character(field, Some(id));
        }
    }

    fn province_ref(&mut self, field: &str, id: Option<&ProvinceRef>) {
        if let Some(ProvinceRef::Id(id)) = id {
            self.province(field, Some(id));
        }
    }

    fn condition(&mut self, condition: &Condition) {
        let data = self.data;
        match condition {
            Condition::FactionExists { faction } | Condition::FactionIsPlayer { faction } => {
                self.faction("conditions.faction", Some(faction));
            }
            Condition::AtWar { a, b } => {
                self.faction("conditions.a", a.as_ref());
                self.faction("conditions.b", b.as_ref());
            }
            Condition::Controls { faction, province } => {
                self.faction("conditions.faction", faction.as_ref());
                self.province("conditions.province", Some(province));
            }
            Condition::CharacterAlive { id } | Condition::CharacterCaptive { id } => {
                self.character("conditions.id", Some(id));
            }
            Condition::RulerIs { faction, character } => {
                self.faction("conditions.faction", Some(faction));
                self.character("conditions.character", Some(character));
            }
            Condition::RulerTrait { faction, trait_id } => {
                self.faction("conditions.faction", faction.as_ref());
                self.check("conditions.trait", Some(trait_id), &data.traits);
            }
            Condition::RulerHouse { faction, .. }
            | Condition::RulerAgeBetween { faction, .. }
            | Condition::TreasuryAbove { faction, .. }
            | Condition::ProvincesBelow { faction, .. } => {
                self.faction("conditions.faction", faction.as_ref());
            }
            Condition::ProvinceUnrestAbove { province, .. }
            | Condition::ProvinceCoastal { province } => {
                self.province("conditions.province", province.as_ref());
            }
            Condition::ProvinceBesieged { province, by } => {
                self.province("conditions.province", province.as_ref());
                self.faction("conditions.by", by.as_ref());
            }
            Condition::ReligionIs { faction, religion } => {
                self.faction("conditions.faction", faction.as_ref());
                self.check("conditions.religion", Some(religion), &data.religions);
            }
            Condition::NotFired { event } | Condition::Fired { event } => {
                self.check("conditions.event", Some(event), &data.events);
            }
            Condition::AnyOf { conditions } => {
                for inner in conditions {
                    self.condition(inner);
                }
            }
            Condition::YearBetween { .. } | Condition::Schism { .. } | Condition::Season { .. } => {
            }
        }
    }

    fn effect(&mut self, effect: &EventEffect) {
        let data = self.data;
        match effect {
            EventEffect::Treasury { faction, .. } | EventEffect::PapalFavor { faction, .. } => {
                self.faction("effects.faction", faction.as_ref());
            }
            EventEffect::Unrest { province, .. }
            | EventEffect::Population { province, .. }
            | EventEffect::Health { province, .. }
            | EventEffect::Wealth { province, .. }
            | EventEffect::Devastation { province, .. } => {
                self.province_ref("effects.province", province.as_ref());
            }
            EventEffect::Prestige {
                character, faction, ..
            }
            | EventEffect::Piety {
                character, faction, ..
            }
            | EventEffect::KillCharacter {
                id: character,
                faction,
            } => {
                self.character_ref("effects.character", character);
                self.faction("effects.faction", faction.as_ref());
            }
            EventEffect::Opinion {
                faction, towards, ..
            } => {
                self.faction("effects.faction", Some(faction));
                self.faction("effects.towards", towards.as_ref());
            }
            EventEffect::DeclareWar { a, b } | EventEffect::Peace { a, b } => {
                self.faction("effects.a", Some(a));
                self.faction("effects.b", Some(b));
            }
            EventEffect::AddTrait {
                character,
                faction,
                trait_id,
            } => {
                self.character_ref("effects.character", character);
                self.faction("effects.faction", faction.as_ref());
                self.check("effects.trait", Some(trait_id), &data.traits);
            }
            EventEffect::SpawnArmy {
                faction,
                province,
                units,
            } => {
                self.faction("effects.faction", faction.as_ref());
                self.province("effects.province", province.as_ref());
                for unit in units {
                    self.check("effects.units", Some(unit), &data.unit_types);
                }
            }
            EventEffect::Claim {
                faction,
                kind,
                target,
            } => {
                self.faction("effects.faction", faction.as_ref());
                let known = match kind {
                    ClaimKind::Throne => FactionId::new(target.clone())
                        .is_ok_and(|id| data.factions.contains_key(&id)),
                    ClaimKind::Province => ProvinceId::new(target.clone())
                        .is_ok_and(|id| data.provinces.contains_key(&id)),
                };
                if !known {
                    self.warn("effects.target", target);
                }
            }
            EventEffect::Loyalty { vassal, .. } => {
                self.faction("effects.vassal", vassal.as_ref());
            }
            EventEffect::PlagueWave { .. } => {}
            EventEffect::CaptureCharacter {
                id,
                faction,
                captor,
            } => {
                self.character_ref("effects.id", id);
                self.faction("effects.faction", faction.as_ref());
                self.faction("effects.captor", Some(captor));
            }
            EventEffect::ReleaseCharacter { id, faction, .. } => {
                self.character_ref("effects.id", id);
                self.faction("effects.faction", faction.as_ref());
            }
            EventEffect::ScheduleEvent { event, .. } => {
                self.check("effects.event", Some(event), &data.events);
            }
            EventEffect::Marry { a, b } => {
                self.character("effects.a", Some(a));
                self.character("effects.b", Some(b));
            }
            EventEffect::FoundChivalricOrder { order, faction } => {
                self.check("effects.order", Some(order), &data.chivalric_orders);
                self.faction("effects.faction", faction.as_ref());
            }
            EventEffect::TransferProvince {
                province,
                faction,
                from,
                payer,
                ..
            } => {
                self.province("effects.province", Some(province));
                self.faction("effects.faction", faction.as_ref());
                self.faction("effects.from", from.as_ref());
                self.faction("effects.payer", payer.as_ref());
            }
            EventEffect::TransferTitle {
                title,
                faction,
                from,
                payer,
                ..
            } => {
                self.check("effects.title", Some(title), &data.titles);
                self.faction("effects.faction", faction.as_ref());
                self.faction("effects.from", from.as_ref());
                self.faction("effects.payer", payer.as_ref());
            }
            EventEffect::SetRuler { character, faction } => {
                self.character("effects.character", Some(character));
                self.faction("effects.faction", faction.as_ref());
            }
        }
    }
}

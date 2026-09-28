//! Lot TW2-T2 (ADR 0102): seasonal replenishment of field armies.
//!
//! Every season, an army that did not fight regains a share of its missing
//! men (`data/rules/replenishment.json`, `replenishment`):
//!
//! - base rate by territory: own > allied > neutral > hostile (0);
//! - multiplied by the stance (entrenched camp more, forced march nothing,
//!   siege and raid less), or by the friendly settlement it stands in;
//! - halved in winter, cut on low supply;
//! - raised by the general's stewardship (governance level, skills, traits)
//!   and, on its own lands, by the military buildings of the province;
//! - capped, and paid in livres per man regained (a share of the per-man
//!   recruitment price). An empty treasury replenishes nothing; a short one
//!   replenishes what it can pay for.
//!
//! Units wiped out are removed after their battle and never come back.
//! Garrisons keep their own reinforcement (`economy`, `garrison` effect).

use data_model::{FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState, Season, Stance};

/// Territory an army stands on, for its replenishment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Territory {
    Own,
    Ally,
    Neutral,
    Hostile,
}

impl Territory {
    pub fn key(self) -> &'static str {
        match self {
            Territory::Own => "own",
            Territory::Ally => "ally",
            Territory::Neutral => "neutral",
            Territory::Hostile => "hostile",
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            Territory::Own => "Terres propres",
            Territory::Ally => "Terres alliées",
            Territory::Neutral => "Terres neutres",
            Territory::Hostile => "Terres ennemies",
        }
    }
}

/// How a factor acts on the rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactorKind {
    /// Base rate, percent of the missing men.
    Base,
    /// Multiplier, percent.
    Multiplier,
    /// Bonus or malus, percent of the rate.
    Bonus,
}

/// One line of the replenishment tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplenishFactor {
    pub kind: FactorKind,
    /// French label.
    pub label: String,
    pub percent: i32,
}

impl ReplenishFactor {
    /// French line: « Terres propres : 20 % », « Hiver : ×50 % », « Chef : +21 % ».
    pub fn text_fr(&self) -> String {
        match self.kind {
            FactorKind::Base => format!("{} : {} %", self.label, self.percent),
            FactorKind::Multiplier => format!("{} : ×{} %", self.label, self.percent),
            FactorKind::Bonus => format!("{} : {:+} %", self.label, self.percent),
        }
    }
}

/// What an army would regain at the end of this season.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReplenishPreview {
    /// Final rate, in hundredths of a percent of the missing men.
    pub rate_bp: u32,
    /// Men missing in the surviving units.
    pub missing: u32,
    /// Men regained (after the treasury cap).
    pub men: u32,
    /// Livres paid.
    pub cost: u32,
    /// Men regained per unit, in the army's unit order.
    pub per_unit: Vec<u32>,
    pub factors: Vec<ReplenishFactor>,
    /// French reason when nothing comes back (fought, hostile lands, empty
    /// treasury, forced march...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
}

impl ReplenishPreview {
    /// Rate rounded to whole percent, for the UI.
    pub fn percent(&self) -> u32 {
        (self.rate_bp + 50) / 100
    }
}

impl CampaignState {
    /// Territory of `army_id` for its replenishment: its own settlement or
    /// province counts as own, an allied (or open by treaty) one as allied,
    /// a faction at war as hostile, the rest as neutral.
    pub fn army_territory(&self, data: &GameData, army_id: &ArmyId) -> Territory {
        let Some(army) = self.armies.get(army_id) else {
            return Territory::Neutral;
        };
        let faction = &army.faction;
        if let Some(settlement) = army.settlement().and_then(|s| self.settlements.get(s)) {
            if &settlement.controller == faction {
                return Territory::Own;
            }
            if self.is_allied(faction, &settlement.controller) {
                return Territory::Ally;
            }
        }
        let Some(province) = self.army_province(data, army) else {
            return Territory::Neutral;
        };
        match self.province_controller(&province) {
            Some(c) if c == faction => Territory::Own,
            _ if self.is_hostile_territory(faction, &province) => Territory::Hostile,
            _ if self.is_friendly_territory(faction, &province) => Territory::Ally,
            _ => Territory::Neutral,
        }
    }

    /// Replenishment `army_id` would get at the end of this season, with the
    /// factors of its rate (army sheet tooltip). `None` for an unknown army.
    pub fn army_replenishment(
        &self,
        data: &GameData,
        army_id: &ArmyId,
    ) -> Option<ReplenishPreview> {
        let treasury = self
            .armies
            .get(army_id)
            .and_then(|a| self.factions.get(&a.faction))
            .map_or(0, |f| f.treasury);
        self.plan_replenishment(data, army_id, treasury)
    }

    /// [`CampaignState::army_replenishment`] against `treasury` livres.
    fn plan_replenishment(
        &self,
        data: &GameData,
        army_id: &ArmyId,
        treasury: i64,
    ) -> Option<ReplenishPreview> {
        let army = self.armies.get(army_id)?;
        let rules = &data.replenishment_rules.replenishment;
        let mut preview = ReplenishPreview {
            missing: army
                .units
                .iter()
                .map(|u| u.max_strength.saturating_sub(u.strength))
                .sum(),
            per_unit: vec![0; army.units.len()],
            ..Default::default()
        };
        let territory = self.army_territory(data, army_id);
        let base = match territory {
            Territory::Own => rules.territory_percent.own,
            Territory::Ally => rules.territory_percent.ally,
            Territory::Neutral => rules.territory_percent.neutral,
            Territory::Hostile => rules.territory_percent.hostile,
        };
        preview.factors.push(ReplenishFactor {
            kind: FactorKind::Base,
            label: territory.label_fr().to_owned(),
            percent: base as i32,
        });
        // Multipliers, percent.
        let mut multiplier: u64 = 100;
        let mut multiply = |factors: &mut Vec<ReplenishFactor>, label: &str, percent: u32| {
            if percent != 100 {
                factors.push(ReplenishFactor {
                    kind: FactorKind::Multiplier,
                    label: label.to_owned(),
                    percent: percent as i32,
                });
            }
            multiplier = multiplier * u64::from(percent) / 100;
        };
        let stance = stance_percent(rules, army.stance);
        let friendly_settlement = army
            .settlement()
            .is_some_and(|s| self.is_friendly_settlement(&army.faction, s));
        if friendly_settlement && army.stance != Stance::ForcedMarch {
            let percent = stance.max(rules.in_settlement_percent);
            multiply(&mut preview.factors, "Dans une place amie", percent);
        } else {
            let label = format!("Posture « {} »", stance_label_fr(army.stance));
            multiply(&mut preview.factors, &label, stance);
        }
        if self.season == Season::Winter {
            multiply(&mut preview.factors, "Hiver", rules.winter_percent);
        }
        if army.supply < rules.low_supply_threshold {
            multiply(&mut preview.factors, "Vivres bas", rules.low_supply_percent);
        }
        // Bonuses, percent of the rate.
        let mut bonus = 0i32;
        let general = army.general.as_ref().and_then(|id| self.characters.get(id));
        if let Some(general) = general {
            let mut stewardship =
                i32::from(general.skills.governance) * rules.general_governance_percent_per_level;
            stewardship += general
                .skills_learned
                .iter()
                .filter_map(|s| rules.general_skill_percent.get(s.as_str()))
                .sum::<i32>();
            stewardship += general
                .traits
                .iter()
                .filter_map(|t| rules.general_trait_percent.get(t.as_str()))
                .sum::<i32>();
            if stewardship != 0 {
                preview.factors.push(ReplenishFactor {
                    kind: FactorKind::Bonus,
                    label: "Intendance du chef".to_owned(),
                    percent: stewardship,
                });
            }
            bonus += stewardship;
        }
        if territory == Territory::Own {
            if let Some(province) = self.army_province(data, army) {
                let effects = self.province_building_effects(data, &province);
                let buildings = (effects.garrison.flat.max(0.0)
                    * f64::from(rules.building_garrison_point_percent)
                    + effects.recruit_slots.flat.max(0.0)
                        * f64::from(rules.building_recruit_slot_percent))
                .round() as i32;
                let buildings = buildings.min(rules.building_bonus_max_percent);
                if buildings > 0 {
                    preview.factors.push(ReplenishFactor {
                        kind: FactorKind::Bonus,
                        label: "Bâtiments de la province".to_owned(),
                        percent: buildings,
                    });
                }
                bonus += buildings;
            }
        }
        let bonus = bonus.max(-90);
        // Final rate in hundredths of a percent.
        let rate_bp = u64::from(base) * multiplier * (100 + bonus) as u64 / 100;
        let rate_bp = (rate_bp as u32).min(rules.max_percent * 100);
        preview.rate_bp = rate_bp;

        if army.fought_turn == Some(self.turn) {
            preview.blocked = Some("a combattu cette saison".to_owned());
        } else if rate_bp == 0 {
            preview.blocked = Some(if territory == Territory::Hostile {
                "en terre ennemie".to_owned()
            } else if army.stance == Stance::ForcedMarch {
                "en marche forcée".to_owned()
            } else {
                "aucun renfort possible".to_owned()
            });
        } else if preview.missing == 0 {
            preview.blocked = Some("effectifs au complet".to_owned());
        } else if treasury <= 0 {
            preview.blocked = Some("trésor vide".to_owned());
        }
        if preview.blocked.is_some() {
            return Some(preview);
        }

        // Men per unit, then the price each, within the treasury.
        let cost_percent = u64::from(rules.cost_percent)
            * u64::from(self.difficulty_recruit_percent(data, &army.faction))
            / 100;
        let mut left = treasury.max(0) as u64;
        for (index, unit) in army.units.iter().enumerate() {
            let missing = unit.max_strength.saturating_sub(unit.strength);
            if missing == 0 || unit.strength == 0 {
                continue;
            }
            let mut men = (u64::from(missing) * u64::from(rate_bp) + 5_000) / 10_000;
            let (money, soldiers) = data.unit_types.get(&unit.unit_type).map_or((0, 1), |t| {
                (u64::from(t.cost.money), u64::from(t.soldiers.max(1)))
            });
            // Price of `men` in livres, rounded up.
            let price = |men: u64| (men * money * cost_percent).div_ceil(soldiers * 100);
            if price(men) > left {
                // What the treasury still pays for.
                men = (left * soldiers * 100)
                    .checked_div(money * cost_percent)
                    .unwrap_or(men)
                    .min(men);
            }
            let cost = price(men);
            left -= cost.min(left);
            preview.per_unit[index] = men as u32;
            preview.men += men as u32;
            preview.cost += cost as u32;
        }
        if preview.men == 0 && preview.blocked.is_none() {
            preview.blocked = Some(if left == 0 {
                "trésor insuffisant".to_owned()
            } else {
                "trop peu d'hommes manquants".to_owned()
            });
        }
        Some(preview)
    }
}

fn stance_percent(rules: &data_model::ArmyReplenishmentRules, stance: Stance) -> u32 {
    let p = &rules.stance_percent;
    match stance {
        Stance::Normal => p.normal,
        Stance::Raid => p.raid,
        Stance::Siege => p.siege,
        Stance::Ambush => p.ambush,
        Stance::ForcedMarch => p.forced_march,
        Stance::Entrenched => p.entrenched,
    }
}

fn stance_label_fr(stance: Stance) -> &'static str {
    match stance {
        Stance::Normal => "en marche",
        Stance::Raid => "chevauchée",
        Stance::Siege => "siège",
        Stance::Ambush => "embuscade",
        Stance::ForcedMarch => "marche forcée",
        Stance::Entrenched => "camp retranché",
    }
}

/// Marks `army_id` as having fought this turn (no replenishment this season).
pub fn mark_fought(state: &mut CampaignState, army_id: &ArmyId) {
    let turn = state.turn;
    if let Some(army) = state.armies.get_mut(army_id) {
        army.fought_turn = Some(turn);
    }
}

/// End of the season: every army replenishes and pays for it, in id order
/// (the treasury of a faction serves its armies one after the other). The
/// player gets one summary line in the season report.
pub fn resolve_replenishment(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ArmyId> = state.armies.keys().cloned().collect();
    let player = state.player_faction.clone();
    let mut player_men = 0u32;
    let mut player_cost = 0u32;
    let mut player_armies = 0u32;
    for id in ids {
        let Some(faction) = state.armies.get(&id).map(|a| a.faction.clone()) else {
            continue;
        };
        let treasury = state.factions.get(&faction).map_or(0, |f| f.treasury);
        let Some(plan) = state.plan_replenishment(data, &id, treasury) else {
            continue;
        };
        if plan.men == 0 {
            continue;
        }
        apply_plan(state, &id, &faction, &plan);
        if faction == player {
            player_men += plan.men;
            player_cost += plan.cost;
            player_armies += 1;
        }
    }
    if player_men > 0 {
        let armies = if player_armies == 1 {
            "Une armée regagne".to_owned()
        } else {
            format!("{player_armies} armées regagnent")
        };
        events.push(
            GameEvent::new(
                EventKind::Recruited,
                format!("Reconstitution : {armies} {player_men} hommes ({player_cost} livres)."),
            )
            .faction(&player),
        );
    }
}

fn apply_plan(
    state: &mut CampaignState,
    army_id: &ArmyId,
    faction: &FactionId,
    plan: &ReplenishPreview,
) {
    if let Some(army) = state.armies.get_mut(army_id) {
        for (unit, men) in army.units.iter_mut().zip(&plan.per_unit) {
            unit.strength = (unit.strength + men).min(unit.max_strength.max(unit.strength));
        }
    }
    if let Some(faction) = state.factions.get_mut(faction) {
        faction.treasury -= i64::from(plan.cost);
    }
}

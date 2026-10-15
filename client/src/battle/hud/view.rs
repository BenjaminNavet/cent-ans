//! Plain views of the simulation for the HUD (what `BattleSim.get_units()` dictionaries are for
//! the Godot HUD): no rule, only a snapshot of what the cards, the tooltips and the result
//! screen display. Pure functions, unit-tested.

use sim_battle::{Unit, UnitFate, UnitState};

use data_model::UnitCategory;

/// The three « batailles » of the army that arrange the cards (`battle_groups.gd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BattleRow {
    Vanguard,
    Main,
    Rear,
}

impl BattleRow {
    pub const ORDER: [BattleRow; 3] = [Self::Vanguard, Self::Main, Self::Rear];

    pub fn key(self) -> &'static str {
        match self {
            Self::Vanguard => "vanguard",
            Self::Main => "main",
            Self::Rear => "rear",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Vanguard => "Avant-garde",
            Self::Main => "Bataille",
            Self::Rear => "Arrière-garde",
        }
    }
}

/// One regiment as the interface shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitView {
    pub id: u32,
    pub unit_type: String,
    pub name: String,
    pub category: UnitCategory,
    pub soldiers: u32,
    pub initial: u32,
    /// 0..100.
    pub morale: f32,
    /// 0..100.
    pub fatigue: f32,
    pub ammo: u32,
    pub max_ammo: u32,
    pub can_shoot: bool,
    pub is_general: bool,
    pub present: bool,
    pub left_field: bool,
    pub reserve: bool,
    pub state: UnitState,
    pub formation: String,
    pub experience: u8,
    pub kills: u32,
    pub fate: UnitFate,
    pub melee: u8,
    pub ranged: u8,
    pub armor: u8,
    pub x: f64,
    pub z: f64,
    pub row: BattleRow,
}

/// View of `unit`.
pub fn unit_view(unit: &Unit) -> UnitView {
    UnitView {
        id: unit.id,
        unit_type: unit.unit_type.to_string(),
        name: unit.name.to_string(),
        category: unit.category,
        soldiers: unit.soldiers(),
        initial: unit.initial_soldiers,
        morale: unit.morale as f32,
        fatigue: unit.fatigue as f32,
        ammo: unit.ammo,
        max_ammo: unit.stats.ammo,
        can_shoot: unit.can_shoot(),
        is_general: unit.is_general,
        present: unit.present(),
        left_field: unit.left_field,
        reserve: unit.reserve,
        state: unit.state,
        formation: unit.formation.def().name.to_string(),
        experience: unit.experience,
        kills: unit.kills.round() as u32,
        fate: unit.fate(),
        melee: unit.stats.melee,
        ranged: unit.stats.ranged,
        armor: unit.stats.armor,
        x: unit.x,
        z: unit.z,
        row: battle_row(unit),
    }
}

/// Default « bataille » of a regiment (`BattleGroups.default_battle`): reserve and engines at
/// the rear, cavalry in the vanguard, the rest in the main body.
pub fn battle_row(unit: &Unit) -> BattleRow {
    if unit.reserve || unit.category == UnitCategory::Siege || unit.ram || unit.siege_tower() {
        BattleRow::Rear
    } else if unit.category == UnitCategory::Cavalry {
        BattleRow::Vanguard
    } else {
        BattleRow::Main
    }
}

/// Short state of a card (`UnitCard.state_text`, with the simulation's labels spelled out).
pub fn state_text(unit: &UnitView) -> &'static str {
    if unit.left_field {
        "hors du champ"
    } else if unit.reserve {
        "en réserve"
    } else if !unit.present {
        "anéantie"
    } else {
        match unit.state {
            UnitState::Idle => "au repos",
            UnitState::Marching => "en marche",
            UnitState::Charging => "à la charge",
            UnitState::Melee => "au corps à corps",
            UnitState::Shooting => "tire",
            UnitState::Routing => "en déroute",
            UnitState::Rallied => "ralliée",
            UnitState::Climbing => "à l’escalade",
        }
    }
}

/// End-of-battle fate (`BattleResultScreen.FATE_LABELS`).
pub fn fate_text(fate: UnitFate) -> &'static str {
    match fate {
        UnitFate::Destroyed => "anéanti",
        UnitFate::Routed => "en déroute",
        UnitFate::Withdrawn => "s’est retiré",
        UnitFate::Reserve => "en réserve",
        UnitFate::Held => "tient le champ",
    }
}

/// Roman numeral of `n` (1..), for homonymous regiments.
pub fn roman(n: usize) -> String {
    const TABLE: [(usize, &str); 9] = [
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
        (0, ""),
        (0, ""),
    ];
    let mut rest = n;
    let mut out = String::new();
    for (value, text) in TABLE {
        while value > 0 && rest >= value {
            out.push_str(text);
            rest -= value;
        }
    }
    out
}

/// Distinct names for the regiments of one side (« Chevaliers », « Chevaliers II »…), in the
/// order given (`BattleHud.distinct_names`).
pub fn distinct_names(units: &[UnitView]) -> Vec<(u32, String)> {
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut total: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for unit in units {
        *total.entry(unit.name.as_str()).or_default() += 1;
    }
    units
        .iter()
        .map(|unit| {
            let rank = seen.entry(unit.name.as_str()).or_default();
            *rank += 1;
            let name = if total[unit.name.as_str()] > 1 && *rank > 1 {
                format!("{} {}", unit.name, roman(*rank))
            } else {
                unit.name.clone()
            };
            (unit.id, name)
        })
        .collect()
}

/// Thousands-separated integer with a no-break space (« 1 234 »).
pub fn number(value: f64) -> String {
    crate::ui::kit::fr::digits(value.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: u32, name: &str) -> UnitView {
        UnitView {
            id,
            unit_type: "unit_x".into(),
            name: name.into(),
            category: UnitCategory::Infantry,
            soldiers: 100,
            initial: 100,
            morale: 80.0,
            fatigue: 0.0,
            ammo: 0,
            max_ammo: 0,
            can_shoot: false,
            is_general: false,
            present: true,
            left_field: false,
            reserve: false,
            state: UnitState::Idle,
            formation: "Ligne".into(),
            experience: 0,
            kills: 0,
            fate: UnitFate::Held,
            melee: 5,
            ranged: 0,
            armor: 3,
            x: 0.0,
            z: 0.0,
            row: BattleRow::Main,
        }
    }

    #[test]
    fn homonymous_regiments_get_numerals() {
        let units = vec![unit(1, "Piquiers"), unit(2, "Archers"), unit(3, "Piquiers")];
        let names = distinct_names(&units);
        assert_eq!(names[0].1, "Piquiers");
        assert_eq!(names[1].1, "Archers");
        assert_eq!(names[2].1, "Piquiers II");
    }

    #[test]
    fn roman_numerals() {
        assert_eq!(roman(2), "II");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
    }

    #[test]
    fn state_text_prefers_field_status() {
        let mut view = unit(1, "A");
        view.state = UnitState::Melee;
        assert_eq!(state_text(&view), "au corps à corps");
        view.reserve = true;
        assert_eq!(state_text(&view), "en réserve");
        view.left_field = true;
        assert_eq!(state_text(&view), "hors du champ");
    }
}

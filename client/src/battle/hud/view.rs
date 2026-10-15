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
    /// Display states read from the core (badges of the cards).
    pub status: StatusView,
    /// Key of the formation (`unit_formations.json`) and the change under way (0..1, `None`
    /// once formed).
    pub formation_key: String,
    pub reform_progress: Option<f32>,
}

/// Display states of a regiment (`BattleSim::unit_status`, `unit_rallied`, modes, fatigue).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatusView {
    pub charging: bool,
    pub under_fire: bool,
    pub engaged: bool,
    pub wavering: bool,
    pub low_ammo: bool,
    pub rallied: bool,
    /// Fatigue above the threshold where the simulation takes morale away.
    pub exhausted: bool,
    /// Running at a target without being in melee or shooting.
    pub running_at_target: bool,
    /// Keys of the modes switched on, in the order of the mode buttons.
    pub modes: Vec<&'static str>,
    /// Keys of the formations the regiment may take (`FormationDef::allows`).
    pub allowed_formations: Vec<&'static str>,
}

/// Reads the display states of `unit` from the simulation.
pub fn status_view(sim: &sim_battle::BattleSim, unit: &Unit) -> StatusView {
    let status = sim.unit_status(unit);
    let mode_list = [
        ("run", unit.mode_run),
        ("guard", unit.guard),
        ("skirmish", unit.skirmish),
        ("melee", unit.melee_mode),
        ("breach", unit.breach),
    ];
    StatusView {
        charging: status.charging,
        under_fire: status.under_fire,
        engaged: status.engaged,
        wavering: status.wavering,
        low_ammo: status.low_ammo,
        rallied: sim.unit_rallied(unit),
        exhausted: unit.fatigue > sim_battle::MoraleRules::bundled().exhausted_fatigue,
        running_at_target: unit.running
            && unit.target.is_some()
            && unit.state != UnitState::Melee
            && unit.state != UnitState::Shooting,
        modes: mode_list
            .into_iter()
            .filter_map(|(key, on)| on.then_some(key))
            .collect(),
        allowed_formations: sim_battle::Formation::all()
            .filter(|f| f.def().allows(unit))
            .map(|f| f.key())
            .collect(),
    }
}

/// Most badges shown on a card (`MAX_BADGES`).
pub const MAX_BADGES: usize = 3;

/// State badges of a regiment, by priority: rout > rallied > wavering > under fire > low ammo >
/// charge / melee > first mode > shooting > tired (`BattleUnitMarkers.state_badges`). Keys are the
/// icon suffixes (`battle_state_<key>`).
pub fn state_badges(unit: &UnitView) -> Vec<String> {
    let status = &unit.status;
    let mut badges: Vec<String> = Vec::new();
    if unit.state == UnitState::Routing {
        badges.push("rout".into());
    } else {
        if status.rallied {
            badges.push("rallied".into());
        }
        if status.wavering {
            badges.push("wavering".into());
        }
        if status.under_fire {
            badges.push("under_fire".into());
        }
        if status.low_ammo {
            badges.push("low_ammo".into());
        }
        if status.charging || status.running_at_target {
            badges.push("charge".into());
        } else if status.engaged {
            badges.push("melee".into());
        }
        if let Some(mode) = status.modes.first() {
            badges.push(format!("mode_{mode}"));
        }
        if unit.state == UnitState::Shooting {
            badges.push("shoot".into());
        }
    }
    if status.exhausted {
        badges.push("tired".into());
    }
    badges.truncate(MAX_BADGES);
    badges
}

/// Background of a badge: rout / wavering blink, the others have a fixed tint.
pub fn badge_color(kind: &str, blink: bool) -> [f32; 3] {
    match kind {
        "rout" if blink => [0.8, 0.1, 0.05],
        "rout" => [0.55, 0.08, 0.05],
        "wavering" if blink => [0.95, 0.68, 0.12],
        "wavering" => [0.8, 0.5, 0.08],
        "under_fire" => [0.93, 0.83, 0.6],
        "rallied" => [0.55, 0.82, 0.5],
        "low_ammo" => [0.95, 0.78, 0.55],
        _ => [0.95, 0.9, 0.78],
    }
}

/// Letter shown on a badge when its ink icon is missing.
pub fn badge_glyph(kind: &str) -> &'static str {
    match kind {
        "rout" => "⚑",
        "rallied" => "✓",
        "wavering" => "~",
        "under_fire" => "↯",
        "low_ammo" => "◦",
        "charge" => "»",
        "melee" => "✕",
        "shoot" => "➶",
        "tired" => "z",
        "mode_run" => "»",
        "mode_guard" => "▣",
        "mode_skirmish" => "↩",
        "mode_melee" => "⚔",
        "mode_breach" => "⛨",
        _ => "•",
    }
}

/// French label of a badge (tooltip).
pub fn badge_label(kind: &str) -> &'static str {
    match kind {
        "rout" => "En déroute",
        "rallied" => "Vient de se rallier",
        "wavering" => "Hésite",
        "under_fire" => "Sous le feu",
        "low_ammo" => "Presque à court de munitions",
        "charge" => "À la charge",
        "melee" => "Au corps à corps",
        "shoot" => "Tire",
        "tired" => "Épuisée",
        "mode_run" => "Mode : course",
        "mode_guard" => "Mode : garde",
        "mode_skirmish" => "Mode : escarmouche",
        "mode_melee" => "Mode : mêlée",
        "mode_breach" => "Mode : battre en brèche",
        _ => "",
    }
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
        status: StatusView::default(),
        formation_key: unit.formation.key().to_owned(),
        reform_progress: unit.reform.as_ref().map(|r| r.progress() as f32),
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
            status: StatusView::default(),
            formation_key: "line".into(),
            reform_progress: None,
        }
    }

    #[test]
    fn badges_follow_the_priority_and_stop_at_three() {
        let mut view = unit(1, "A");
        view.status = StatusView {
            wavering: true,
            under_fire: true,
            low_ammo: true,
            engaged: true,
            exhausted: true,
            ..Default::default()
        };
        assert_eq!(state_badges(&view), ["wavering", "under_fire", "low_ammo"]);
        view.state = UnitState::Routing;
        assert_eq!(state_badges(&view), ["rout", "tired"]);
        view.state = UnitState::Idle;
        view.status = StatusView {
            engaged: true,
            modes: vec!["guard", "run"],
            ..Default::default()
        };
        assert_eq!(state_badges(&view), ["melee", "mode_guard"]);
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

//! Campaign difficulty (lot DF1, ADR 0030): the level chosen at launch and
//! frozen for the campaign, and the rule hooks that read its modifiers
//! (`data/rules/difficulty.json`).
//!
//! Every hook is exactly neutral at [`Difficulty::Normal`]: percentages of
//! 100 leave the value untouched (no rounding), offsets of 0 add nothing, so
//! a normal campaign plays bit for bit as before the lot.
//!
//! Levers (who they touch):
//! - income of the AI factions / of the player ([`CampaignState::faction_income_effective`]);
//! - upkeep of the AI armies and garrisons ([`CampaignState::faction_upkeep`]);
//! - recruitment cost of the AI ([`CampaignState::recruit_cost`]);
//! - unrest in the player's provinces (`population::resolve_population`);
//! - attitude of the AI towards the player ([`CampaignState::attitude`]) and
//!   the power ratio it demands before declaring war on him
//!   (`diplomacy::war_target`);
//! - morale of the AI armies fighting the player, in the 3D battle setup
//!   (`battle_request`) and in the auto-resolver (`movement::auto_fight`,
//!   sieges, forecast).

use data_model::{DifficultyModifiers, FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::battle_auto::Side;
use crate::state::{ArmyId, CampaignState};

/// Difficulty level of a campaign. Saves without the field load as
/// [`Difficulty::Normal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
    VeryHard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 4] = [
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
        Difficulty::VeryHard,
    ];

    /// Id used in `data/rules/difficulty.json` and by the bridge.
    pub fn as_str(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
            Difficulty::VeryHard => "very_hard",
        }
    }

    /// Parses an id of `data/rules/difficulty.json`.
    pub fn from_id(id: &str) -> Option<Difficulty> {
        Difficulty::ALL.into_iter().find(|d| d.as_str() == id)
    }
}

/// `value × percent / 100`, exactly `value` at 100 %.
pub fn scale_i64(value: i64, percent: u32) -> i64 {
    if percent == 100 {
        value
    } else {
        (value as f64 * f64::from(percent) / 100.0).round() as i64
    }
}

impl CampaignState {
    /// The campaign's difficulty level.
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    /// Sets the difficulty level; refused (false) once the first turn has
    /// been played: the level is frozen for the campaign.
    pub fn set_difficulty(&mut self, level: Difficulty) -> bool {
        if self.turn > 0 {
            return self.difficulty == level;
        }
        self.difficulty = level;
        true
    }

    /// Modifiers of the campaign's level (neutral when the data lacks it).
    pub fn difficulty_modifiers(&self, data: &GameData) -> DifficultyModifiers {
        data.difficulty.modifiers(self.difficulty.as_str())
    }

    fn is_ai(&self, faction: &FactionId) -> bool {
        faction != &self.player_faction
    }

    /// Income percentage of `faction` (AI or player lever).
    pub fn difficulty_income_percent(&self, data: &GameData, faction: &FactionId) -> u32 {
        let m = self.difficulty_modifiers(data);
        if self.is_ai(faction) {
            m.ai_income_percent
        } else {
            m.player_income_percent
        }
    }

    /// Upkeep percentage of `faction`'s armies and garrisons (AI only).
    pub fn difficulty_upkeep_percent(&self, data: &GameData, faction: &FactionId) -> u32 {
        if self.is_ai(faction) {
            self.difficulty_modifiers(data).ai_upkeep_percent
        } else {
            100
        }
    }

    /// Recruitment cost percentage of `faction` (AI only).
    pub fn difficulty_recruit_percent(&self, data: &GameData, faction: &FactionId) -> u32 {
        if self.is_ai(faction) {
            self.difficulty_modifiers(data).ai_recruit_cost_percent
        } else {
            100
        }
    }

    /// Unrest points added in a province controlled by `controller` (the
    /// player only).
    pub fn difficulty_unrest(&self, data: &GameData, controller: &FactionId) -> f64 {
        if self.is_ai(controller) {
            0.0
        } else {
            f64::from(self.difficulty_modifiers(data).player_unrest)
        }
    }

    /// Attitude points of `a` towards `b`: an AI towards the player.
    pub fn difficulty_attitude(&self, data: &GameData, a: &FactionId, b: &FactionId) -> i32 {
        if self.is_ai(a) && !self.is_ai(b) {
            self.difficulty_modifiers(data).ai_attitude_to_player
        } else {
            0
        }
    }

    /// Factor on the power ratio an AI demands before declaring war on
    /// `target` (1 unless `target` is the player).
    pub fn difficulty_war_ratio_factor(&self, data: &GameData, target: &FactionId) -> f64 {
        let percent = if self.is_ai(target) {
            100
        } else {
            self.difficulty_modifiers(data).ai_war_ratio_percent
        };
        f64::from(percent) / 100.0
    }

    /// Morale bonus of a side (AI) fighting the player: `side_has_player`
    /// and `enemy_has_player` tell whether a player army or garrison stands
    /// on each side.
    pub fn difficulty_morale_bonus(
        &self,
        data: &GameData,
        side_has_player: bool,
        enemy_has_player: bool,
    ) -> f64 {
        if !side_has_player && enemy_has_player {
            f64::from(self.difficulty_modifiers(data).ai_morale_vs_player)
        } else {
            0.0
        }
    }

    /// True when one of the armies `ids` belongs to the player.
    pub fn coalition_has_player(&self, ids: &[ArmyId]) -> bool {
        ids.iter().any(|id| {
            self.armies
                .get(id)
                .is_some_and(|a| a.faction == self.player_faction)
        })
    }

    /// Adds the difficulty morale bonus to both auto-resolve sides (general
    /// morale bonus: it lifts the whole side's morale like a general's
    /// `ArmyMorale`, as `sim-battle` does for the 3D battle).
    pub fn apply_difficulty_morale(
        &self,
        data: &GameData,
        attacker: &mut Side,
        attacker_has_player: bool,
        defender: &mut Side,
        defender_has_player: bool,
    ) {
        attacker.general_morale_bonus +=
            self.difficulty_morale_bonus(data, attacker_has_player, defender_has_player);
        defender.general_morale_bonus +=
            self.difficulty_morale_bonus(data, defender_has_player, attacker_has_player);
    }
}

/// French one-line summaries of a level's effects, for the UI tooltip
/// (neutral levers are omitted).
pub fn effect_summary(m: &DifficultyModifiers) -> Vec<String> {
    let mut lines = Vec::new();
    let percent = |label: &str, value: u32, lines: &mut Vec<String>| {
        if value != 100 {
            let delta = i64::from(value) - 100;
            lines.push(format!("{label} : {delta:+} %"));
        }
    };
    percent("Revenus de l'IA", m.ai_income_percent, &mut lines);
    percent("Vos revenus", m.player_income_percent, &mut lines);
    percent("Entretien des armées IA", m.ai_upkeep_percent, &mut lines);
    percent(
        "Coût de recrutement IA",
        m.ai_recruit_cost_percent,
        &mut lines,
    );
    if m.player_unrest != 0 {
        lines.push(format!(
            "Agitation dans vos provinces : {:+}",
            m.player_unrest
        ));
    }
    if m.ai_attitude_to_player != 0 {
        lines.push(format!(
            "Attitude de l'IA envers vous : {:+}",
            m.ai_attitude_to_player
        ));
    }
    if m.ai_war_ratio_percent != 100 {
        let delta = i64::from(m.ai_war_ratio_percent) - 100;
        lines.push(format!(
            "Rapport de forces exigé par l'IA pour vous déclarer la guerre : {delta:+} %"
        ));
    }
    if m.ai_morale_vs_player != 0 {
        lines.push(format!(
            "Moral des armées IA contre vous : {:+}",
            m.ai_morale_vs_player
        ));
    }
    if lines.is_empty() {
        lines.push("Aucun modificateur.".to_owned());
    }
    lines
}

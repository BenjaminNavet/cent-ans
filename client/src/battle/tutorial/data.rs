//! Data and pure evaluation of the battle tutorial (`data/tutorial/battle_prologue.json`, port of
//! the static functions of `battle_prologue.gd`): the steps, their pass conditions and
//! [`evaluate`], which decides a condition from an [`Observed`] state of the battle and the
//! [`Baseline`] taken when the step began. No ECS here, so it is tested without a scene.

use std::collections::HashMap;

use bevy::math::Vec3;
use serde::Deserialize;
use sim_battle::custom::CustomBattle;

/// The whole file.
#[derive(Debug, Clone, Deserialize)]
pub struct Prologue {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub menu_label: String,
    #[serde(default)]
    pub menu_detail: String,
    #[serde(default)]
    pub advisor: String,
    /// Custom battle configuration (the `seed` key is read apart, see [`Prologue::seed`]).
    pub battle: serde_json::Value,
    #[serde(default)]
    pub enemy_passive: bool,
    #[serde(default)]
    pub defeat: Defeat,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Defeat {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub text: String,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub advice: String,
    pub condition: Condition,
    /// Interface element or unit to highlight (`cards`, `pause`, `minimap`, `enemy`, `unit:<category>`).
    #[serde(default)]
    pub target: String,
    /// From this step on the enemy fights again.
    #[serde(default)]
    pub enemy_ai: bool,
}

impl Step {
    pub fn manual(&self) -> bool {
        matches!(self.condition, Condition::Manual)
    }
}

fn d_move() -> f32 {
    25.0
}
fn d_cam_move() -> f32 {
    40.0
}
fn d_cam_zoom() -> f32 {
    30.0
}
fn d_cam_yaw() -> f32 {
    0.4
}
fn d_ratio() -> f32 {
    0.15
}
fn d_count() -> usize {
    1
}

/// Pass condition of a step (`condition.type` of the file).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    Manual,
    CameraMoved {
        #[serde(default = "d_cam_move")]
        min_distance: f32,
        #[serde(default = "d_cam_zoom")]
        min_zoom: f32,
        #[serde(default = "d_cam_yaw")]
        min_yaw: f32,
    },
    Selection {
        #[serde(default)]
        category: String,
        #[serde(default = "d_count")]
        min_count: usize,
    },
    Moved {
        #[serde(default)]
        category: String,
        #[serde(default = "d_move")]
        min_distance: f32,
    },
    FormationChanged {
        #[serde(default)]
        category: String,
        #[serde(default = "d_ratio")]
        min_ratio: f32,
    },
    UnitState {
        #[serde(default)]
        category: String,
        #[serde(default)]
        states: Vec<String>,
    },
    Paused,
    Victory,
}

impl Prologue {
    pub fn parse(text: &str) -> Result<Self, String> {
        let prologue: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if prologue.steps.is_empty() {
            return Err("no step".to_owned());
        }
        prologue.custom_battle()?;
        Ok(prologue)
    }

    /// The custom battle of the tutorial.
    pub fn custom_battle(&self) -> Result<CustomBattle, String> {
        serde_json::from_value(self.battle.clone()).map_err(|e| format!("battle: {e}"))
    }

    /// Seed of the skirmish (fixed: the same one every time).
    pub fn seed(&self) -> u64 {
        self.battle["seed"].as_u64().unwrap_or(1337)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.steps.iter().position(|s| s.id == id)
    }
}

/// A regiment as the conditions see it.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitObs {
    pub id: u32,
    /// Belongs to the player's side.
    pub own: bool,
    pub present: bool,
    pub category: String,
    pub state: String,
    pub x: f32,
    pub z: f32,
    /// Front width in metres.
    pub width: f32,
    pub formation: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraObs {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
}

/// State of the battle read each check.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Observed {
    pub units: Vec<UnitObs>,
    pub selected: Vec<u32>,
    pub paused: bool,
    pub camera: Option<CameraObs>,
    pub finished: bool,
    /// The player's side won (meaningful when `finished`).
    pub player_won: bool,
}

/// State at the start of a step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Baseline {
    /// Per regiment: position, front width, formation.
    pub units: HashMap<u32, (f32, f32, f32, String)>,
    pub camera: Option<CameraObs>,
}

impl Baseline {
    pub fn of(state: &Observed) -> Self {
        Self {
            units: state
                .units
                .iter()
                .map(|u| (u.id, (u.x, u.z, u.width, u.formation.clone())))
                .collect(),
            camera: state.camera,
        }
    }
}

/// Own regiments on the field, of `category` unless it is empty.
fn own_units<'a>(state: &'a Observed, category: &'a str) -> impl Iterator<Item = &'a UnitObs> {
    state
        .units
        .iter()
        .filter(move |u| u.own && u.present && (category.is_empty() || u.category == category))
}

/// Smallest signed difference between two angles (rad).
pub fn angle_difference(a: f32, b: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let d = (b - a).rem_euclid(tau);
    if d > std::f32::consts::PI { d - tau } else { d }
}

/// Is `condition` met on `state` (reference `base`)? (`evaluate`)
pub fn evaluate(condition: &Condition, state: &Observed, base: &Baseline) -> bool {
    match condition {
        Condition::Manual => false,
        Condition::CameraMoved {
            min_distance,
            min_zoom,
            min_yaw,
        } => {
            let (Some(now), Some(before)) = (state.camera, base.camera) else {
                return false;
            };
            now.target.distance(before.target) >= *min_distance
                || (now.distance - before.distance).abs() >= *min_zoom
                || angle_difference(before.yaw, now.yaw).abs() >= *min_yaw
        }
        Condition::Selection {
            category,
            min_count,
        } => {
            own_units(state, category)
                .filter(|u| state.selected.contains(&u.id))
                .count()
                >= *min_count
        }
        Condition::Moved {
            category,
            min_distance,
        } => own_units(state, category).any(|u| {
            base.units.get(&u.id).is_some_and(|(x, z, _, _)| {
                ((u.x - x).powi(2) + (u.z - z).powi(2)).sqrt() >= *min_distance
            })
        }),
        Condition::FormationChanged {
            category,
            min_ratio,
        } => own_units(state, category).any(|u| {
            base.units
                .get(&u.id)
                .is_some_and(|(_, _, width, formation)| {
                    u.formation != *formation
                        || (*width > 0.0 && (u.width - width).abs() / width >= *min_ratio)
                })
        }),
        Condition::UnitState { category, states } => {
            own_units(state, category).any(|u| states.contains(&u.state))
        }
        Condition::Paused => state.paused,
        Condition::Victory => state.finished && state.player_won,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: u32, own: bool, category: &str) -> UnitObs {
        UnitObs {
            id,
            own,
            present: true,
            category: category.to_owned(),
            state: "idle".to_owned(),
            x: 0.0,
            z: 0.0,
            width: 20.0,
            formation: "line".to_owned(),
        }
    }

    fn camera(x: f32, distance: f32, yaw: f32) -> CameraObs {
        CameraObs {
            target: Vec3::new(x, 0.0, 0.0),
            distance,
            yaw,
        }
    }

    #[test]
    fn the_shipped_file_parses_and_its_battle_is_valid() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../data/tutorial/battle_prologue.json"),
        )
        .expect("battle_prologue.json");
        let prologue = Prologue::parse(&text).unwrap();
        assert_eq!(prologue.steps.len(), 10);
        assert_eq!(prologue.seed(), 1337);
        assert!(prologue.index_of("victory").is_some());
        let battle = prologue.custom_battle().unwrap();
        assert_eq!(battle.defender.units.len(), 4);
        assert_eq!(battle.player_side, "defender");
        assert!(prologue.steps.iter().filter(|s| s.manual()).count() == 2);
        assert!(prologue.steps.iter().any(|s| s.enemy_ai));
    }

    #[test]
    fn camera_moved_by_pan_zoom_or_turn() {
        let cond = Condition::CameraMoved {
            min_distance: 40.0,
            min_zoom: 30.0,
            min_yaw: 0.4,
        };
        let base = Baseline {
            camera: Some(camera(0.0, 100.0, 0.0)),
            ..default_baseline()
        };
        let mut state = Observed {
            camera: Some(camera(10.0, 110.0, 0.1)),
            ..Default::default()
        };
        assert!(!evaluate(&cond, &state, &base));
        state.camera = Some(camera(50.0, 100.0, 0.0));
        assert!(evaluate(&cond, &state, &base));
        state.camera = Some(camera(0.0, 140.0, 0.0));
        assert!(evaluate(&cond, &state, &base));
        // Turning across the ±pi seam is a short turn.
        let base = Baseline {
            camera: Some(camera(0.0, 100.0, 3.0)),
            ..default_baseline()
        };
        state.camera = Some(camera(0.0, 100.0, -3.0));
        assert!(!evaluate(&cond, &state, &base));
        state.camera = Some(camera(0.0, 100.0, 2.4));
        assert!(evaluate(&cond, &state, &base));
    }

    fn default_baseline() -> Baseline {
        Baseline::default()
    }

    #[test]
    fn selection_counts_only_own_regiments_of_the_category() {
        let mut state = Observed {
            units: vec![unit(1, true, "infantry"), unit(2, false, "infantry")],
            selected: vec![2],
            ..Default::default()
        };
        let cond = Condition::Selection {
            category: String::new(),
            min_count: 1,
        };
        assert!(!evaluate(&cond, &state, &default_baseline()));
        state.selected = vec![1];
        assert!(evaluate(&cond, &state, &default_baseline()));
        let cav = Condition::Selection {
            category: "cavalry".into(),
            min_count: 1,
        };
        assert!(!evaluate(&cav, &state, &default_baseline()));
    }

    #[test]
    fn moved_and_formation_changed_compare_with_the_baseline() {
        let mut state = Observed {
            units: vec![unit(1, true, "infantry")],
            ..Default::default()
        };
        let base = Baseline::of(&state);
        let moved = Condition::Moved {
            category: String::new(),
            min_distance: 25.0,
        };
        let reform = Condition::FormationChanged {
            category: String::new(),
            min_ratio: 0.15,
        };
        assert!(!evaluate(&moved, &state, &base) && !evaluate(&reform, &state, &base));
        state.units[0].x = 20.0;
        state.units[0].z = 20.0;
        assert!(evaluate(&moved, &state, &base));
        state.units[0].width = 21.0;
        assert!(!evaluate(&reform, &state, &base));
        state.units[0].width = 30.0;
        assert!(evaluate(&reform, &state, &base));
        state.units[0].width = 20.0;
        state.units[0].formation = "harrow".into();
        assert!(evaluate(&reform, &state, &base));
        // A regiment unknown at the start (reinforcement) never counts.
        state.units.push(unit(9, true, "infantry"));
        state.units[0].formation = "line".into();
        assert!(!evaluate(&reform, &state, &base));
    }

    #[test]
    fn unit_state_pause_and_victory() {
        let mut state = Observed {
            units: vec![unit(1, true, "cavalry")],
            ..Default::default()
        };
        let charge = Condition::UnitState {
            category: "cavalry".into(),
            states: vec!["charging".into(), "melee".into()],
        };
        assert!(!evaluate(&charge, &state, &default_baseline()));
        state.units[0].state = "charging".into();
        assert!(evaluate(&charge, &state, &default_baseline()));
        state.units[0].present = false;
        assert!(!evaluate(&charge, &state, &default_baseline()));
        assert!(!evaluate(&Condition::Paused, &state, &default_baseline()));
        state.paused = true;
        assert!(evaluate(&Condition::Paused, &state, &default_baseline()));
        state.finished = true;
        assert!(!evaluate(&Condition::Victory, &state, &default_baseline()));
        state.player_won = true;
        assert!(evaluate(&Condition::Victory, &state, &default_baseline()));
        assert!(!evaluate(&Condition::Manual, &state, &default_baseline()));
    }
}

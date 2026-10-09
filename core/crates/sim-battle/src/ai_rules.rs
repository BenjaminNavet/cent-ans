//! BA6: tuning constants of the battle AI (`ai/`), moved out of the code
//! into `data/rules/battle_ai.json` with their former values.

use serde::{Deserialize, Serialize};

/// Distances (metres), delays (seconds) and thresholds of the battle AI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleAiRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// B6: a defensive side looks for cover this far on either side of the
    /// centre of its deployment line.
    pub cover_lateral: f64,
    /// ... this far ahead of its deployment line (towards the enemy) ...
    pub cover_ahead: f64,
    /// ... and this far behind it.
    pub cover_behind: f64,
    /// Shooters stand this far behind a hedge, a fence or a ditch (well within
    /// [`crate::site::HEDGE_COVER_REACH`]).
    pub cover_setback: f64,
    /// R4: setbacks tried in turn behind a hedge on a crest (the last one still
    /// clear of the obstacle's [`OBSTACLE_REACH`]).
    pub cover_setbacks: [f64; 3],
    /// Shooters stand this far inside the edge of a village.
    pub village_setback: f64,
    /// Shooters stand this far in front of the line (the usual defensive order).
    pub shooters_ahead: f64,
    /// EP3: shooters hold the bank this far back from the water.
    pub bank_setback: f64,
    /// EP3: a crossing farther than this from the deployment line is not held.
    pub river_reach: f64,
    /// EP3: an advancing side waits this long on its bank while its shooters
    /// duel with the enemy shooters covering the crossing.
    pub crossing_patience: f64,
    /// EP3: each enemy shooter covering a crossing's far end costs this many
    /// metres of march.
    pub crossing_exposure: f64,
    /// EP3: the line forms this far beyond a crossing.
    pub bridgehead_depth: f64,
    /// B6 score a cover must reach to be considered at all.
    pub cover_threshold: f64,
    /// EP6: decor areas whose missile cover is at most this good count as a
    /// defensive position (hamlets, churchyards, manors, farms).
    pub village_cover_max: f64,
    /// IA night: the attacker's waiting horse stands this far (metres) behind
    /// its foot's centre ...
    pub wing_behind_foot: f64,
    /// ... until a foot regiment is this close (metres) to an enemy one (80 m:
    /// 672 wins of 768 against the novice and the passive side, before 618;
    /// 150 m: 651, 250 m: 610).
    pub wing_release_m: f64,
    /// IA night: the rule holds for a horse that took missile casualties within
    /// this many seconds (40 s: 670 wins of 768, mirrored armies unchanged and
    /// the demo's first contact still near 70 s; always: 672, but the first
    /// contact at 192 s).
    pub wing_under_fire_s: f64,
    /// Distance at which the line closes in at the run.
    pub charge_distance: f64,
    /// A defensive line counter-charges enemies this close.
    pub counter_charge_distance: f64,
    /// SG4: shooters behind their planted stakes on the military crest of a
    /// defensive position fall back when enemy foot comes this close.
    pub crest_stakes_safety: f64,
    /// Shooters fall back when enemy melee troops come this close.
    pub shooter_safety: f64,
    /// R4: shooters behind a hedge, a ditch or in a village fall back when enemy
    /// foot comes this close.
    pub cover_safety: f64,
    /// Distance kept from the field's edges by the AI's moves (a move outside
    /// the field is refused).
    pub field_margin: f64,
    /// Enemy shooters farther than this from their own melee troops are
    /// "isolated" (a cavalry target).
    pub isolation_distance: f64,
    /// Radius within which cavalry looks for targets.
    pub cavalry_reach: f64,
    /// Longest archery duel before the line advances regardless (seconds).
    pub duel_time: f64,
    /// A clearly stronger attacker (its enemy stands on the defensive) gives up
    /// the archery duel after this long and closes in (F5d: two AI armies
    /// always end up engaging).
    pub attacker_duel_time: f64,
    /// The archery duel is fought when the lines stand this close (metres);
    /// EP9b: closer than this, a line closing in after the duel advances in two
    /// echelons.
    pub duel_range: f64,
    /// A side whose losses exceed the enemy's by more than this share is
    /// losing the archery duel and closes in (B4).
    pub duel_loss_margin: f64,
    /// AI destinations keep this far from the edge of deep water (F5d).
    pub river_margin: f64,
    /// Closing in, the cavalry charges enemy horse this close to the line.
    pub assault_range: f64,
    /// Window in which a clearly stronger attacker rides at the enemy horse
    /// to open the fight (F5d).
    pub attacker_patience: f64,
    /// A weaker attacker waits this long for the defender to come to it, then
    /// engages anyway (B4: it is the side that sought the battle; armies meet
    /// within one to three minutes).
    pub attacker_wait: f64,
    /// A defensive side gives up waiting after this long ...
    pub defender_patience: f64,
    /// ... unless nobody has fought for this long: the attacker does not come,
    /// the defender keeps its ground and lets the battle be refused (EP9,
    /// ADR 0056).
    pub defender_quiet: f64,
    /// Besiegers wait for their engines at most this long before escalading.
    pub engine_patience: f64,
    /// SG4: a ram whose crew falls below this share of its full crew calls a
    /// foot regiment to take it over.
    pub ram_relief_crew: f64,
    /// SG4: the relieving regiment stands this far behind the ram, away from
    /// the gate (outside the reach of the boiling oil).
    pub ram_relief_stand: f64,
    /// SG4: an assault with no progress (ram blow, ladders, wall walk gained,
    /// tower docked, works down) for this long is abandoned.
    pub assault_stall: f64,
    /// SG4: an assault is abandoned when the besiegers' strength falls below
    /// this share of the garrison's (no opening, nobody on the walls).
    pub assault_hopeless: f64,
    /// SG4: engines choose the weakest front wall; each metre of distance from
    /// the engines weighs as this many HP (the nearest of equal walls).
    pub engine_target_hp_per_m: f64,
    /// R2b: a rise steeper than this (metres per metre over 20 m) is not
    /// charged at the run from afar: the regiment walks up and charges close.
    pub steep_climb: f64,
    /// R2b: a regiment hit by missiles within this many seconds is under fire.
    pub under_fire: f64,
    /// R2b: a line regiment this far ahead of the line's centre waits for it.
    pub line_slack: f64,
    /// R2b: below this distance a regiment charges whatever the slope.
    pub close_charge: f64,
    /// B8: horsemen give up a pursuit once the routing target has fled this far
    /// from the battle line's anchor, and fall back to it instead.
    pub pursuit_leash: f64,
    /// B8: dense bocage can chain several hedges between a horse and its target;
    /// this many are tried in turn before giving up and waiting.
    pub detour_hops: u32,
    /// B6: horsemen ride this far past the end of a hedge before charging.
    pub detour_clearance: f64,
    /// RJ-a: ... and this far out on the target's side, so that the charge is
    /// not ridden along the hedge (within its reach it would break; the old
    /// 10 m only worked while an instant wedge lengthened the riders' reach).
    pub detour_depth: f64,
    /// R2b: a defender this much higher than the enemy (mean ground under the
    /// regiments, metres) holds its heights ...
    pub hold_height: f64,
    /// ... unless it is this much stronger.
    pub hold_ratio: f64,
    /// R4: a defensive line regiment helps its shooters in a melee this close.
    pub rescue_distance: f64,
    /// R4: shooters run to the military crest when the enemy is this close.
    pub post_run: f64,
    /// R4: a defender receives an enemy marching on it from this close.
    pub receive_distance: f64,
    /// R4: an attacker above an enemy of shooters waits at most this long.
    pub attacker_hold_time: f64,
    /// R4: ... when shooters make more than this share of the enemy's power.
    pub shooter_army: f64,
    /// B8: within the forward arc, the line leans this many metres (at most)
    /// towards a defender offset sideways from dead ahead, instead of marching
    /// straight past it. A defender squarely in front (`dx` ~ 0) is unaffected.
    pub advance_lean_max: f64,
    /// R2b: lateral shifts tried for each step of an advancing line.
    pub relief_leans: [f64; 4],
    /// R2b: a shifted step must save this much march (metres) to be taken.
    pub relief_saving: f64,
    /// R4: the own line must reach a position in less than this share of the
    /// time the nearest enemy needs to get there.
    pub race_margin: f64,
    /// R4: a spot of the heights must beat the deployment spot by this much.
    pub ground_margin: f64,
    /// R4: a cover away from the deployment line costs this much per metre
    /// aside and in depth (B6's distance penalty, in points of height).
    pub cover_lateral_cost: f64,
    pub cover_depth_cost: f64,
    /// R4: metres of lateral offset a line regiment accepts to strike an enemy
    /// holding one point less of ground ([`score_position`]).
    pub weak_point: f64,
    /// R2b: lateral offsets of the firing spots tried by a shooter.
    pub firing_laterals: [f64; 5],
    /// R2b: a firing spot within reach of this many enemy shooters costs this
    /// much march (metres) each.
    pub exposure_cost: f64,
    /// SG4: enemy horse this close to one of our shooters draws our horse's
    /// counter-charge.
    pub shooter_guard: f64,
    /// R2b: horsemen keep this far from the front of planted stakes.
    pub stakes_guard: f64,
}

data_model::bundled_rules!(BattleAiRules, "rules/battle_ai.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        let rules = BattleAiRules::bundled();
        assert_eq!(rules.charge_distance, 60.0);
        assert_eq!(rules.cover_setbacks[0], rules.cover_setback);
        assert!(rules.counter_charge_distance < rules.charge_distance);
    }
}

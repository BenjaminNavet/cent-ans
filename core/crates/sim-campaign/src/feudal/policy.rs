//! Call points of the feudal AI (lot FE5, ADR 0110).
//!
//! The reactive feudal decisions happen inside the resolution of another
//! faction's order (a declaration of war calls the target's suzerain, who
//! summons its host...), where no planner runs. The core asks them of a
//! [`FeudalPolicy`]: the provisional rules of F2 by default (minimal AI,
//! the simulation's own tests), the scores of `crates/ai` once
//! `ai::feudal::install` has registered them (process-wide, set once: the
//! policy is a set of pure functions of the state, so determinism holds).
//!
//! The proactive decisions (forfeiture, grant, revolt, homage, titles
//! demanded at the peace) are orders planned by `crates/ai`.

use std::sync::OnceLock;

use data_model::{FactionId, GameData};

use super::Arbitration;
use crate::state::CampaignState;

/// Score of a suzerain called to protect a vassal, relative to its
/// decision threshold (`>= 0`: it intervenes), and its main reason (French).
pub type ProtectionFn =
    fn(&CampaignState, &GameData, &FactionId, &FactionId, &FactionId) -> (i32, String);
/// Verdict of a lord on the private war of `attacker` against `target`.
pub type ArbitrationFn =
    fn(&CampaignState, &GameData, &FactionId, &FactionId, &FactionId) -> (Arbitration, String);
/// Does `vassal` answer the host of `liege` against `enemy`?
pub type HostFn = fn(&CampaignState, &GameData, &FactionId, &FactionId, &FactionId) -> bool;

/// The feudal decisions the core delegates.
#[derive(Debug, Clone, Copy)]
pub struct FeudalPolicy {
    /// `(liege, vassal, aggressor)`.
    pub protection: ProtectionFn,
    /// `(lord, attacker, target)`.
    pub arbitration: ArbitrationFn,
    /// `(vassal, liege, enemy)`, AI vassals only (the player's vassals
    /// follow the loyalty threshold).
    pub answers_host: HostFn,
    /// Band (± points around the threshold) shown as « uncertain ».
    pub certainty_margin: fn(&GameData) -> i32,
    /// `true`: AI vassals revolt by their own order (`Order::Revolt`); the
    /// random revolt of the diplomacy phase only strikes the player's
    /// vassal ties.
    pub planned_revolts: bool,
}

/// Provisional rules of the core (F2).
pub const PROVISIONAL: FeudalPolicy = FeudalPolicy {
    protection: provisional_protection,
    arbitration: super::ai_arbitration,
    answers_host: provisional_host,
    certainty_margin: provisional_margin,
    planned_revolts: false,
};

static POLICY: OnceLock<FeudalPolicy> = OnceLock::new();

/// Registers `policy` for the whole process. `false` when one was already
/// installed (the first one stays).
pub fn install_policy(policy: FeudalPolicy) -> bool {
    POLICY.set(policy).is_ok()
}

/// The installed policy, or the provisional rules.
pub fn policy() -> &'static FeudalPolicy {
    POLICY.get().unwrap_or(&PROVISIONAL)
}

fn provisional_protection(
    state: &CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
    aggressor: &FactionId,
) -> (i32, String) {
    let (score, reason) = super::protection_score(state, data, liege, vassal, aggressor);
    (
        score - data.feudal_rules.escalation.score.intervene_at,
        reason,
    )
}

fn provisional_host(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
    _liege: &FactionId,
    _enemy: &FactionId,
) -> bool {
    state
        .factions
        .get(vassal)
        .is_some_and(|f| f.loyalty >= data.feudal_rules.call_to_arms_loyalty)
}

fn provisional_margin(data: &GameData) -> i32 {
    data.feudal_rules.escalation.score.certainty_margin
}

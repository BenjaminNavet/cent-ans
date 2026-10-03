//! Building slots of a settlement (lot A6-L15, ADR 0181): the grid shown by
//! the slot bar. A slot is one upgrade chain (`upgrades_from`) of the
//! buildings allowed in the settlement's kind; it holds at most one building
//! of its chain (upgrading replaces it, see `complete_building`). Read-only:
//! no rule is added, the grid is derived from `data/buildings/`.

use data_model::{Building, BuildingId, GameData, SettlementId};
use serde::{Deserialize, Serialize};

use crate::buildings::BuildOption;
use crate::state::CampaignState;

/// Reasons that do not lock a slot (money or an ongoing build, ownership).
const TRANSIENT_REASONS: [&str; 3] = [
    "trésor insuffisant",
    "une construction est déjà en cours",
    "la colonie doit être possédée",
];

/// One slot of the grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingSlot {
    /// First building of the chain (stable id of the slot).
    pub root: BuildingId,
    /// Building standing in the slot, if any.
    pub built: Option<BuildingId>,
    /// Level of `built` in its chain (1 = base), 0 when empty.
    pub level: u8,
    /// Highest level reachable in this kind of settlement.
    pub max_level: u8,
    /// Next steps (the root when empty, the upgrades of `built` otherwise).
    pub next: Vec<BuildOption>,
    /// Empty slot whose next steps are all structurally impossible here
    /// (missing resource, coast, river, technology, prerequisite).
    pub locked: bool,
    /// Why, when `locked`.
    pub locked_reason: Option<String>,
}

impl CampaignState {
    /// The building slots of `settlement`, by category then id; empty for an
    /// unknown settlement.
    pub fn building_slots(&self, data: &GameData, settlement: &SettlementId) -> Vec<BuildingSlot> {
        let Some(state) = self.settlement_state(settlement) else {
            return Vec::new();
        };
        let kind = state.kind;
        let options = self.buildable(data, settlement);
        let allowed = |b: &&Building| b.allowed_in(kind);
        let mut roots: Vec<&Building> = data
            .buildings
            .values()
            .filter(allowed)
            .filter(|b| match &b.upgrades_from {
                None => true,
                // A step whose parent cannot stand here starts its own chain.
                Some(parent) => data
                    .buildings
                    .get(parent)
                    .is_none_or(|p| !p.allowed_in(kind)),
            })
            .collect();
        roots.sort_by_key(|b| (b.category as u8, b.id.clone()));
        roots
            .into_iter()
            .map(|root| {
                let depth_of = |id: &BuildingId| chain_level(data, id, &root.id);
                let built = state
                    .buildings
                    .iter()
                    .find(|id| depth_of(id).is_some())
                    .cloned();
                let level = built.as_ref().and_then(depth_of).unwrap_or(0);
                let next_ids: Vec<&BuildingId> = match &built {
                    None => vec![&root.id],
                    Some(built) => data
                        .buildings
                        .values()
                        .filter(|b| b.allowed_in(kind) && b.upgrades_from.as_ref() == Some(built))
                        .map(|b| &b.id)
                        .collect(),
                };
                let next: Vec<BuildOption> = options
                    .iter()
                    .filter(|o| next_ids.contains(&&o.building))
                    .cloned()
                    .collect();
                let locked_reason = if built.is_none() {
                    next.iter()
                        .map(|o| o.reason.as_deref())
                        .try_fold(None, |first: Option<String>, reason| match reason {
                            Some(r) if !TRANSIENT_REASONS.iter().any(|t| r.starts_with(t)) => {
                                Ok(first.or_else(|| Some(r.to_owned())))
                            }
                            _ => Err(()),
                        })
                        .ok()
                        .flatten()
                } else {
                    None
                };
                BuildingSlot {
                    root: root.id.clone(),
                    built,
                    level,
                    max_level: max_depth(data, &root.id, kind),
                    next,
                    locked: locked_reason.is_some(),
                    locked_reason,
                }
            })
            .collect()
    }
}

/// Level (1 = root) of `id` in the chain starting at `root`, `None` if `id`
/// is not in it.
fn chain_level(data: &GameData, id: &BuildingId, root: &BuildingId) -> Option<u8> {
    let mut current = id;
    for level in (1u8..).take(16) {
        if current == root {
            return Some(level);
        }
        current = data.buildings.get(current)?.upgrades_from.as_ref()?;
    }
    None
}

fn max_depth(data: &GameData, root: &BuildingId, kind: data_model::SettlementKind) -> u8 {
    let children = data
        .buildings
        .values()
        .filter(|b| b.allowed_in(kind) && b.upgrades_from.as_ref() == Some(root));
    1 + children
        .map(|c| max_depth(data, &c.id, kind))
        .max()
        .unwrap_or(0)
}

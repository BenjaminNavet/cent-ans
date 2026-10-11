//! RG11 (ADR 0345): the construction tree of a settlement. Every building of
//! `data/buildings/` as a node with its state (built, under construction,
//! queued, available, locked), the plain-French reason of a lock and its
//! requirements. Read-only: it reuses the validations of
//! [`CampaignState::buildable`]; building is still `Order::Build`.

use data_model::{BuildingId, GameData, SettlementId};

use crate::state::CampaignState;

/// State of a node of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Built,
    Building,
    Queued,
    Available,
    Locked,
}

/// Skeleton: filled in by the next commit.
pub struct BuildingTree {
    pub nodes: Vec<TreeNode>,
}

/// Skeleton: filled in by the next commit.
pub struct TreeNode {
    pub building: BuildingId,
    pub state: NodeState,
}

impl CampaignState {
    /// The construction tree of `settlement` (empty for an unknown id).
    pub fn building_tree(&self, _data: &GameData, _settlement: &SettlementId) -> BuildingTree {
        BuildingTree { nodes: Vec::new() }
    }
}

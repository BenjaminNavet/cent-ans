//! Trade hubs and routes (lot C5), mirroring `data/schemas/trade.schema.json`
//! (`data/economy/trade.json`).
//!
//! A [`TradeHub`] is a historic marketplace settlement (Bruges, Bordeaux,
//! Calais...). A [`TradeRouteDef`] links two hubs; the rules that turn it
//! into income (distance, security, agreements, coinage) live in
//! `sim-campaign::trade`.

use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, Sources};
use crate::ids::{ResourceId, SettlementId};

/// Contents of `data/economy/trade.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TradeCatalog {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub hubs: Vec<TradeHub>,
    pub routes: Vec<TradeRouteDef>,
}

impl TradeCatalog {
    pub fn hub(&self, id: &str) -> Option<&TradeHub> {
        self.hubs.iter().find(|h| h.id == id)
    }
}

/// A historic marketplace, anchored on one settlement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeHub {
    /// Stable id, e.g. `hub_bruges`.
    pub id: String,
    pub settlement: SettlementId,
    pub name: LocalizedName,
    /// Goods this marketplace is known for.
    pub goods: Vec<ResourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

/// A trade route between two hubs, before distance/security are applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeRouteDef {
    /// Stable id, e.g. `route_bruges_londres`.
    pub id: String,
    pub from_hub: String,
    pub to_hub: String,
    pub goods: Vec<ResourceId>,
    /// Seasonal income (livres) at full security and reference distance,
    /// before the faction-of-both-ends split.
    pub base_value: i64,
    /// The Champagne fairs' historic decline (lower activity from the 1330s).
    #[serde(default)]
    pub declining: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

//! `CampaignSim` feudal API (lot FE6, spec FE § 6): feudal tree, province
//! allegiance breadcrumb, obligations, war escalation preview, objectives,
//! faction sheets, and the player's feudal orders. Every rule lives in
//! `sim_campaign::feudal`; this module only converts to Godot values.

use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {}

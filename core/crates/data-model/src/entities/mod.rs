//! One module per entity type, each mirroring `data/schemas/<entity>.schema.json`.

pub mod agent;
pub mod ai_alignment;
pub mod ai_diplomacy;
pub mod battle_order;
pub mod building;
pub mod character;
pub mod chivalric_order;
pub mod diet;
pub mod edict;
pub mod event;
pub mod faction;
pub mod movement;
pub mod names;
pub mod province;
pub mod religion;
pub mod resource;
pub mod retinue;
pub mod settlement;
pub mod skill;
pub mod technology;
pub mod trade;
#[path = "trait_.rs"]
pub mod r#trait;
pub mod unit_type;
pub mod vision;

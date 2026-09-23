//! One module per entity type, each mirroring `data/schemas/<entity>.schema.json`.

pub mod battle_order;
pub mod building;
pub mod character;
pub mod diet;
pub mod event;
pub mod faction;
pub mod names;
pub mod province;
pub mod religion;
pub mod resource;
pub mod skill;
pub mod technology;
#[path = "trait_.rs"]
pub mod r#trait;
pub mod unit_type;

//! Shared visual effects (rendering only). `flipbook`: billboard sprites that play the player's
//! FA flame and smoke sheets (`game/assets/textures/fx/`, ADR 0164), port of `fx_flipbook`,
//! `fire_flame`, `fire_smoke` and `life_flame` shaders.

pub mod flipbook;
pub mod model_mips;

pub use flipbook::FlipbookPlugin;
pub use model_mips::ModelMipsPlugin;

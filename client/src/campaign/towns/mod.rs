//! Detailed towns at close zoom: town builder/plan/layers, outbuildings, landmarks, monuments, growth.
//!
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod data;
pub mod growth;
pub mod kit;
pub mod layer;
pub mod look;
pub mod plan;
pub mod profile;
pub mod rng;

pub use layer::{CampaignTownsPlugin, TownsState};

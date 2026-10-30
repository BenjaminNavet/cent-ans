//! IA-PLAN-1: division-based battle planner (audit 2026-10-29, R3).
//!
//! Behind `battle_ai.json` `planner: "plans"`; `"legacy"` (the default) keeps
//! [`super::plan_field`] alone. A side is cut into [`division::Division`]s at
//! deployment; a [`plans::Plan`] (data: `data/rules/battle_plans.json`) is a
//! list of phases, each giving a [`roles::Role`] to every division.
//! Skeleton: see `docs/wip/ia-plan-1.md`.

pub(super) mod division;
pub(super) mod plans;
pub(super) mod roles;

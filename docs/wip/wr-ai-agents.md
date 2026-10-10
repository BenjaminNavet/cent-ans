# WR ai-agents — état

Branche wr/ai-agents. Code : `core/crates/sim-campaign/src/agents/ai_strikes.rs` (un seul point d'accroche dans `agents/ai.rs`, boucle des agents de `plan_agents`). Paramètres : `data/rules/agents.json` bloc `ai_agents` (schéma mis à jour). Journal des attentats : `AgentsState.strike_log` (plafond glissant).
Écart à la spec : l'IA d'agents vit dans sim-campaign (pas dans la crate ai).
Fait : code + 5 tests (`tests/campaign_life/wr_ai_agents.rs`). Reste : sonde campaign_probe, ADR 0300.

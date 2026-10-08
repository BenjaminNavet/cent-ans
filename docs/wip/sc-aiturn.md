# SC aiturn (branche sc/aiturn)

État : campaign.rs découpé en `campaign/{army,economy,characters}.rs` (struct `Fleet`/`ArmyTurn`,
étapes nommées), constantes dans `data/ai/campaign.json` (`data_model::AiCampaign`), sels dans
`ai/src/salts.rs`, `threat` précalculé par province, grille spatiale pour `Enemy::near`,
`ai_minimal::plan_turn` découpé. Digest `turn_digest 6 1 2` identique à la référence.

Reste : CC10 (PlanCache explicite) bloqué : les accesseurs `faction_power` / `are_neighbors` / `rivals`
vivent dans `diplomacy.rs` (lot treaty) ; voir rapport.

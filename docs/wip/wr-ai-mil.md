# WR ai-mil — état

Branche `wr/ai-mil` (worktree `../gp-wr-ai-mil`). ADR 0301.
- Fait (à valider par cargo) : `ai/src/campaign/military_orders.rs` (RecruitInto, Sortie, DemandSurrender), seuils `data/ai/campaign.json:military_orders` + schéma + `MilitaryOrdersRules`, compteurs sorties/redditions dans `campaign_probe`, tests `ai/tests/movement/wr_ai_military_orders.rs`.
- Prochaine étape : clippy/tests, sonde avant (désactivée par seuils) / après, ADR 0301.

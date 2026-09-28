# RS-B — économie et ordre public (cœur)

Branche `feat/rs-b-order` (worktree d'agent). Orchestration : `docs/wip/restes.md`. ADR réservé : 0098.
Cible cargo privée : `core/target-rs-b` (à supprimer en fin de lot).

## État
- [x] 1. Constantes économiques de `economy.rs` → `data/rules/economy.json` (+ schéma, `EconomyRules`,
  pytest) : `tax_efficiency`, `tax_per_head`, `tax_rates` (multiplicateur, part prélevée),
  `production_tax_share`, `upkeep_months_per_season`, `garrison_upkeep_percent`,
  `garrison_relief_*`, `garrison_reinforce_*` (le plafond 50 du renfort était en ligne). Valeurs
  inchangées ; `TaxRate::multiplier/burden` et `province_income` prennent désormais les règles.
- [ ] 2. Sommes non pondérées par `province_effect_percent` (garnison qui apaise, peste, revenu
  estimé par l'IA ; recherche des abbayes déjà faite en DC6b via `research_percent`).
- [ ] 3. Révoltes 4-10 / partie (mesure avant/après, réglage dans les données, critères EQ6).
- [ ] 4. fmt / clippy / test / pytest, `git merge main`, suppression de la cible.

## Prochaine étape
Point 2 : pondérer `province_garrison_strength` (population), `plague_resistance` (medicine),
`province_income` de l'IA.

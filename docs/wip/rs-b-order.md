# RS-B — économie et ordre public (cœur)

Branche `feat/rs-b-order` (worktree d'agent). Orchestration : `docs/wip/restes.md`. ADR réservé : 0098.
Cible cargo privée : `core/target-rs-b` (à supprimer en fin de lot).

## État
- [x] 1. Constantes économiques de `economy.rs` → `data/rules/economy.json` (+ schéma, `EconomyRules`,
  pytest) : `tax_efficiency`, `tax_per_head`, `tax_rates` (multiplicateur, part prélevée),
  `production_tax_share`, `upkeep_months_per_season`, `garrison_upkeep_percent`,
  `garrison_relief_*`, `garrison_reinforce_*` (le plafond 50 du renfort était en ligne). Valeurs
  inchangées ; `TaxRate::multiplier/burden` et `province_income` prennent désormais les règles.
- [x] 2. Sommes pondérées par `province_effect_percent` : garnison qui apaise
  (`CampaignState::weighted_garrison_strength`, lue par `population`), résistance à la peste
  (`medicine::plague_resistance` via `province_building_effects`), revenu estimé par l'IA
  (`ai::campaign` `province_income` via `province_income_with` + effets pondérés) et évaluation des
  bâtiments par l'IA (apaisement, santé, croissance au poids de province, recherche à
  `research_percent`). Recherche des abbayes : déjà faite en DC6b. Test `rs_b_weighted_sums.rs` (4).
- [ ] 3. Révoltes 4-10 / partie (mesure avant/après, réglage dans les données, critères EQ6).
- [ ] 4. fmt / clippy / test / pytest, `git merge main`, suppression de la cible.

## Prochaine étape
Point 3 : mesurer `balance_probe campaign 200` graines 1-16 avant (binaire `bp_base`, commit 5c96ea30)
et après pondération, puis régler les révoltes dans les données.

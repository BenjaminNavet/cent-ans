# RS-B — économie et ordre public (cœur)

Branche `feat/rs-b-order` (worktree d'agent). Orchestration : `docs/wip/restes.md`. ADR : 0100 (0098 pris par FE).
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

## Mesures (`balance_probe campaign 200`, normale, graines 1-16, binaires de release dans le scratchpad)
| Variante | Révoltes / partie | Par graine | Guerre FR-EN | Banqueroutes | Impôt Haut | Mécontent. moyen |
|---|---|---|---|---|---|---|
| base (5c96ea30 = main + constantes) | 4,1 | 1 4 11 6 0 5 1 0 1 2 1 4 15 6 2 6 | 74 % | 0,08 | 33 % | 19,2 |
| pondération (108709cc) | 2,8 | 1 0 3 1 11 0 3 0 9 1 2 0 5 5 1 3 | 69 % | 0,10 | 32 % | 20,5 |
| + `revolt_seasons` 3 → 2 | 4,8 | 1 2 9 7 6 0 5 2 10 4 3 7 7 8 1 4 | 69 % | 0,10 | 33 % | 20,6 |
| + seuil 75 → 72 | 11,1 | 12 3 10 11 12 12 17 7 12 20 6 1 9 19 16 10 | 69 % | 0,08 | 30 % | 19,8 |
| **+ seuil 75 → 74 (retenu)** | **7,8** | 8 10 1 4 13 6 7 7 2 9 4 4 15 8 13 13 | 69 % | 0,13 | 31 % | 19,6 |

Bruit énorme entre graines (0 à 15). La pondération relève le mécontentement moyen (+1,3) mais pas
le compte de révoltes de façon mesurable.

## Prochaine étape
Retenu : `revolt_seasons` 2, `revolt_unrest_threshold` 74 (population.json + défauts Rust + codex
`cdx_jeu_ordre_public`). En cours : `century_probe` 4 niveaux sur ces données (`out/final`) et sonde
m3 release (1er essai : trésor France dans la bande, échec sur « sieges per turn », valeur à relire).
Puis ADR 0100 (écrit), fusion de main, tests complets, suppression de `core/target-rs-b`.

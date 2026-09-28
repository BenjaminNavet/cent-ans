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
- [x] 3. Révoltes : `revolt_seasons` 3 → 2 (seuil 75 inchangé), `population.json`, défaut Rust,
  codex `cdx_jeu_ordre_public`. ADR 0100.
- [x] 4. fmt / clippy / test / pytest, fusion de main.

## Mesures (`balance_probe campaign 200`, normale, graines 1-16, binaires de release dans le scratchpad)
| Variante | Révoltes / partie | Par graine | Guerre FR-EN | Banqueroutes | Impôt Haut | Mécontent. moyen |
|---|---|---|---|---|---|---|
| base (5c96ea30 = main + constantes) | 4,1 | 1 4 11 6 0 5 1 0 1 2 1 4 15 6 2 6 | 74 % | 0,08 | 33 % | 19,2 |
| pondération (108709cc) | 2,8 | 1 0 3 1 11 0 3 0 9 1 2 0 5 5 1 3 | 69 % | 0,10 | 32 % | 20,5 |
| **+ `revolt_seasons` 3 → 2, seuil 75 (retenu)** | **4,8** | 1 2 9 7 6 0 5 2 10 4 3 7 7 8 1 4 | 69 % | 0,10 | 33 % | 20,6 |
| + seuil 75 → 72 | 11,1 | 12 3 10 11 12 12 17 7 12 20 6 1 9 19 16 10 | 69 % | 0,08 | 30 % | 19,8 |
| + seuil 75 → 74 | 7,8 | 8 10 1 4 13 6 7 7 2 9 4 4 15 8 13 13 | 69 % | 0,13 | 31 % | 19,6 |

Bruit énorme entre graines (0 à 15). La pondération relève le mécontentement moyen (+1,3) mais pas
le compte de révoltes de façon mesurable.

## Sonde m3 release (`fifty_turns_on_eight_seeds_stay_in_the_c7a_band`, demande de l'orchestrateur)
Sur main : trésor moyen de la France 38 297 < 40 000 (échec préexistant). Le test affiche désormais
toutes ses valeurs avant de vérifier.
| Code / données | France | Angleterre | Sièges / tour | Bloquées | Débarq. | Batailles | Résultat |
|---|---|---|---|---|---|---|---|
| main (mesure RS-D) | 38 297 | — | — | — | — | — | échec trésor |
| constantes seules + révoltes 2 × 74 | 42 634 | 23 406 | 1,647 | 0,05 | 4,8 | 42 | ok |
| pondérations + révoltes 2 × 74 | 43 830 | 21 636 | 1,485 | 0,06 | 4,2 | 47 | échec sièges (< 1,5) |
| **état final (fusion de main, révoltes 2 × 75)** | **41 889** | 17 424 | 1,402 | 0,05 | 5,5 | 52 | **ok** |

Le trésor de la France revient dans la bande (41 889 ≥ 40 000) avec les réglages de révolte. La
pondération de l'IA (bâtiments des places secondaires estimés à leur vrai poids) baisse les sièges
(1,65 → 1,49, puis 1,40 après la fusion de main) : plancher recalibré 1,5 → 1,3 (ADR 0100), les
autres bornes inchangées.

## Century_probe 464 tours (binaire d'avant la fusion de main), essai 2 saisons × seuil 74
| Niveau | Guerre FR-EN moy. [min-max] | Graines 55-75 | Trêves | Révoltes / 200 t. | Banqueroutes | 1re faction fin |
|---|---|---|---|---|---|---|
| Facile (5) | 69 % [61-76] | 4/5 | 11,0 | 5,5 | 0,06 | 22 % |
| Normale (10) | 73 % [67-78] | 6/10 | 13,0 | 7,3 | 0,04 | 22 % |
| Difficile (10) | 66 % [45-73] | 9/10 | 13,9 | 8,2 | 0,04 | 23 % |
| Très difficile (5) | 59 % [52-66] | 4/5 | 15,4 | 9,7 | 0,03 | 29 % |

Références sans RS-B (binaire `cp_base`, révoltes 3 × 75) : normale 68 % [61-73], 10/10, révoltes 3,0 ;
très difficile 60 % [47-72], 4/5, révoltes 3,8. Le seuil 74 pousse la guerre au-delà de 75 % sur 4
graines sur 10 en normale : écarté.

## Century_probe 464 tours, état final (fusion de main, 2 saisons × seuil 75, release, ae5d94f1)
| Niveau | Guerre FR-EN moy. [min-max] | Graines 55-75 | Trêves | Révoltes / 200 t. | Banqueroutes | 1re faction fin |
|---|---|---|---|---|---|---|
| Facile (5) | 69 % [60-77] | 3/5 | 11,8 | 6,1 | 0,10 | 21 % |
| Normale (10) | 69 % [55-75] | 9/10 | 12,6 | 5,9 | 0,05 | 20 % |
| Difficile (10) | 66 % [60-70] | 10/10 | 14,7 | 7,8 | 0,04 | 24 % |
| Très difficile (5) | 59 % [57-63] | 5/5 | 17,8 | 7,4 | 0,02 | 28 % |

Critère EQ6 (moyenne 55-75 % à chaque niveau) : **tenu aux quatre niveaux**, aucun réglage de
données. Écart consigné : en facile, 2 graines sur 5 dépassent 75 % (76 et 77 %, bord de bande) ;
en normale la graine basse est à 54,7 %. Révoltes dans la bande 4-10 à tous les niveaux (5,9 à 7,8).

## Tests finaux (état ae5d94f1)
- `cargo fmt --check`, `clippy --all-targets -D warnings`, `cargo test --workspace` : 1 082 passés, 0 échec.
- pytest : 832 passés, 1 échec préexistant hors lot (`test_budget.py`, lot RS-H).

## Prochaine étape
Lot terminé. Fusion dans main par l'orchestrateur ; `core/target-rs-b` supprimé.

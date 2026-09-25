# EQ2 — équilibre (impôt Haut, troubles bloqués, paliers de départ, Aide féodale, guerre FR-EN)

Branche : `worktree-agent-a781586b50d0b6c1b` (EQ1 + main fusionnés).

## État : terminé (à fusionner)

## Mesures avant / après (après = EQ2 + main fusionné, 11ad6e96)

| Mesure | Avant (EQ1 + main) | Après |
|---|---|---|
| Impôt Haut (balance_probe 8 × 200) | 42-45 % | 31 % (eq2_probe : 32 %) |
| Haut en guerre, trésor au-dessus de la réserve, sans déficit ni dette | majoritaire | 1 % |
| Banqueroutes / faction / décennie (8 × 200) | < 0,5 | 0,36 |
| Province-saisons à 90+ de troubles ; plus longue série | jusqu'à 30 saisons à 100 | 0,00 % ; 0 saison |
| Aide féodale (part des provinces sous édit) | 0 % | 3 % |
| Guerre FR-EN, siècle 5 × 464 | 55 % | 55 % [51-61], 2/5 graines dans 55-75 % |
| Revenu de départ France / Angleterre (tour 0) | 27 579 / 19 008 | 27 522 / 18 999 |

Tests : 617 réussis, 0 échec ; clippy propre.

- [x] Référence : balance_probe 8 × 200 et century_probe 5 × 464 (sur EQ1 + main).
- [x] Paliers de départ : données nettoyées (`data/settlements/prov_*.json`, `data/provinces/*.json`,
      191 colonies, 95 provinces) ; normalisation au démarrage (`GameData::normalize_building_tiers`) ;
      un palier supérieur satisfait les prérequis du palier remplacé (`GameData::has_building`).
- [x] Troubles de province : décroissance 2 + 10 % par saison (données `population.json`).
- [x] Aide féodale : poids de l'argent selon guerre / trésor bas / dette / vassaux ; effets 10+5 → 6+4.
- [x] Impôt Haut : l'IA ne le met que si dette, déficit qui viderait la marge, ou guerre avec trésor
      sous 3 saisons de revenu (`plan_economy`).
- [x] Guerre FR-EN : `pretender_reluctance` 35 → 45 (`data/ai/diplomacy.json`).
- [x] ADR 0049 (0048 est pris par MF1).
- [x] `git merge main` (DF1 pas encore dans main : branche `feature/df1-difficulty` seulement).
- [x] Mesures après fusion (eq2_probe 8 × 200, balance_probe 8 × 200, century_probe 5 × 464).

## Points ouverts
- Guerre FR-EN : `pretender_reluctance` 45 ne change presque rien (55 %) ; les phases de guerre
  s'arrêtent toujours vers la durée minimale. Piste : `min_war_turns` ou la fatigue de guerre.
- Écosse détruite en 1405 et 1444 (graines 5 et 3) : à comparer avec la référence.
- DF1 (difficulté, ADR 0037) n'est pas dans main : refusionner et remesurer quand il y sera.

## Prochaine étape
Fusion dans main par le coordinateur.

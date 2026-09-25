# EQ2 — équilibre (impôt Haut, troubles bloqués, paliers de départ, Aide féodale, guerre FR-EN)

Branche : `worktree-agent-a781586b50d0b6c1b` (EQ1 + main fusionnés).

## État : en cours

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
- [ ] Mesures après fusion (eq2_probe 8 × 200, balance_probe 8 × 200, century_probe 5 × 464), tableau.

## Prochaine étape
Relever les mesures après fusion et remplir le tableau avant/après ci-dessous. Si DF1 arrive dans
main, refusionner et remesurer.

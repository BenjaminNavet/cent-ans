# EQ2 — équilibre (impôt Haut, troubles bloqués, paliers de départ, Aide féodale, guerre FR-EN)

Branche : `worktree-agent-a781586b50d0b6c1b` (EQ1 + main fusionnés).

## État : en cours

- [x] Référence : balance_probe 8 × 200 et century_probe 5 × 464 (sur EQ1 + main).
- [x] Paliers de départ : données nettoyées (`data/settlements/prov_*.json`, `data/provinces/*.json`,
      191 colonies, 95 provinces) ; normalisation au démarrage (`GameData::normalize_building_tiers`) ;
      un palier supérieur satisfait les prérequis du palier remplacé (`GameData::has_building`).
- [x] Troubles de province : décroissance 2 + 10 % par saison (données `population.json`).
- [x] Aide féodale : poids de l'argent selon guerre / trésor bas / dette / vassaux ; effets 10+5 → 6+4.
- [ ] Impôt Haut : diagnostic en cours (`eq2_probe`).
- [ ] Guerre FR-EN graine 5 : à vérifier (référence : 55 %, déjà dans la bande).
- [ ] Mesures après, ADR, tableau.

## Prochaine étape
Terminer le diagnostic de l'impôt Haut (sonde `core/crates/ai/examples/eq2_probe.rs`), puis mesures.

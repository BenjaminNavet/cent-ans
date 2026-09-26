# EP11 — Poussée continue des lignes en mêlée

Branche : worktree agent-a5b7ba925e656a5c7. ADR prévue : 0070.

## État
- Règles `data/rules/battle_push.json` + schéma `battle_push_rules.schema.json` + pytest
  `tools/tests/test_battle_push_schema.py`.
- `sim-battle/src/push.rs` : règles, `PushShape` (champ `Unit::push`), pression, vitesse de recul,
  facteur de mêlée (compression, enroulement), déformation des figurines (bombement, rangs serrés,
  files qui pivotent).
- `sim-battle/src/sim/push.rs` : `resolve_push` (appelé juste avant `resolve_melee` dans `step`),
  blocage (bord, mur, maison, eau profonde, ami), suivi du vainqueur, compression, moral, forme.
- Branchements dans `sim.rs` : `mod push`, champ `push_rules`, appel, facteur dans `melee_damage` ;
  `sim/siege_assault.rs::soldier_poses` déforme les figurines.
- Mesures de référence (avant) dans le bloc-notes : ep7 survey_all, ep9b survey, ep9 survey_crecy.

## Prochaine étape
- Tests `sim-battle/tests/ep11_push.rs` (lourd contre léger, enroulement, compression).
- Mesures après, bench perf, captures `docs/img/ep11/`, ADR 0070.

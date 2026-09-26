# EQ7 — la cavalerie attend son infanterie sous les flèches

Suite de l'ADR 0052. ADR **0083**. Branche `feat/eq7-cavalry-waits`, worktree `../gp-eq7`.

## État
- [x] sonde `tests/eq7_cavalry.rs` (`probe_mixed_battle`, `probe_symmetric_battle`)
- [x] règle `waits_for_foot` dans `plan_horse` (étapes 1b et 5, assaillant seulement),
      `data/rules/battle_horse_wait.json` + schéma + test Python
- [x] mesures : b6 31 → 57/64 ; R4 identique ; R2b 118 → 115/128 ; EP9b vert
- [x] empreintes b6 (graines 3 et 11 passent aux Français), test de garde 16 graines
- [x] suite complète workspace (seul échec : test de durée `m3_grid_ai` sous charge, passe seul), fusionné dans main (d898672c)

## Limites / suites
- R2b perd 3/128 (l'IA active attaque moins tôt avec sa cavalerie contre des archers passifs).
- Pas de manœuvre de flanc distincte (les tireurs pivotent) : voir ADR 0083.

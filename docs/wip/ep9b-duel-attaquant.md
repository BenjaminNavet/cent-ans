# EP9b — Duel de l'attaquant et milice en second échelon

Lot : correction d'EP9 (ADR 0056 § EP9b) mesurée par SG4. Branche `ep9b-duel-attaquant`
(worktree `agent-a7bf63ca963f71d60`), partie d'`integration/night` (main + SG4) : le problème est
mesuré avec SG4 (`sg4_balance.rs`), qui n'est pas encore sur main.

## État

- [x] Reproduction : `sim-battle/tests/ep9b_duel.rs` (60 régiments/camp, plat, sans pieux,
  graines 1-10) ; avant : attaquant 0/10 (défaite par armée brisée à ~340 s).
- [ ] Duel prolongé tant que l'attaquant le gagne (règles en données).
- [ ] Milice (moral bas) en second échelon à l'assaut.
- [ ] Mesures après, ADR 0056 § EP9b, tests verts.

## Mesures avant (integration/night, 2452ef13)

Matrice `sg4_balance` (10 graines), attaquant / défenseur / nuls :
plat pieux 3/7/0 ; plat sans pieux 0/10/0 ; crête 0/10/0 (avec et sans pieux) ; plaine générée
2/8/0 et 5/5/0 ; comme ep1_scale 2/8/0. Crécy-like (`survey_crecy`) : Anglais 11/12.

## Prochaine étape

Suivi des pertes par fenêtre glissante dans `BattleSim`, règle du duel dans `ai.rs::plan_field`.

# CB0 — Extraction des entrées de bataille (état)

Branche : `feat/cb0-battle-input`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
(section CB0). Orchestration : `docs/wip/cb.md` (lecture seule pour cet agent).

## État

- [x] Squelette `game/scripts/battle/battle_input.gd` (signaux, pas encore utilisé par la scène).
- [ ] Test d'équivalence `game/tests/cb0_input_equivalence_test.gd` + golden
      `game/tests/data/cb0_orders_golden.json`, générés sur le code actuel (avant extraction).
- [ ] Extraction vers `battle_input.gd`, test toujours vert, golden inchangé.
- [ ] Sélection rapide (Ctrl/Cmd+A, double clic gauche même type, double clic carte même type +
      recentrage).
- [ ] `smoke.gd` et le test d'équivalence passent tous les deux.

## Prochaine étape

Écrire le test d'équivalence sur le code actuel (avant toute extraction), ajouter
`issued_log` / `log_orders_for_test` à `battle_scene.gd`, générer le golden, commit `test:`
séparé.

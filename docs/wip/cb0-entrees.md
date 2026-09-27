# CB0 — Extraction des entrées de bataille (état)

Branche : `feat/cb0-battle-input`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
(section CB0). Orchestration : `docs/wip/cb.md` (lecture seule pour cet agent).

## État

- [x] Squelette `game/scripts/battle/battle_input.gd` (signaux, pas encore utilisé par la scène).
- [x] Test d'équivalence `game/tests/cb0_input_equivalence_test.gd` + golden
      `game/tests/data/cb0_orders_golden.json` (14 commandes), générés sur le code d'avant
      l'extraction (`log_orders_for_test`/`issued_log` ajoutés à `battle_scene.gd`).
      Piège rencontré : la fenêtre headless par défaut est 64×64 (`root.size` doit être forcé,
      comme les autres tests de capture) et la caméra doit cadrer tout le champ (pas seulement le
      camp du joueur) pour que la cible ennemie de l'étape 7 soit à l'écran.
- [ ] Extraction vers `battle_input.gd` (déjà écrite en local, pas encore commitée/branchée),
      test toujours vert, golden inchangé.
- [ ] Sélection rapide (Ctrl/Cmd+A, double clic gauche même type, double clic carte même type +
      recentrage).
- [ ] `smoke.gd` et le test d'équivalence passent tous les deux.

## Prochaine étape

Brancher `battle_input.gd` dans `battle_scene.gd` (nœud enfant créé en `_ready`, signaux
connectés, fonctions déplacées), vérifier que le test d'équivalence reste vert sans toucher au
golden, commit `refactor:` séparé.

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
- [x] Extraction vers `battle_input.gd` : nœud enfant créé en `_ready`, signaux connectés,
      `_unhandled_input`/`_finish_left`/`_finish_right`/`handle_group_key`/`_on_command`/
      `_next_formation`/`_available_selection`/état glisser+double clic/`_deploy_selection`
      déplacés. Délégations fines gardées sur la scène (`issue`, `handle_group_key`) pour
      `smoke.gd`. Test d'équivalence vert sans toucher au golden.
- [ ] Sélection rapide (Ctrl/Cmd+A, double clic gauche même type, double clic carte même type +
      recentrage).
- [ ] `smoke.gd` (en cours de vérification) et le test d'équivalence passent tous les deux.

## Prochaine étape

Ajouter la sélection rapide dans `battle_input.gd` (Ctrl/Cmd+A, double clic gauche 350 ms même
`type`, `card_double_clicked` même type + recentrage sur `_on_card_double_clicked`), avec des
assertions ajoutées au test d'équivalence sans réécrire le golden existant. Commit `feat:` séparé.

# CB6 — Formations de groupe (état)

Branche : `feat/cb6-group-formations` (depuis `main` cad1ca05). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB6 et décisions après relecture).
Relecture historique : `docs/research/cb6-formations.md`.
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb6`.

## État
- [x] Données : `data/rules/group_formations.json` (6 préréglages, `description_fr` de l'historien) + schéma
      `data/schemas/group_formations.schema.json` + `tools/tests/test_group_formations_schema.py`.
- [x] Golden de non-régression : `core/crates/sim-battle/tests/fixtures/cb6_deploy_golden.json` (écrit sur le
      code d'avant CB6 : démo, armée mixte, grande armée, armée sans infanterie, avec et sans déploiement IA de
      chaque camp, Crécy, Azincourt, Poitiers).
- [x] Cœur : `sim-battle/src/group_formation.rs` (`layout`, `Frame`, `BattleSim::formation_slots`), `deploy()`
      et `ai_deploy` via « Ligne de bataille » (golden identique au bit près).
- [x] Tests cœur `sim-battle/tests/cb6_group_formation.rs` (7).
- [x] Pont : `battle_sim_formation.rs` : `formation_presets()`, `formation_slots(preset_id, ids, x, z, facing)`.
- [x] Godot : `battle_formation_picker.gd` (sélecteur bas droite, Attaque / Défense / Marche, infobulles,
      « Placer en formation » / « Valider la formation » / « Annuler » en déploiement), crochets dans
      `battle_input.gd` (Alt+Maj+1…6, clic droit et aperçu), `battle_groups.gd` (`lock_as`),
      `battle_path_preview.gd` (taille des fantômes des places), `battle_scene.gd` (création).
- [x] Test Godot `game/tests/cb6_group_formation_test.gd` (OK).
- [ ] Script de capture `game/tests/cb6_formation_shot.gd` (+ `--probe`).
- [ ] Vérifications finales (fmt, clippy, tests, release, pytest, Godot).

## Touches (à intégrer à l'aide F1 par CB2)
- Alt+Maj+1 … Alt+Maj+6 : préréglage 1 à 6 (Ligne de bataille, La herse, Trois batailles, Charge de la
  chevalerie, Bataille à pied, Ordre de marche) ; même touche : désactiver.
- Préréglage actif + clic droit : places de la formation devant la sélection (orientée du centre du groupe vers
  le point) ; glisser-droit : ligne de front et orientation du glisser. Fantômes pendant l'appui, puis un `move`
  par régiment et groupe verrouillé (Ctrl/Cmd+G pour le défaire).
- Déploiement : « Placer en formation » (sélection, sinon toute l'armée), clic droit pour déplacer la
  proposition, second appui pour valider.

## Prochaine étape
Script de capture, puis vérifications finales.

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
- [x] Script de capture `game/tests/cb6_formation_shot.gd` (+ `--probe` : OK, 8 places / 8 fantômes, archers
      tournés vers l'intérieur 2/2, dans la zone).
- [x] Vérifications finales (09-27) : `cargo fmt`, `cargo clippy --workspace --all-targets -D warnings` propres ;
      `cargo test --workspace` 1017 réussis, 0 échec ; release `ep13_replay` 8/8, `b6` 12/12, `ep7_historical`
      8/8, `eq7_cavalry` 1/1 ; pytest 802 réussis ; Godot `smoke.gd`, `cb6_group_formation_test`,
      `cb0_input_equivalence_test`, `cb_m1_outline_test`, `cb_m2_path_hover_test`, `cb_m3_queue_test`,
      `cb_m4_range_compare_test`, `cb1_drag_formation_test`, `cb6_formation_shot.gd --probe` : code 0, aucune
      « SCRIPT ERROR ».

## Choix / écarts
- « Ligne de bataille » = placement d'avant CB6 au bit près (golden) : pour cela `layout` travaille en
  coordonnées absolues sur les axes du repère, et la mise en place initiale garde +x comme axe latéral des deux
  camps et la répartition alternée des ailes (droite puis gauche). `formation_slots` (joueur) lit l'axe droit
  de l'orientation demandée et garde l'ordre gauche-droite des régiments (ailes : partie gauche à gauche).
- Le régiment du chef prend la place `general` dès 3 régiments (`general_min_group`) ; la mise en place initiale
  le laisse dans son rôle et `ai_deploy` le recule de 60 m, valeur lue dans le préréglage.
- « Trois batailles » classé `attack` (une seule posture par préréglage). Profondeurs et écarts : ordres de
  grandeur de l'historien, sans relecture chiffrée ; les places proches du bord de zone sont ramenées dans la
  zone (la herse avancée de 35 m se retrouve à +30 si le front est à 30 m du bord).
- Largeur en files seulement pour l'ordre de marche (6 files à pied, 4 montés) ; les autres préréglages gardent
  la formation de chaque régiment.
- Aperçu des trajets : `preview_path` par régiment (`compute_places`, comme un groupe verrouillé CB1) plutôt que
  `preview_group`, qui redistribuerait les places.
- En déploiement, un clic droit sans sélection ne fait rien (comme avant) : le bouton seul vise toute l'armée.
- Pas d'addendum ADR 0095 ici (finalisation par la session principale).

## Touches (à intégrer à l'aide F1 par CB2)
- Alt+Maj+1 … Alt+Maj+6 : préréglage 1 à 6 (Ligne de bataille, La herse, Trois batailles, Charge de la
  chevalerie, Bataille à pied, Ordre de marche) ; même touche : désactiver.
- Préréglage actif + clic droit : places de la formation devant la sélection (orientée du centre du groupe vers
  le point) ; glisser-droit : ligne de front et orientation du glisser. Fantômes pendant l'appui, puis un `move`
  par régiment et groupe verrouillé (Ctrl/Cmd+G pour le défaire).
- Déploiement : « Placer en formation » (sélection, sinon toute l'armée), clic droit pour déplacer la
  proposition, second appui pour valider.

## Capture à produire (session principale)
`godot --path game --resolution 1600x900 --script res://tests/cb6_formation_shot.gd -- --out=docs/img/cb/cb6-herse.png`
(non exécutée ici, non relue).

## Prochaine étape
Relecture de la capture, fusion (après CB3 et CB5, avant CB2) ; CB2 ajoute Alt+Maj+1…6 à l'aide F1 et au
remappage ; addendum ADR 0095.

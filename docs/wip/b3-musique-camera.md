# B3 — musique dynamique de bataille (T4) + caméra de suivi (T6)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B3), spéc T4/T6 dans
`docs/design/2026-09-24-analyse-total-war.md`.

## État — terminé

- [x] Squelette + exploration (`AudioDirector`, `BattleScene`, `BattleCamera`, `get_units()`,
      pistes disponibles sous `game/assets/audio/`).
- [x] `battle_camera.gd` : `follow_unit(id, position_of)` / `stop_follow()` / `is_following()`,
      lissage exponentiel, orbite bouton du milieu pendant le suivi, WASD/bords d'écran et
      `look_at_point` (minicarte, rappel de groupe) annulent le suivi.
- [x] `game/scripts/battle/battle_music.gd` (nouveau) : direction musicale de bataille par
      intensité (approche/engagement/critique/victoire-défaite), hystérésis, transforme
      `music/war.ogg` (volume, filtre passe-bas sur un bus dédié `BatailleMusique`) + couches
      `sfx/sword_clash.ogg` (ambiance de mêlée) et `sfx/march_drum.ogg` (percussion critique) +
      stinger fanfare/chœur (`compute_state` : fonction statique pure, testable seule).
- [x] Câblage `battle_scene.gd` : touche `C` (verrouille/libère le suivi sur la sélection ou le
      général), double-clic sur une carte d'unité = centrer la caméra ; `BattleMusicDirector`
      instancié dans `begin()`, `update()` par tick ; mise en veille d'`AudioDirector`
      (`stop_all()`) à l'entrée en bataille, `refresh_context()` au retour.
- [x] Aide F1 (`battle_hud.gd`) : documente `C` et le double-clic.
- [x] Tests `smoke.gd` (`_check_battle_music_camera_b3`) : `compute_state` sur des dictionnaires
      d'unités simulant calme / contact / camp proche de la déroute / bataille finie ; hystérésis
      (montée immédiate, descente après ≥ 2,5 s stables) ; caméra qui referme la distance sur un
      régiment suivi (ticks `_process` déterministes), libération au clavier (touche `C`),
      double-clic carte → centrage exact (comparaison au sol, `target.y` suit le terrain).
- [x] `cargo` : aucun changement Rust pour ce lot (uniquement GDScript).
- [x] Smoke complet : `godot --headless --path game --script res://tests/smoke.gd` → exit 0,
      aucune fuite (`--verbose` sans « leak »/« orphan »).

## Prochaine étape

Aucune pour ce lot ; B3 est terminé. Suite possible : B4 (effets visuels de charge/tir).

## Touches / interactions ajoutées

- `C` : verrouille la caméra sur le régiment sélectionné (ou le général du joueur) ; nouvel appui
  libère (bascule). Molette de la souris toujours zoom ; bouton du milieu = orbite pendant le
  suivi (panoramique sinon).
- Double-clic sur une carte d'unité (HUD) : centre la caméra sur ce régiment (jump, ne modifie
  pas le suivi).

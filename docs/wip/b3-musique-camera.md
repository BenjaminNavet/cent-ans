# B3 — musique dynamique de bataille (T4) + caméra de suivi (T6)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B3), spéc T4/T6 dans
`docs/design/2026-09-24-analyse-total-war.md`.

## État

- [x] Squelette + exploration (`AudioDirector`, `BattleScene`, `BattleCamera`, `get_units()`,
      pistes disponibles sous `game/assets/audio/`).
- [x] `battle_camera.gd` : `follow_unit(id, position_of)` / `stop_follow()` / `is_following()`,
      lissage exponentiel, orbite bouton du milieu pendant le suivi, WASD/bords d'écran et
      `look_at_point` (minicarte, rappel de groupe) annulent le suivi.
- [ ] `game/scripts/battle/battle_music.gd` (nouveau) : direction musicale de bataille par
      intensité (approche/engagement/critique/victoire-défaite), hystérésis, transforme
      `music/war.ogg` (volume, filtre passe-bas) + couches `sfx/sword_clash.ogg` (ambiance) et
      `sfx/march_drum.ogg` (percussion critique) + stinger fanfare/chœur.
- [ ] Câblage `battle_scene.gd` : touche `C` (verrouille/libère le suivi sur la sélection ou le
      général), double-clic sur une carte d'unité = centrer la caméra ; `BattleMusicDirector`
      instancié dans `begin()`, `update()` par tick ; mise en veille d'`AudioDirector`
      (`stop_all()`) à l'entrée en bataille, `refresh_context()` au retour.
- [ ] Aide F1 (`battle_hud.gd`) : documenter `C` et le double-clic.
- [ ] Tests `smoke.gd` : changement d'état musical déclenché par un contact simulé (fonction pure
      `BattleMusicDirector.compute_state`) ; la caméra suit la position d'un régiment sur
      plusieurs ticks.
- [ ] `cargo` : aucun changement Rust prévu pour ce lot.

## Prochaine étape

Écrire `battle_music.gd`, câbler `battle_scene.gd` (touche C, double-clic carte, AudioDirector
stop/refresh), étendre `smoke.gd`, lancer le smoke test complet.

## Touches / interactions ajoutées

- `C` : verrouille la caméra sur le régiment sélectionné (ou le général du joueur) ; nouvel appui
  libère (bascule). Molette de la souris toujours zoom ; bouton du milieu = orbite pendant le
  suivi (panoramique sinon).
- Double-clic sur une carte d'unité (HUD) : centre la caméra sur ce régiment (jump, ne modifie
  pas le suivi).

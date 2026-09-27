# CB-M4 — Portée au sol et comparaison au survol (état)

Branche : `feat/cb-m4-range-compare` (partie de `main` 88ec35be, qui contient CB-M2 : la tête
initiale du worktree, be631979, précédait CB-M2 et n'avait pas `hover_context`).
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB-M4).
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm4`.

## État : terminé, en attente de relecture visuelle et de fusion (session principale)

- [x] Données : `range_arc.fire_half_angle_deg` (60°) dans `data/rules/battle_hover.json`,
      schéma et test pytest à jour. Le cœur ne restreint pas l'angle de tir (le tireur pivote
      vers sa cible, `turn_towards` au tir) : c'est le secteur dessiné, couvert sans pivoter.
- [x] Cœur `hover.rs` : `RangeArcRules`, `BattleSim::ground_range(unit)` = `effective_range`
      contre le sol à ses pieds (météo, heure, hauteur du tireur : rempart, colline) ; 0 sans
      tir ou sans munitions. Test `sim-battle/tests/cb_range.rs` (2).
- [x] Pont `get_units` : `effective_range`, `fire_arc` (radians) — deux entrées.
- [x] Godot `battle_range_arc.gd` : arc (trait 1,4 m) + bords du secteur plus pâles, sommets à
      `get_walk_height` + 0,35 m, reconstruit seulement si position/cap/portée changent ; au plus
      8 arcs (sélectionnés puis survolés, ennemis compris).
- [x] Godot `battle_compare_panel.gd` : page de vélin UI1 en bas à gauche au-dessus du bandeau,
      9 lignes du dictionnaire `compare`, vert (`GOOD`) / rouge (`RUBRIC`) selon `advantages`,
      « — » pour tir/portée nuls ; appel au cœur au plus toutes les 0,25 s pendant le survol.
      Ennemi survolé = `_hovered_ids` de la scène (terrain `world_hover`, bannière
      `markers.hovered`, carte du HUD). Fermé en rejeu.
- [x] Branchement : `battle_scene.gd` (+11 lignes : deux variables, création, appel dans
      `_update_outlines`). `battle_hud.gd` non modifié (panneau ajouté à `hud.root`).
- [x] Test Godot `game/tests/cb_m4_range_compare_test.gd` ; capture
      `game/tests/cbm4_compare_shot.gd` (`--probe` texte : arc 930 sommets, 100 % à l'écran,
      hauteurs 0,9 à 5,8 m ; panneau 341×238 px dans la fenêtre, au-dessus du bandeau).

## Prochaine étape

Session principale : capture
`godot --path game --resolution 1600x900 --script res://tests/cbm4_compare_shot.gd -- --out=docs/img/cb/cbm4-compare.png`,
jugement visuel, fusion. Conflit attendu avec CB-M3 dans `get_units` (entrées voisines).

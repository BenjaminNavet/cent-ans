# bm/svgconv

Fait : `cent-ans icons svg-png` (12 skill_* + marker_ruins en PNG encre, index mis à jour, pytest), ruines de campagne texturées, curseur de bataille peint (`client/src/ui/cursor.rs`), sceaux et plaques (`client/src/ui/kit/fa.rs`), enluminures du kit (voir README).

Restes :
- Curseur d'attaque de campagne sur une place ennemie (Godot `is_attack_target` : armée ou place) ; Bevy ne le montre que sur une armée hostile. `hand`, `move`, `siege` de `cursors_campaign` ne sont utilisés par aucun script Godot.
- Plaques sur les boutons de difficulté / dates de `faction_select.gd` et `intro_cards.gd` (non portés en boutons plaqués).
- `initial_frame` et `inset_ornate` : thème Godot `InitialFrame` / `InsetFrame` jamais utilisés par un script, donc non branchés.

# VT — villes 1:1 à toutes les hauteurs (vrai territoire)

Demande du joueur (30/09) : la ville « en très rapproché » à toutes les hauteurs de la vue 3D
(sauf parchemin), pour toutes les villes, sans maquette agrandie. ADR 0138.
Worktree `../game_project-vt`, branche `feat/vt`. Coût cloud : 0 $.

## Lots
Vague 1 :
- A (Sonnet) : grille `ground_m` dans `tools/.../geo/towns.py`, schéma, test, nouvelle exécution de `geo towns`.
- B : `game/scripts/map/town_far_builder.gd` (fonctions pures F1/F2/v2) + `tf_far_mesh_test.gd`.
- C : `game/shaders/town_far.gdshader`, `roofscape.gdshaderinc` extrait de `town_building.gdshader`, masque d'enfoncement.
- D : retraits dans `settlement_layer.gd` (maquettes, SZ4b, loupe), étiquettes au sol, clic et anneau sur l'emprise.
Vague 2 : E `town_far_layer.gd` (tuiles, masque) ; F (Sonnet) `max_rig_distance` TownLayer + nettoyage LandmarkCityLayer ; G (Sonnet) consommateurs `model_*`, zones caméra/armées, MapPropScale, hameaux 1:1 ; H (Sonnet) tests.
Vague 3 : I bancs (d = 1100, 150, 30), captures (≤ 6), docs `godot-map.md`.

## État
- [x] Plan, ADR 0138, cette note.
- [ ] Vague 1

## Prochaine étape
Lancer la vague 1 (A-D).

# VT-B — TownFarBuilder (maillage lointain des villes)

État : fait. `game/scripts/map/town_far_builder.gd` (F1, F2, v2, `append`, `mesh_arrays`,
`triangle_count`) + `game/tests/tf_far_mesh_test.gd` (passe).
Mesures (2 134 villes, un fil) : F1 294 tri/ville (max 706, Milan), F2 50,5 (max 56) ;
F1 ≈ 1,15 s + F2 ≈ 0,25 s ; v2 1 347 tri/ville (max 3 613), 22 ms.
Prochaine étape : lot E (`town_far_layer.gd`) consomme l'API ; générer dans `WorkerThreadPool`.
Points ouverts : sol v2 uniforme (`ground_z_m`, pas de grille `ground_m` pour les villes v2) ;
face intérieure des murs F1 omise ; teinte de toit sans information régionale (tuile, chaume
pour les villages).

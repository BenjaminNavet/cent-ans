# RIVIERES — fleuves lisibles à toute distance (boucle juge 2)

Branche : worktree agent-a385069b57faf9288 (partie de main cb89b690f). Ne pas pousser ni fusionner.
Liens non commités : `data/map/pyramid`, `game/assets/models/dn` vers le checkout principal.

## But
Loire, Seine, Garonne, Rhône, Tamise, Meuse, Rhin visibles et continus à d 300, 600, 900 (>= 2 px écran, sans trou entre paliers) ;
bande riparienne de VIS-R5 (`look.riparian`) vérifiée là où elle est prévue. Réglages en données.

## Cause (trouvée)
Dump : rubans majeurs présents et visibles (`river_bands` : 6 px dessinés à d 600, aucun disque fin, `fine_zone` nul). Test de
couleur magenta opaque dans `river.wesl` : la géométrie est bien à l'écran, donc c'est l'alpha du fragment. Cause : `side` est en
unités de demi-largeur vraie, `sw` = demi-largeur d'eau dessinée (> 1 dès que le plancher en pixels élargit le ruban : d >= 200
pour la Seine). `edge = min(|side|, sw)` allait donc de 0 à `sw` et `max(1 - edge, 0)` valait 0 sur presque toute la largeur : `px_edge` = 0,
`soft` = 0, alpha nul hors d'un filet central. Plus d'eau à d 600 et 900 ; VIS-R5 (plancher 4 -> 6 px) l'a aggravé.
Ce n'est ni le disque de géo fine ni `generalised.width_scale`.

## État
- [x] instruire la cause
- [x] correctif : `edge` normalisé par `sw`, `half_px` = pixels de demi-largeur d'eau (`river.wesl`) ; `look.far_color` plus sombre (données)
- [ ] captures avant/après
- [ ] vérification riparienne

## Journal
- Squelette de la note.

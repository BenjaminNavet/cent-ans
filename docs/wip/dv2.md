# DV2 — lieux sans pictogramme (écu + nom), HeraldryAtlas

Branche `feat/dv2` (depuis `feat/dv`, fusionnée avec 666cb40fa). Spec DV, ADR 0124 (Écarts priment).

## État : terminé
- `HeraldryAtlas` (`game/scripts/map/heraldry_atlas.gd`) : atlas des écus extrait de
  `SettlementMarkers` ; retient aussi les factions sans armoiries (plus de recomposition à
  chaque rafraîchissement).
- `SettlementMarkers` allégé : rangs, tailles, visibilité par rang, dé-encombrement, bloc `shield`
  (`size_factor` 0,45 de `size_px`, `gap_px` 1).
- `settlement_markers.json` + schéma allégés ; PNG + `.import` supprimés ;
  `tools/cent_ans_tools/map_markers.py`, son test et la commande `assets map-markers` supprimés ;
  nouveau `tools/tests/test_settlement_markers_schema.py`. Sources peintes
  `tools/assets/map_markers/*.jpg` gardées (orphelines, à décider).
- `settlement_icon.gdshader` : écu seul, ancré sur la position du nom, bas de l'écu `gap_px`
  au-dessus du haut du texte ; pas d'armoiries = quad replié.
- `settlement_layer.gd` : `normal = 1 − strategic_weight` ; maquettes et écus sur toute la vue
  normale ; nom au-dessus de la maquette dès que les maquettes sont affichées ; nom + écu = un
  couple dé-encombré ensemble ; densité par rang seule (constante 380 supprimée) ; picking
  maquette + écu + nom.
- Légende : « Vue normale : maquette et écu », « Petites places », « Vue stratégique : vignette à
  l'encre » (`parchment_town`, `ParchmentOverlay.paint_town`) ; port retiré.
- Tests : `dv_markers_test.gd` (ex-da3), `settlements_render_test.gd` (§ 6 DV2), `ux1_test.gd`,
  `c5_settlements_ui_test.gd` (ordre direct, le picking marche maintenant à 300).

## Points ouverts
- `da7d_overlap_test` : seuil 4 ms dépassé sous charge (6,4 ms ; base feat/dv 5,3 ms à la même
  charge, load ~50). À remesurer sur machine calme.
- `sz4b_colonies_forests_test` : 6 échecs déjà présents sur feat/dv (DV0), hors DV2.
- Rendu jamais vu : taille/placement de l'écu à juger à la capture DV3.

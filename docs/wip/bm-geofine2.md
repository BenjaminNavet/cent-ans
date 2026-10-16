# bm-geofine2 : couche géographique fine dans le cadre composé

## État (16/10)
- Outil `cent-ans geo compose-fine` (`tools/cent_ans_tools/compose/fine_geo.py`, tests
  `tools/tests/test_compose_fine_geo.py`) : transporte les tuiles fines réelles (fleuves, routes
  drapées) et les ancrages fins dans le cadre composé avec la déformation de la composition
  (`ComposedWarp.forward_px`, densification 8 px réels comme `composed_geometry`).
- Prochaine étape : génération complète, vérification (écart ancrages ↔ colonies, Paris), dump Bevy.

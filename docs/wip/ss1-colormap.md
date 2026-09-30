# SS1 — carte de couleur du sol (`cent-ans geo colormap`)

Branche `feat/ss-colormap` (worktree `../gp-ss-cm`), fusion par l'orchestrateur dans `feat/ss`.

## État
- Module `tools/cent_ans_tools/geo/colormap.py` : 5 couches (base régionale, mosaïque Voronoï + lanières + bocage + vigne, auréoles de villes, routes, lacs), peinture par bandes (hachage sur coordonnées monde : indépendant des bandes), BC1 + mipmaps, aperçu JPEG.
- Style `data/map/colormap_style.yaml`, schéma `data/schemas/colormap_style.schema.json`.
- Tests `tools/tests/test_colormap.py` verts (10).

- Cuisson réelle faite : 4 parts `data/map/colormap_bc1_{0..3}.bin` (46 Mo zlib, 117,4 Mo bruts BC1 + mipmaps), `map.json.colormap.bc1`, aperçu `data/map/colormap_preview.jpg` ; ~2 min 20 (peinture 123 s, BC1 16 s).

## Prochaine étape
- SS3 : brancher la texture dans `terrain.gdshader` (`ReliefLandcover.load_gpu_copy(map_dir, meta.colormap.bc1)`).

## Points ouverts
- Taches de forêt de `splat.png` à bords flous (héritées) ; 5 réservoirs modernes du masque peints en terre (Sainte-Croix, Der, Orient, Ebro, Riaño) alors que le shader peut encore y dessiner de l'eau.
- Mer Noire / Caspienne : au-delà de `lakes.max_area_px`, traitées en mer (Ladoga et Onega restent des lacs).

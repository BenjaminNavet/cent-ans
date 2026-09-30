# SS1 — carte de couleur du sol (`cent-ans geo colormap`)

Branche `feat/ss-colormap` (worktree `../gp-ss-cm`), fusion par l'orchestrateur dans `feat/ss`.

## État
- Module `tools/cent_ans_tools/geo/colormap.py` : 5 couches (base régionale, mosaïque Voronoï + lanières + bocage + vigne, auréoles de villes, routes, lacs), peinture par bandes (hachage sur coordonnées monde : indépendant des bandes), BC1 + mipmaps, aperçu JPEG.
- Style `data/map/colormap_style.yaml`, schéma `data/schemas/colormap_style.schema.json`.
- Tests `tools/tests/test_colormap.py` verts (10).

## Prochaine étape
- Cuisson réelle (`uv run --project tools cent-ans geo colormap`), commit des parts `colormap_bc1_*.bin`, `map.json`, `colormap_preview.jpg`.

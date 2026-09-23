# V2 — Terrain de campagne semi-réaliste (branche `visual-v2`)

Plan : `docs/design/visuel-semi-realiste.md` (lot V2), ADR 0004.

## État

- [x] Exagération verticale de la géométrie ramenée de ×14 à ×4,3 (`MapData.HEIGHT_SCALE` 0,02 → 0,006,
      paramètre unique utilisé par terrain, villes, armées, picker, fleuves).
- [x] Heightmap en texture `FORMAT_R16` (filtrage matériel exact + mipmaps) au lieu de LA8.
- [x] Splatmap + champs de distance (frontières, côte) : `tools/cent_ans_tools/geo/splat.py`,
      `uv run --project tools cent-ans geo splat`.
- [x] Textures PBR Poly Haven (`game/assets/textures/terrain/`, `cent-ans geo textures`) + `Texture2DArray`.
- [x] Shader terrain (matériaux, parcellaire, frontières SDF, couleur politique au dézoom) — premier jet.
- [x] Eau (profondeur, vagues, reflet borné, écume), fleuves et côte en traits fins (`terrain_line.gdshader`).
- [ ] Réglages visuels (reflets du ciel sur la mer, teintes), mesure de performance.
- [ ] Captures `docs/img/visuel/v2_*.png`.

Changement minime hors périmètre : `campaign_map.gd` accepte `--stage=map` (capture sans sélection).

## Prochaine étape

Itérer sur les captures (near/mid/far/Alpes/côte), mesurer les i/s, captures finales.

## Contrat avec V3 (végétation) et les autres lots

- `data/map/splat.png` : RGBA8, R = prairie, G = cultures, B = forêt, A = roche/lande ; poids 0-255
  normalisés (somme ≈ 255 sur terre, 0 en mer), même repère que `province_ids.png` (taille lue dans le fichier).
- Échelle verticale : une seule source, `MapData.HEIGHT_SCALE` (unités monde par mètre, 0,006 ≈ ×4,3) ;
  poser les objets avec `MapData.surface_world_at(x, y)`.

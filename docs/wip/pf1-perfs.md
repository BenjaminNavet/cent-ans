# PF1 — performances (préréglages, zoom comté, fuite particules, user://)

## Tâches
1. [~] Préréglages Basse / Moyenne efficaces — code fait (`RenderQuality` clés « scène »), tableau de mesures en cours
2. [x] Zoom comté en Haute : relief fin en blocs culled + LOD selon la taille des triangles ; filtre d'ombre de carte moyen
3. [ ] Fuite « ParticlesShaderRD were never freed » (export à tester)
4. [x] user:// du jeu exporté isolé + copie unique des fichiers du joueur (ADR 0031) — à vérifier sur l'export

## État
- `FineTerrainJob` : tuile fine découpée en blocs de 64 quads, 3 LOD Godot (pas ×2, ×4, ×8) ;
  clé = min(erreur de hauteur, arête du niveau plus fin / 6 px). `TerrainBuilder` : nœuds `Fine_<i>`.
- `RenderQuality` : clés `fine_relief`, `fine_lod_bias`, `terrain_near`, `veg_*`, `map_shadow_*`,
  `battle_lod`, `grass`, `particles` ; groupe `render_quality_client` (`apply_render_quality`) ;
  particules réduites à la création (`node_added`).
- `--map-ab` : attend relief fin + végétation, revise le point, primitives/dessins, `--ab-shots=`.
- Test : `godot --headless --path game --script res://tests/pf1_quality_test.gd`.

## Mesures (brouillon)
d=150 Haute, Metal, plafonné : avant 18,6 / p95 29,9 ms ; après 16,68 / p95 17,0 (2 passes).
Non plafonné : avant 20,8-21,6 ms, 7,25 M prim. ; après 16,9-17,8 ms, 6,0 M prim.

## Prochaine étape
Tableau carte (4 zooms × 4 niveaux, ×2) et bataille (20/50 régiments, ×2) : scratchpad
`map_table.sh` / `battle_table.sh` ; puis export et fuite de particules.

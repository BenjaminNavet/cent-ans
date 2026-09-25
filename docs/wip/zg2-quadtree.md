# ZG2 — moteur de relief streamé en quadtree (ADR 0036)

Branche `zg2-quadtree` (worktree agent). Liens symboliques non versionnés : `data/map/pyramid`,
`tools/geo/raw` → dépôt principal.

## Architecture retenue (résumé, détail dans `docs/godot-map.md`)
- `ReliefPyramid` : manifeste (RLE → ensembles de tuiles par étage), E0 = `data/map/height/`.
- `ReliefQuadtree` : quadtree CDLOD sur toute la carte (racine 4096 unités, profondeur n,
  étage de tuile L = n − 4), sélection par distance équivalente à l'erreur à l'écran
  (espacement des sommets projeté ≤ `max_vertex_px`), morphing géomorphe + jupes, patch fixe
  partagé (64 quads, demi-patch 32 pour les quadrants), pages `Texture2DArray` R16 + mipmaps,
  paramètres de nœud en `instance uniform`, décodage PNG dans `WorkerThreadPool` (Rust
  `GameDataStore`, repli `Png16`), LRU, fondu des pages.
- Shader : `relief_quadtree.gdshaderinc` + 3 crochets d'une ligne dans `terrain.gdshader`.

## État
- [ ] ReliefPyramid
- [ ] ReliefQuadtree (sélection, pages, décodage, LRU)
- [ ] Shader include + crochets
- [ ] Intégration TerrainBuilder (surface_height_at, chunk_surface_changed, grilles)
- [ ] Pyramide synthétique de test + test headless
- [ ] Banc `--bench-map`, captures `docs/img/zg2/`
- [ ] Docs godot-map.md, addendum ADR

## Prochaine étape
Implémenter ReliefPyramid puis ReliefQuadtree.

## Mesures
(à venir)

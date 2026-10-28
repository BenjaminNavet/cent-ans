# ADR 0790 — LOD1-2 refaits par voxels et texture recuite (classes massives)

Date : 2026-10-28. Statut : accepté.

## Contexte

Essai de Meshy Remesh (article Meshy « Blender MCP », 2 × 5 crédits) contre nos LOD de `dn-ingest` sur
`vineyard_hut` (30 161 → 1 500 triangles) et `rock_cliff_face_limestone` (31 212 → 3 000) : nos LOD1 sont
déchiquetés (toit en éclats, échardes, trous), ceux de Meshy propres. Cause : la décimation par effondrement
d'arêtes de Blender bute sur les maillages Meshy/TRELLIS faits de coques superposées (tuiles, pierres) et
découpés aux coutures d'UV ; les soudures de repli de `decimate()` arrachent alors la surface.
MeshLab (décimation qui respecte la texture) bloque à 4 000 triangles sur les coutures et son module Python
plante à l'export.

## Décision

Option de classe `lod_method` (`collapse` par défaut, `voxel_bake`), surchargeable par `--lod-method`.
`voxel_bake` refait chaque LOD1-2 à partir du LOD0 : remaillage en voxels (`lod_voxels`, 200 sur la plus
grande dimension) qui fond les coques en une surface fermée, effondrement au budget, UV neuves (projection
intelligente), couleur de base recuite depuis le LOD0 (Cycles, sélection vers actif), texture de la classe au
LOD1 et moitié au LOD2. Le modèle est d'abord épaissi (1,5 voxel) pour que toits et murs fins survivent.
Si l'effondrement cale (arches et fenêtres : surface trouée de partout), nouvel essai à grille plus grossière
(200 → 120 → 72 voxels) qui referme les petites ouvertures ; la portée de cuisson suit la taille du voxel.
Une soudure de repli sur la coque a été essayée puis écartée : elle déchiquette le LOD2. Si le résultat dépasse le budget (pièces fines : tentes, piquets), repli sur
`collapse`. Le manifeste note `lod_methods` par LOD.

Classes en `voxel_bake` : `house`, `major_building`, `rock`, `bridge`, `landmark_city`. Végétation, champs,
accessoires, charrettes, navires, engins restent en `collapse` : la vigne et le cyprès n'y gagnent rien et
les voxels effacent les pièces fines.

Les assets déjà ingérés sont refaits par `tools/experiments/lod_voxel_rebake.py` (LOD0 intact).

## Conséquences

- Gratuit, ≈ 3 s par LOD ; qualité comparable à Meshy Remesh, qui reste une option payante au cas par cas.
- Les LOD1-2 refaits ont une texture propre (UV neuves) au lieu de partager celle du LOD0.
- Le LOD0 des assets non fidèles (TRELLIS) est encore décimé par `collapse` : non examiné ici.

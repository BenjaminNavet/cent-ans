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

## Complément (28/10) : LOD0 TRELLIS, accessoires et arbres

Contrôle des LOD0 TRELLIS (non fidèles) : maisons, grands bâtiments, navires, animaux, charrettes sont fidèles
au brut (réduction ≤ 5×). 180 modèles sont réduits plus de 10× (110 accessoires, 49 arbres, 19 roches) et
fondent (`ruin_castle` 62 816 → 1 500). `voxel_bake` refait donc aussi le LOD0 quand le brut dépasse
`lod0_voxel_ratio` (5) fois son budget ; en deçà, l'effondrement garde les UV d'origine, plus nettes.
Essais : four à chaux équivalent ; chanvre, échafaudage, ruine, arganier nettement mieux ; oranger équivalent.
`prop` et `tree` passent en `voxel_bake` ; la grille peut descendre à 20 voxels pour les LOD2 de 150 triangles.
Navires, charrettes, engins, animaux, champs restent en `collapse` (LOD1-2 non examinés).

## Amendement (28/10) — textures recuites vides

Le premier rebake laissait des textures à 50-70 % noires : le dépliage `smart_project` à 66° découpait la
coque en escalier en milliers d'îlots minuscules, sur un fond noir avec 4 px de marge ; de loin, les villes
paraissaient gris foncé uniforme. Désormais : angle de dépliage 80° (`bake_uv_angle_deg`), empaquetage
`pack_islands` concave avec rotation, marge de cuisson de 16 px en extension, et les texels non écrits
(fond, rayons perdus) sont remplis depuis leurs voisins puis à la couleur moyenne. Le script rend la
part de texels cuits (`bake_coverage`) ; `check_result` refuse une couverture sous `bake_coverage_min`
(0,5). Cathédrale ouest LOD1 : 68 % → 0,2 % de noir, couverture 90 %.

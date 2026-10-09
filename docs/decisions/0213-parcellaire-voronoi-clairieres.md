# 0213 — Parcellaire du sol : parcelles de Voronoi, régions dominantes, clairières

## Contexte
Le damier de carrés alignés vu à distance 45 à 120 venait de `hb_ground.gdshaderinc` (HB3) : grille
tournée par région de 60 px carte (coutures nettes entre régions), cellules de 0,4 px carte, culture
tirée au hasard par parcelle (patchwork). `terrain.gdshader` `field_at` (V2b) n'est visible que de très
près et n'est pas en cause.

## Décision
- Parcelles de Voronoi à poids additifs (tailles inégales), bords gauchis à deux échelles ; étirement
  `strip` plafonné à 2,2 ; `hb_cell_scale` 1,2 -> 3,2 (champs « maquette », révise ADR 0158 sur ce point).
- Régions de culture dominante (Voronoi large, `hb_region_cells` = 16 px) : orientation et tirage de
  culture dominante partagés (70 % des parcelles) ; le reste varie.
- Clairières (`hb_clearing_splat`) : au XIVe s. la forêt domine ; hors finages (`terroir.r`) et fonds
  plats, la part de cultures du splat tombe à `hb_clear_floor` (0,3 ; 0,55 dans un paysage ME8) et
  redevient prés/landes. La forêt elle-même reste semée par `core` (DN-FORET).
- Pas de texture fal ni de dépense : les matières de culture existantes suffisent.

## Conséquences
Pas de périodicité visible à 45 / 120. Le brûlis (TB4) suit toujours la parcelle (`parcel`).
`field_at` (très près) garde sa trame ; à reprendre si le joueur la voit de près.

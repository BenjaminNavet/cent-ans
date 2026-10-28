# LOD voxel_bake (ADR 0790) — reprise

État (28/10) : TERMINÉ. 319/321 assets des classes massives ont leurs LOD1-2 refaits (épaississement,
voxels 200 → 120 → 72, texture recuite) ; manifeste commité. Contrôles visuels bons (cabane, falaise, château
byz, cathédrale, entrepôt du port, porte d'eau).
Non traités : `env_glacier_ice_tongue`, `env_iceberg_floe` (budgets hors classe, l'outil les saute).
Ouvert : LOD0 des assets TRELLIS non fidèles toujours en `collapse` ; les glb (`game/assets/models/dn/`)
sont hors git, leur publication (release des modèles) attend le feu vert du joueur.

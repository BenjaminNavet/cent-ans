# LOD voxel_bake (ADR 0790) — reprise

État (28/10) : TERMINÉ. 563/565 assets (house, major_building, rock, bridge, landmark_city, prop, tree)
refaits : LOD1-2 en voxel_bake, LOD0 aussi quand le brut dépasse 5× son budget (275 LOD0 refaits).
Non traités : `env_glacier_ice_tongue`, `env_iceberg_floe` (budgets hors classe).
Publication du paquet de modèles (v6, ≈ 1,4-1,7 Go) : feu vert du joueur donné, mais bloquée par le
classifieur ; le joueur la lance : `uv run --project tools cent-ans art models-update --publish`.
Ouvert : LOD1-2 des navires, charrettes, engins, animaux, champs (toujours `collapse`, non examinés).

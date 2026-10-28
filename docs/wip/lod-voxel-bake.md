# LOD voxel_bake (ADR 0790) — reprise

État (28/10) : TERMINÉ. 563/565 assets (house, major_building, rock, bridge, landmark_city, prop, tree)
refaits : LOD1-2 en voxel_bake, LOD0 aussi quand le brut dépasse 5× son budget (275 LOD0 refaits).
Non traités : `env_glacier_ice_tongue`, `env_iceberg_floe` (budgets hors classe).
Publication du paquet de modèles v6 : envoi lancé le 28/10 puis arrêté par le joueur (connexion lente, ≈ 0,27 Mo/s, ≈ 1 h 25 pour 1 385 Mo) ;
Release models-v6 créée vide sur GitHub, hébergement resté en v5. Relancer : `uv run --project tools cent-ans art models-update --publish` (écrase la v6).
Ouvert : LOD1-2 des navires, charrettes, engins, animaux, champs (toujours `collapse`, non examinés).

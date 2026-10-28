# LOD voxel_bake (ADR 0790) — reprise

État (28/10, après-midi) : correctif des textures recuites vides (amendement ADR 0790) commité ; rebake
complet relancé en arrière-plan : `uv run --project tools python tools/experiments/lod_voxel_rebake.py
<scratch> -j 4` (≈ 563 assets). Le rapport `report.json` du dossier scratch liste les échecs.
Prochaine étape : vérifier le rapport, la part de noir des textures (< 5 %), commiter le manifeste, puis
publier le paquet de modèles v6 (`uv run --project tools cent-ans art models-update --publish`, connexion
lente ≈ 1 h 25, la Release models-v6 existe vide).
Non traités : `env_glacier_ice_tongue`, `env_iceberg_floe` (budgets hors classe).
Ouvert : LOD1-2 des navires, charrettes, engins, animaux, champs (toujours `collapse`, non examinés).

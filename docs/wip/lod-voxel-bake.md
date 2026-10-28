# LOD voxel_bake (ADR 0790) — reprise

État : code et données commités ; régénération en cours par
`uv run --project tools python tools/experiments/lod_voxel_rebake.py <scratch> -j 4` (≈ 320 assets des classes
massives). Rapport : `<scratch>/report.json` (ok / repli collapse / échec).
Prochaine étape : relire le rapport, commiter `data/art/dn_manifest.json` ; les glb (`game/assets/models/dn/`)
sont hors git, leur publication (release des modèles) demande le feu vert du joueur.
Ouvert : LOD0 des assets TRELLIS non fidèles toujours décimés par `collapse`.

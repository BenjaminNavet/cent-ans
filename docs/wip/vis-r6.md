# VIS-R6b — lieux génériques Meshy (famille ouest)

## État
- Squelette : nouvelle classe d'ingestion `settlement_maquette` (`data/art/dn_ingest_classes.json`).
- Entrées : `~/dev/cent-ans-raw/vis-r6/meshy/{village,castle,town,abbey}_west.glb` (meshy-6, ≈ 1,4 M triangles, hors dépôt).
- Worktree : `data/map/pyramid` lié, `game/assets/models/dn` et packs 2048 clonés (`cp -c`) depuis le checkout principal.

## Prochaine étape
1. Ingestion des 4 modèles (mêmes ids que les existants, `--no-grade`, classe `settlement_maquette`).
2. Densité (`town_maquettes.json` visibility village 900 → 350), loi d'échelle des voisins, teinte régionale.
3. Captures (≤ 4), ADR, tests.

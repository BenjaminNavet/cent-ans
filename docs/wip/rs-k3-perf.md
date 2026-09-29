# RS-K3 — perf carte (suite de RS-K2) : étiquettes, dé-encombrement, hameaux

Branche `feat/rs-k3-perf` (depuis `main` 104f7a43). Aucun changement Rust visé (dylib copiée),
pas d'ADR (application d'ADR 0051 : étalement sur 1-2 images). Liens non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal.

## Cibles (restes de RS-K2)
- `settle/labels` pire ~8 ms : toutes les étiquettes recalculées quand l'échelle verticale change.
- `settle/declutter` p99 ~8,6 ms.
- `settle/hamlets` pire ~15 ms : construction d'une tuile isolée.

## État
- Squelette : note créée ; mesures de base à faire.

## Prochaine étape
Banc de base (3 passes alternées), puis correctifs.

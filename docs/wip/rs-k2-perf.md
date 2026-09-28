# RS-K2 — perf carte (suite de RS-K) : colonies aux pas d'échelle, effets de vie

Branche `feat/rs-k2-perf` (depuis `main` 5aae540f). Pas de changement Rust (dylib copiée).
Liens non versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal.

## Cibles
1. `settle/scale`, `settle/labels`, `settle/hamlets` (p99 17-27 ms à chaque pas d'échelle) :
   mémoriser `_effective_scale` par passe, ne recalculer que ce qui change.
2. `life/smoke_rewrite` (p99 ~19 ms), `life/reground` (pire ~50 ms) : étaler sur plusieurs
   images (ADR 0051).

## État
- Squelette : note créée, lecture du code en cours.

## Prochaine étape
Mesure de base (banc `--stage=map --hide-armies --bench-map --bench-probe`).

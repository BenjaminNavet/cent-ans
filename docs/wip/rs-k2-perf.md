# RS-K2 — perf carte (suite de RS-K) : colonies aux pas d'échelle, effets de vie

Branche `feat/rs-k2-perf` (depuis `main` 5aae540f). Pas de changement Rust (dylib copiée).
Liens non versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal.

## Cibles
1. `settle/scale`, `settle/labels`, `settle/hamlets` (p99 17-27 ms à chaque pas d'échelle) :
   mémoriser `_effective_scale` par passe, ne recalculer que ce qui change.
2. `life/smoke_rewrite` (p99 ~19 ms), `life/reground` (pire ~50 ms) : étaler sur plusieurs
   images (ADR 0051).

## Diagnostic (sous-sections ajoutées : `settle/scale/{place,absorb,label_h}`,
`settle/hamlets/{scale,build}`)
- `settle/scale` : `place` ~10 ms p99 et `label_h` ~11 ms p99 ; `absorb` < 0,5 ms.
- `settle/hamlets` : tout dans les constructions de tuiles (pas l'échelle, < 0,4 ms) ; sonde
  temporaire : le tri des hameaux (`covered_by_landmark`, `on_settlement_model`, hachage) domine,
  refait à chaque recalage de relief.
- `settle/labels` : étiquettes du palier moyen, caméra relue pour chaque étiquette.

## Correctifs (en cours de mesure)
- `SettlementLayer` : `_effective_scale` mémorisée (distance courante, taille réelle),
  exagération calculée une fois par distance (`MapPropScale.settlement_scale_with`, même
  formule) ; affectations de nœud seulement si la valeur change, une écriture de position par
  étiquette ; exclusions et graines des hameaux mémorisées (oubliées quand maquettes/ancrages
  changent) ; échelle écran des étiquettes lue une fois par image.
- `LifeEffects` : réécriture des panaches par tranches de 1 600 points par image avec curseur,
  colonie calculée une fois pour ses points ; recalage des sols en file par tranches de ≥ 700
  points, hauteurs en un appel `surface_heights_at`. Hors image ouverte (tests) : tout d'un coup.

## Prochaine étape
Tests puis A/B 3 passes (`scratchpad/rsk2/ab.sh`).

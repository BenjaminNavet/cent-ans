# RS-K2 — perf carte (suite de RS-K) : colonies aux pas d'échelle, effets de vie

Branche `feat/rs-k2-perf` (depuis `main` 5aae540f). **Aucun changement Rust** (dylib copiée), pas
d'ADR (application d'ADR 0051 : étalement sur 1-2 images). Liens non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal.

## Diagnostic (sous-sections du banc : `settle/scale/{place,absorb}`, `settle/hamlets/{scale,build}`)
- `settle/scale` : réécriture des ~570 maquettes (affectations de nœud : propagation à toute la
  maquette) et de toutes les étiquettes à chaque pas ; `absorb` < 0,5 ms.
- `settle/hamlets` : pas l'échelle (< 0,4 ms) mais les constructions de tuiles de hameaux,
  refaites à chaque recalage de relief : le tri des hameaux (`covered_by_landmark`,
  `on_settlement_model`, hachage du nom) dominait.
- `settle/labels` : étiquettes du palier moyen, caméra et boîte des marqueurs relues par étiquette.
- `life/*` : boucles par point (panaches, moulins) et hauteurs lues point par point.
- Piège de mesure : un micro-banc lancé trop tôt après le chargement mesure la contention des
  tâches de fond (écarts ×3 sur du code identique) ; attendre ~400 images.

## Correctifs
- `SettlementLayer` : `_effective_scale` mémorisée (distance courante, taille réelle ; entrée
  oubliée par `_forget_scale` quand place, rayons ou rayon réel changent), exagération calculée
  une fois par distance (`MapPropScale.settlement_scale_with`, même formule que
  `settlement_scale`). Pas d'échelle → tour de réécriture étalé (`PLACE_SLICE` = 300 maquettes
  par image dans une image ouverte, curseur, masquage à chaque tranche ; `flush()` termine le
  tour) ; seules les étiquettes posées sur une maquette ordinaire suivent (palier près) ;
  affectations de nœud seulement si la valeur change, une écriture de position par étiquette.
  Exclusions et graines des hameaux mémorisées (oubliées dans `_fit_models`, les ancrages fins,
  `replace_model`). Échelle écran des étiquettes lue une fois par image.
- `LifeEffects` : tours de réécriture étalés avec curseur (panaches 1 600 / image / famille,
  moulins 500), colonie calculée une fois pour ses points consécutifs ; recalage des sols en file
  (tranches ≤ ~400 points), hauteurs en un appel `surface_heights_at`. Hors image ouverte
  (tests, outils) : tout d'un coup. Pas besoin de Rust.

## Mesures (A/B 3 passes alternées, base = `main` + sous-sections du banc, médianes des passes)
Passe finale (charge 5-17) :
| | base | RS-K2 |
|---|---|---|
| image p50 / p99 / pire (ms) | 17,6 / 62,2 / 123,0 | 17,6 / 34,3 / 51,5 |
| images > 50 ms | 50 | 1 |
| descente p99 / scripts p99 | 75,3 / 65,3 | 36,1 / 28,6 |
| `settle/scale` p99 / pire | 16,3 / 43,2 | 3,9 / 5,9 |
| `settle/labels` p99 / pire | 14,1 / 25,5 | 4,7 / 7,6 |
| `settle/hamlets` p99 / pire | 14,7 / 30,2 | 2,4 / 15,2 |
| `life/smoke_rewrite` p99 / pire | 12,3 / 30,8 | 3,3 / 6,7 |
| `life/reground` p99 / pire | 9,1 / 52,5 | 6,3 / 10,6 |
| `life/mill_rewrite` p99 / pire | 3,3 / 6,8 | 4,0 / 6,5 |
| `map.settlements` p99 / pire | 29,0 / 73,5 | 10,3 / 25,7 |
| `map.life` p99 / pire | 23,4 / 52,8 | 8,8 / 17,5 |
Passes précédentes (charge 12-60) : même sens (image p99 80,5 → 49,6 et 84,1 → 50,9).

## Tests
smoke, sz4_prop_scale, sz4b_colonies_forests, cv1_campaign_life, rs_k_finest_levels,
pb3g_quadtree : OK.

## Restes
- `settle/labels` pire ~8 ms : recalcul de toutes les étiquettes quand l'échelle verticale change
  (palier moyen) ; étalable de la même façon si besoin.
- `settle/hamlets` pire ~15 ms : une construction de tuile isolée (au moins une par image).
- `settle/declutter` p99 ~8,6 ms inchangé (hors périmètre).

## Prochaine étape
Lot terminé : fusion par l'orchestrateur (pas de dylib à reconstruire).

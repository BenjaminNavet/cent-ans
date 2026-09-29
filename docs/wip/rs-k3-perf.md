# RS-K3 — perf carte (suite de RS-K2) : étiquettes, dé-encombrement, hameaux

Branche `feat/rs-k3-perf` (depuis `main` 104f7a43). Aucun changement Rust visé (dylib copiée),
pas d'ADR (application d'ADR 0051 : étalement sur 1-2 images). Liens non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal.

## Cibles (restes de RS-K2)
- `settle/labels` pire ~8 ms : toutes les étiquettes recalculées quand l'échelle verticale change.
- `settle/declutter` p99 ~8,6 ms.
- `settle/hamlets` pire ~15 ms : construction d'une tuile isolée.

## Diagnostic (sous-sections ajoutées : `settle/labels/vscale`, `settle/declutter/{prep,run}`,
`settle/hamlets/{prep,heights,mesh}`)
- `settle/labels` : tout vient de `labels/vscale` (réécriture des ~570 étiquettes à chaque pas
  d'échelle verticale ; au palier moyen, `surface_world_at` relu par étiquette).
- `settle/hamlets` pire : `hamlets/prep` ; la première tuile construite chargeait les maillages
  des hameaux (`ModelLibrary.hamlet_meshes`, scènes + remappage : ~15 ms une fois).
- `settle/declutter` : la boucle (`run`) sur ~1 200 colonies ; `prep` (épinglés, séquence) < 1 ms.

## Correctifs
- Étiquettes : pas d'échelle verticale → tour de réécriture étalé (`LABEL_SLICE` = 300 par image
  dans une image ouverte, curseur ; un nouveau pas relance un tour) ; recalages de morceaux
  toujours immédiats ; altitude du sol du palier moyen lue une fois (`_label_ground_m`, même
  valeur que `surface_world_at`) ; écritures de position / décalage seulement si elles changent.
- Hameaux : cause réelle du pire = tri des hameaux d'une tuile froide (`on_settlement_model` :
  ~25 µs par hameau, 3 × 3 morceaux relus) ; emprises des maquettes voisines lues une fois par
  morceau (`_on_model_disk`, `_model_disks`, même test, 0 écart sur les 2 988 hameaux) : tuile
  froide 28,6 → 7,9 µs par hameau (headless). Maillages chargés au `setup` ; partie fixe d'une tuile (hameaux posés, points des
  hauteurs, lacets, échelles) mémorisée (`_hamlet_tiles`, vidée avec les exclusions) : une
  reconstruction ne relit que les hauteurs et la dévastation.
- Dé-encombrement : passe étalée (`DECLUTTER_SLICE` = 600 colonies par image, 2 images) avec
  l'état de caméra figé au début (projection recalculée, formule de `unproject_position` /
  `is_position_behind`) : tranches cohérentes même caméra en mouvement. `declutter()` public :
  passe complète immédiate (tests).

## Mesures (A/B alternées, base = 5627d860 = `main` + sous-sections, médianes des passes ; charge 3-10)
Passe finale `k3b` (base 2 passes valides, une base sortie sans rapport ; new 3) :
| | base | RS-K3 |
|---|---|---|
| image p50 / p99 / pire (ms) | 40,8 / 153,0 / 189,0 | 39,8 / 155,2 / 224,4 |
| images > 50 ms | 256 | 183 |
| descente scripts p99 | 44,7 | 32,0 |
| `map.settlements` p99 / pire | 19,4 / 28,9 | 8,0 / 11,2 |
| `settle/labels` p99 / pire | 8,9 / 13,1 | 1,8 / 2,5 |
| `settle/declutter` p99 / pire | 16,7 / 22,7 | 5,8 / 7,9 |
| `settle/hamlets` p99 / pire | 3,1 / 8,9 | 1,8 / 3,0 |
Passe `k3` (3 + 3, avant le cache des emprises) : labels 8,8/12,1 → 2,0/3,0 ; declutter
15,4/19,6 → 5,4/7,8 ; hamlets 2,5/9,7 → 2,7/9,8 (inchangé : d'où le cache des emprises).
Images p99 / pire : dominées par la charge (p50 ~40 ms, machine partagée) et hors de
`settle/*`, pas significatives ici.

## Tests
smoke, sz4_prop_scale, sz4b_colonies_forests, cv1_campaign_life, rs_k_finest_levels,
pb3g_quadtree, settlements_render : OK. `da7d_overlap_test` : seuil 4 ms du dé-encombrement
dépassé sous charge par la base comme par RS-K3 (5,8-7,3 ms contre 5,6-6,1 ms, passe complète
synchrone), pas une régression.

## État
Lot terminé, aucun changement Rust nécessaire.

## Prochaine étape
Fusion par l'orchestrateur (pas de dylib à reconstruire).

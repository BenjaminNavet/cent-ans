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
- Hameaux : maillages chargés au `setup` ; partie fixe d'une tuile (hameaux posés, points des
  hauteurs, lacets, échelles) mémorisée (`_hamlet_tiles`, vidée avec les exclusions) : une
  reconstruction ne relit que les hauteurs et la dévastation.
- Dé-encombrement : passe étalée (`DECLUTTER_SLICE` = 600 colonies par image, 2 images) avec
  l'état de caméra figé au début (projection recalculée, formule de `unproject_position` /
  `is_position_behind`) : tranches cohérentes même caméra en mouvement. `declutter()` public :
  passe complète immédiate (tests).

## État
- Correctifs commités ; tests en cours ; banc A/B à faire (`ab3.sh`, base = 5627d860).

## Prochaine étape
Banc A/B 3 passes alternées (`runs3/`), tableau avant/après ici.

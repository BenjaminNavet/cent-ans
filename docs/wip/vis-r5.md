# VIS-R5 — Eau : bathymétrie et rivières de carte

Spec : `docs/audits/2026-10-29-visuel-campagne.md` (tableau d, ligne R5 ; G2-2, G2-8 ; bible c). ADR : `docs/decisions/0806-vis-r5-eau.md`.
Branche : worktree agent-a6e0f261dba299bab (partie de main 8323c8cdc). Ne pas pousser ni fusionner.

## État : FAIT (à vérifier au merge)
- Outil `cent-ans geo bathymetry` + `data/map/bathymetry.{png,json}` (GEBCO 2025, 0,3 Mo), schéma, test, crédits.
- `sea.wesl` lit la bathymétrie (liaison 13) ; `data/fx/sea_life.json` `eau.bathymetry` ; liseré cyan retiré.
- Rivières : `river_display.json` (`major_min_px` 6, `fine.min_px` 6, `look.riparian`), `river.wesl` bande en 3 zones.
- Mesures : camp_calais, boîtes fixes 120x60 px (voir rapport).

## Points ouverts
- Dans le worktree, il faut lier `data/map/pyramid` (gitignoré) vers le checkout principal pour que les captures montrent le terrain.
- Aucune rivière visible à d 600 (camp_0600) avant comme après : cause non trouvée (palier fin masquant le moyen ? tuiles ?). À instruire.
- Berge claire / ripisylve / plaine alluviale non vérifiées visuellement (palier moyen, non visible dans les vues capturées).
- Gironde : coin triangulaire = forme du masque de terre composé (`data/map/compose/`), à redessiner dans l'éditeur de carte.
- Pas de bathymétrie dans les lacs ; estrans toujours limités aux 8 zones de `tide.zones`.

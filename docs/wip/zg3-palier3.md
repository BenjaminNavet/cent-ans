# ZG3 — relief palier 3 (E5-E7) sur les zones de détail

Lot ZG3 de l'ADR 0036. Branche `worktree-agent-a17b7688304a10059`.
Commande : `uv run --project tools cent-ans geo detail-dem [--zones id,...] [--force]`.
Doc : `docs/geo.md` § « Relief palier 3 ». Crédits : `CREDITS.md`.

## État
- [x] Services vérifiés (25/09) : IGN WMS-R (GeoTIFF float32, 5010 px max) ; EA WCS 2.0.1
  (`scalefactor`) ; AHN PDOK (`scalesize`, lent) ; Flandre WCS (multipart, ≤ 2000² px, pas de
  mise à l'échelle) ; Wallonie : WMS rendu seulement → repli GLO-30 ; Overpass OK.
- [x] Code : `detail_dem.py`, `detail_sources.py`, `anachronisms.py`, CLI, tests
  (`tools/tests/test_detail_dem.py`, 24 tests).
- [x] Zones : 34 dans `data/map/detail_zones.json` (schéma étendu : `extra_sources`,
  `level_half_km`).
- [x] Essais : château-Gaillard (IGN), Douvres (EA), Tournai (GLO-30) cuits et contrôlés
  (raccords sans marche).
- [x] Bruges + L'Écluse (Flandre + AHN) cuits.
- [x] Cuisson complète des 34 zones (25/09, 743 s de cuisson, téléchargements compris
  avant : ~30 min), manifeste (lignes 5-7), aperçus `docs/img/zg3/` (Calais, Poitiers,
  Château-Gaillard). Aucun échec de service, aucun repli imprévu.

## Prochaine étape
Lot terminé. **Quand ZG1 aura cuit E3-E4**, relancer
`cent-ans geo detail-dem --force` (~15 min, tout en cache) : le fondu de bord des zones
se raccorde à l'ancêtre existant au moment de la cuisson (aujourd'hui E2/E1).

## Reprise
Idempotent : relancer la même commande ; les blocs téléchargés et les zones à jour
(`tools/geo/raw/detail/<zone>/done_E<k>.json`) sont sautés.

## Tailles (25/09)
- Tuiles : E5 378 (50,9 Mo), E6 824 (89,6 Mo), E7 908 (83,0 Mo) — **0,22 Go** (plafond 1,5 Go).
- Bruts `tools/geo/raw/detail/` : **1,42 Go** (plafond 8 Go), dont ~60 Mo de travail
  (`_work/`, moyennes 2 × 2 pour les parents, supprimables).
- Part effacée (anachronismes) dans l'emprise : 0 % (Azincourt E7) à 18 % (Harfleur E5),
  10-16 % dans les grandes villes.

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
- [ ] Bruges + L'Écluse (Flandre + AHN) — en cours
- [ ] Cuisson complète des 34 zones, manifeste, aperçus `docs/img/zg3/`
- [ ] Mesures de taille finales ci-dessous

## Prochaine étape
Lancer `cent-ans geo detail-dem` (toutes les zones, ~1 h), vérifier les aperçus, commiter le
manifeste (lignes 5-7 seulement) et les aperçus.

## Reprise
Idempotent : relancer la même commande ; les blocs téléchargés et les zones à jour
(`tools/geo/raw/detail/<zone>/done_E<k>.json`) sont sautés.

## Tailles
Estimation : ~2 100 tuiles, < 1 Go de tuiles ; bruts ~2-3 Go.

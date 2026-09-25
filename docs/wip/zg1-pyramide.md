# ZG1 — pyramide de relief, paliers 1-2 (E1-E4)

ADR 0036 (section « Mise en œuvre des paliers 1-2 »). Commande :
`uv run --project tools cent-ans geo pyramid [--levels 1,2,3,4] [--force] [--workers N] [--limit N]`.
Branche : `worktree-agent-a0df650280bb11446`. Doc : `docs/geo.md` (dernière section).

## État
- **E1-E2 PRÊTES (pour ZG2)** : 313 tuiles E1 (91 Mo) + 1 069 tuiles E2 (≈ 310 Mo), 405 Mo,
  51 s sur 14 cœurs. Cache partagé : `/Users/jean_hubert/dev/game_project/data/map/pyramid/E1`,
  `E2`. Manifeste à jour.
- **E3-E4** : cuisson complète en cours (`--levels 3,4 --force`, 499 blocs E2, 1 779 tuiles E3 +
  6 672 tuiles E4 attendues). Reprendre par `geo pyramid --levels 3,4` (sans `--force` : les
  tuiles présentes sont sautées), puis committer `data/map/relief_pyramid.json`.
- Code : `geo/pyramid.py` (géométrie, RLE, `update_manifest_levels`, palier 1),
  `geo/surface.py` (correction GLO-30, retenues, palier 2), `geo/glo30.py` (sources, mosaïque).
- Données : `data/map/modern_reservoirs.json` (57 retenues, schéma
  `data/schemas/modern_reservoirs.schema.json`). Tests : `tools/tests/test_pyramid.py` (20).
- Aperçus : `docs/img/zg1/` (Paris, forêt d'Orléans, Serre-Ponçon avant/après ; E0/E1/E2 Aletsch).

## Mesures
- Bruts : GLO-30 5,18 Go (172 tuiles) + WorldCover 1,54 Go (23 tuiles) + GLO-90 0,96 Go (déjà
  là) ; travail `tools/geo/raw/pyramid_work/` ≈ 0,8 Go. Téléchargement ≈ 6,5 min.
- E0 vs moyenne 2 × 2 d'E1 (terre) : moyenne +0,03 m, médiane |d| 0,48 m, p95 7,6 m, max
  295 m (coutures 1° d'E0, pas d'E1). E1 vs E2 : p99 0,06 m (quantification).
- Canopée mesurée aux lisières (terrain plat) : Orléans 11,4 m, Sologne 10,0, Weald 12,1,
  Ardenne 11,7, Landes 0,8 (coupes rases) → 10 m retenus.
- Retenues détectées (surface trouvée / publiée) : Serre-Ponçon 28,1/28,2 km², Sainte-Croix
  21,2/22, Der 38,9/48, Kielder 10,7/10,9, Rutland 11,5/12,6, Möhne 9,5/10,4 ; Edersee 4,3/11,8
  (GLO-30 non téléchargé à l'est de 9° E). Rapport : `tools/geo/raw/pyramid_work/reservoirs/report.json`.
- Tuile E4 moyenne ≈ 0,2-0,36 Mo (montagne), PNG niveau 9 (le niveau 6 est 50 × plus rapide
  mais 4 % plus gros).

## Limites / points ouverts
- E0 a des coutures le long des méridiens/parallèles entiers (lecture GLO-90 tuile par tuile) :
  régénérer `geo relief-shade` avec `glo30.mosaic_to_grid` (hors lot, touche E0).
- Canopée : décalage uniforme ; lisières de jeunes plantations encore visibles ; Landes sur-
  corrigées (WorldCover 2021 vs radar 2011-2015). Bâti : les très grands bâtiments (> 340 m)
  restent partiellement.
- E1-E2 ne sont pas corrigés (GLO-90 est aussi un modèle de surface, comme E0) : en forêt, E3
  est ≈ 10 m sous E2 ; le moteur (ZG2) doit fondre les étages (morphing).

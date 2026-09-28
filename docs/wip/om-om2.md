# OM2 — chaîne géo sur l'emprise Oural–Méditerranée

Branche `feat/om-om2`, worktree `../gp-om-om2`. ADR 0115 : EPSG:3035, 718,9765625 m/unité,
7168 × 6144, x 2 169 486 → 7 323 110, y 775 684 → 5 193 076 ; pixels existants : x identique,
y + 1280.

## Environnement du worktree
- `tools/geo/raw/*` : liens symboliques vers les bruts du dépôt principal, sauf `kk10/`,
  `copernicus_cache/`, `pyramid_work/` (dossiers propres : caches dépendant de la grille, à ne pas
  écraser pour `main`).
- Pas de lien `data/map/pyramid` (rien n'est recuit).

## État
- [ ] project.py : grille rectangulaire, bornes explicites
- [ ] téléchargements ETOPO manquants, KK10 nouvelle bbox
- [ ] générateurs (build, provinces, relief, relief-shade, splat/landcover, navgrid, routes, colonies, hameaux, horizon, towns, fine_anchors…)
- [ ] provinces : règle de distance aux graines
- [ ] relief_pyramid.json root_origin_tiles [0, 5]
- [ ] migration +1280 y des données en pixels
- [ ] fichiers ≤ 50 Mo
- [ ] tests pytest, docs/geo.md, m1-campaign-map.md

## Prochaine étape
project.py.

## Points ouverts / coordination OM1

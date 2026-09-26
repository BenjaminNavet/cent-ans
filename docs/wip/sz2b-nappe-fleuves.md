# SZ2b — nappe d'eau des fleuves au palier site (suite de SZ2)

Chantier SZ (`docs/wip/sz-suites-zoom.md`). Branche `sz2b-water-sheet` (worktree d'agent
`agent-a098d00691a342944`, depuis `main` 89bc960a). Liens symboliques non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal ; dylib copiée du dépôt principal.
Ne touche ni aux villes (VH5-VH7), ni à l'exagération (SZ1).

## Défaut
Au palier site, la Seine à Rouen s'affiche en lit sableux gris sans nappe d'eau, grève verte entre
mur de rive et lit (`docs/img/vh4/rouen_site.jpg`, `rouen_pont.jpg`, `rouen_seine_sud.jpg`).

## Plan
1. Diagnostic : niveau d'eau `hydro_fine` vs lit creusé vs relief, ruban construit / masqué,
   Rouen puis Loire (Orléans, Tours), Tamise (Londres), Garonne (Bordeaux), 3 paliers.
2. Correctif à la source (outils et/ou rendu).
3. Captures avant/après `docs/img/sz2b/`, tests zg5b_fine_geo, zg7a, smoke, pytest.

## Diagnostic (en cours)
- Cause A : l'eau fine (rubans + lit creusé) est coupée sur les **emprises des maquettes de
  colonies** (rayon de maquette × 0,8 : Orléans 5,2 km, Tours 2,9 km) et dans les **zones
  personnalisées** (Rouen, Paris, Londres…) : règle ZG5b d'avant ZG6/VH4. Au palier près, les
  maquettes sont masquées (villes 1:1) : plus d'eau du tout autour des villes (Orléans, Tours,
  Rouen). Captures `avant_*`.
- Cause B (à confirmer en jeu) : décalage d'un demi-pixel carte (0,5 unité = 360 m) entre les
  rasters affichés par Godot (pixel i centré en x = i : heightmap, pyramide `GRID_OFFSET`,
  `uv = (p + 0,5) / taille`) et toutes les données vectorielles des outils (x = (E − minx) / m,
  pixel i centré en i + 0,5 : fleuves fins, colonies, villes v2). Mesure : les fleuves fins
  collent au relief E4/E7 avec un décalage (−0,5 ; −0,5) exactement à Orléans, Rouen, Paris,
  Londres, Tours (`scratchpad/shift.py`). Conséquence : eau posée à 360 m de son lit, tranchée
  de 7 m à Orléans (fleuve fin sur la berge à 94 m, eau à 86,8 m), lit réel sec à côté.

## État
- [x] 1a. cause A corrigée : rubans marqués dans les emprises / zones de villes 1:1 et effacés
  par le shader tant que les maquettes sont affichées (`cover_open`, `zone_open`), lit creusé
  partout sauf zones des maquettes L1/L2 sans ville 1:1 ; ponts-portes masqués quand les
  maquettes le sont
- [ ] 1b. cause B
- [ ] 2. correctif
- [ ] 3. captures, tests, docs

## Prochaine étape
Diagnostic.

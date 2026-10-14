# CC C4a — couches dérivées indépendantes des provinces

Branche `cc/c4a` (worktree `game_project-cc4a`), issue de `cc-carte-composee`. C4b (provinces, lieux, navgrid,
ports, routes) viendra après C3 et ne doit pas être anticipé ici.

## Plan
1. Carpates murées : `ridges.json` (chaîne d'auteur ≈ 2 500 m) + 3-4 cols dans `crossings.json`, `compose-relief`, test massif.
2. Outils communs : ellipses en km réels -> polygones lon/lat (échelle locale du warp), Köppen/déserts rééchantillonnés par lon/lat,
   filtre majoritaire, mode « indépendant » (sans provinces ni lieux) de splat/landcover/colormap.
3. Régénération : coast_dist, biomes (+ blend), agri_regions, landcover (wetlands, forest_kind, splat provisoire),
   lacs historiques, lakes, relief_occlusion, colormap provisoire (sans lieux/routes), textures GPU, freshwater.
4. Masques d'auteur `compose/masks/*.png` appliqués en surcharge (forêt, roche, champs, marais).
5. `horizon.py` / relief de bataille : relief réel.
6. Tests adaptés ; planche hors dépôt `~/dev/cent-ans-raw/compose/c4a/`.

## État
- Carpates faites (7097374c5) : chaîne à 2 500 m, cols Dukla/Verecke/Prédéal/Turnu Roșu dans `crossings.json` ; test massif :
  23 échantillons non murés sur 452 le long de l'arc, tous aux 4 cols.
- Lacs : `compose/lakes_fine.py` (appelé par `geo compose`, clé `lakes_fine` de `generalise.json`) remet dans `land_mask.png`
  les lacs Natural Earth >= 250 km² réels et les lacs historiques `punch_mask` ; `compose-relief` aplatit chaque plan d'eau
  intérieur (`flatten_lakes`, niveau juste sous la rive) pour que `geo lakes` les voie comme des lacs.
- Outils : ellipses en km réels tracées par polygone à travers la déformation (`landcover.area_signed_distance`), déserts,
  Köppen par lon/lat + filtre majoritaire (`biomes`), `compose/masks.py`, `compose/layers.py`, `geo compose-layers`.

## Reconstruction
```
uv run --project tools cent-ans geo compose          # ≈ 30 s, côtes, lacs fins, fleuves, crêtes
uv run --project tools cent-ans geo compose-relief   # ≈ 3 min
uv run --project tools cent-ans geo compose-layers   # couches dérivées (voir `compose/layers.py`)
```

## Prochaine étape
- Lancer `compose-layers`, corriger ce qui casse, tests, planche.

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
- Squelette posé (ce fichier).

## Prochaine étape
- Carpates.

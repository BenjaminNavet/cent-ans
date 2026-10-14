# CC — carte composée façon Total War

Spécification : `docs/design/2026-10-14-carte-composee.md` ; ADR 0355.
Choix du joueur (14/10) : réinterprétation (C), Europe + Méditerranée, ≈ 400 lieux, géographie
reconnaissable mais caricaturée ; batailles au relief réel ; 0 $.

## État
- 14/10 : inventaires faits (chaîne de fabrication, références aux identifiants), spec + ADR.
- C0a fait (`docs/wip/cc-c0a-selection.md`) : périmètre `realm`, 208 provinces, 400 lieux.
- C0b fait (`docs/wip/cc-c0b-cartogramme.md`) : cartogramme par diffusion + planches.
  Itération v2 : sigma 200 km, densité de la mer 1,0 (au lieu de 150 km / 0,6) : la Méditerranée,
  l'Adriatique et l'Égée ne sont plus écrasées, l'Italie redevient une botte. Strength 1,0 écarté
  (Italie et Ibérie méconnaissables). Planches : `~/dev/cent-ans-raw/compose/v2/`.

- **14/10 : C0 validé par le joueur : strength 0,5** (sigma 200 km, mer 1,0), périmètre `realm`.

## Contrat commun (fixé le 14/10, partagé par C1/C3/C5)
- Canevas composé = emprise recadrée `CROP_LONLAT` (−11..45 E, 28..66 N) de l'ancienne grille,
  pleine résolution (≈ 7080 × 6104 px), `meters_per_px` **nominal** 719 (réglable en C6).
- `data/map/map.json` : `"crs": "composed"` + bloc `composed` (chemin de `warp.json`, taille,
  `meters_per_px` nominal). L'ancienne grille EPSG:3035 reste décrite (`real_grid`) : elle sert à
  lire les rasters réels (relief de bataille, climat, couverture du sol).
- `data/map/compose/warp.json` (versionné) : paramètres (strength 0,5, sigma 200, mer 1,0),
  `real_grid` (origine, m/px de l'ancienne carte), `crop_box_px`, `factor` 8, `size_px`, et deux
  grilles binaires float32 little-endian, forme (ny, nx, 2), ligne 0 = nord :
  - `warp_forward.f32` : pour chaque nœud de la grille réelle réduite (px réels / 8, dans le
    recadrage), la position composée (x, y) en px pleine résolution ;
  - `warp_inverse.f32` : pour chaque nœud de la grille composée réduite (px composés / 8), la
    position réelle (x, y) en px pleine résolution de l'ancienne grille.
  Interpolation bilinéaire ; hors emprise → NaN / `None`.
- Projection : lon/lat → px réels (EPSG:3035, `MapGrid` réel) → `forward` → px composés. Une seule
  implémentation par langage : Python `MapGrid` (geo/project.py), Rust `data-model/src/map.rs`,
  GDScript `MapProjection` (nouveau, `game/scripts/map/map_projection.gd`).
- Chaque province et lieu garde son `lonlat` **réel** (règles : météo, peste, sécheresse, batailles)
  ; ses px composés sont précuits par la chaîne (C4). Les règles ne projettent pas à l'exécution.
- Identifiants : `data/map/compose/id_map.json` (C0a) fait foi.

## Vagues
- V1 (parallèle) : C1 compilateur (`cc/c1`), C3 données (`cc/c3`), C5 exécution (`cc/c5`).
- V2 : C2 relief (après C1). V3 : C4 couches dérivées (après C1-C3). V4 : C6 tests/équilibre.
  Puis C7 avec le joueur.
- **CE — éditeur de carte** (ajouté le 14/10 à la demande du joueur, après C2) : page web locale
  sur la planche 2D ; crêtes, cols, épingles, lieux déplaçables à la souris ; réécrit les JSON
  d'auteur et redessine la planche en quelques secondes ; pinceau de masques (forêt, roche,
  champs, marais) qui alimente les couches peintes ; bouton « recompiler » pour la chaîne.
- **CP — carte peinte** (décision du joueur, 14/10) : en DERNIER, une fois la géographie stabilisée
  par l'éditeur ; peindre de préférence des masques plutôt qu'une image finale figée.

## Prochaine étape
- V1 en cours.

## Points ouverts
- Taille nominale de la nouvelle carte (px, `meters_per_px`) : à fixer en C0 selon la densité visée.

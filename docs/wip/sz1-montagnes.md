# SZ1 — haute montagne au palier vallée (défaut S1 de ZG7c)

Branche `sz1-montagnes` (depuis `main`, worktree d'agent). Liens symboliques non versionnés :
`data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`.

## Constat (captures `docs/img/sz1/avant_*`)
Pyrénées, Alpes, Galles au palier vallée (d = 6, ×3,41) : versants en murs, caméra au fond des
canyons. L'amplitude régionale (2 000 m et plus) × 3,4 fait 10 unités de haut pour une caméra à 6.

## Approche
Écrasement des montagnes dans la fonction unique de hauteur affichée :
`y = s·(h − c(s)·k(x,z)·max(h − base, 0) + g·max(h − fond, 0))`
- `base` : fond non plafonné (min + flou, déjà calculé par `ReliefFloor`) ;
- `k` : facteur d'écrasement par cellule, fonction de l'amplitude régionale A = sommets − base
  (genou `mountain_knee_m`, pente `mountain_ratio`) : 0 pour collines, falaises, plaines ;
- `c(s)` : poids selon l'échelle (0 en vue stratégique → 1 au palier vallée).
Fond, base et k dans une seule texture RGBF (`campaign_relief_floor`) et la grille partagée
(GDScript `MapData`, Rust `vegetation`).

## État
- [x] Captures « avant » (`docs/img/sz1/avant_*`)
- [x] Squelette : réglages dans `ReliefExaggerationProfile` / `relief_exaggeration.tres`
- [ ] ReliefFloor : champs base + k ; MapData : formule, inverse, poids publié
- [ ] Shaders : `campaign_relief.gdshaderinc`, `relief_quadtree.gdshaderinc`
- [ ] Rust vegetation + pont ; job d'arbres
- [ ] Bornes AABB (quadtree, villes)
- [ ] Caméra au-dessus des crêtes voisines
- [ ] Test `sz1_mountain_test.gd` ; zg4/zg8/smoke ; cargo
- [ ] Captures après + itération

## Prochaine étape
Première case non cochée.

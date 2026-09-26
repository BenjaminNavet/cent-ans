# SZ4b — maquettes de colonies continues, forêts denses au palier vallée (suites SZ4)

Branche `feat/sz4b-colonies-forets` (worktree d'agent, depuis `main` 01f09d2b). Liens symboliques non
versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Compilation avec
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`, dylib copiée dans `game/bin`.

## Défauts traités (laissés par SZ4)
1. Maquettes des colonies géantes jusqu'à d ≈ 8, puis remplacées d'un coup par les villes 1:1 (ZG6) ;
   entre d = 28 et 8, moulins et hameaux rétrécis à côté d'une maquette encore géante.
2. Forêts clairsemées au palier vallée : arbres à taille réelle, semis dimensionné pour des arbres
   de ~1 km.

## Conception
### 1. Maquettes
- Chaque maquette rétrécit avec la courbe `MapPropScale` (même `shrink_start` = 28) vers sa **taille
  réelle propre** : rayon bâti de `towns_1340.json` (ZG6) / rayon de la maquette, atteinte à
  `settlement_shrink_end` (8 = seuil du palier vallée, où la ville 1:1 s'active).
- Masquage **par colonie** : la maquette d'une colonie n'est cachée que si sa ville 1:1 est
  construite et affichée (`TownLayer.is_shown`) ; hors du rayon de chargement ZG6, la maquette
  à taille réelle reste (plus de trou ni de saut global).
- Moulins et panaches de cheminée suivent l'échelle de leur colonie (distance au centre × échelle) :
  ils finissent autour de / dans la ville réelle ; hauteurs interpolées entre deux poses gardées.

### 2. Forêts
- Couche « forêt dense » (`ForestDetail`, enfant de `Vegetation`) : cellules de 16 × 16 unités autour
  du point visé, semées par le pool natif (`VegetationScatter`, crate `vegetation`) au pas fin
  `spacing × tree_ratio` (grilles grossières de la tuile réutilisées, rectangle de semis, part des
  graines gardées `keep`, pas de haies).
- Part visible = `ratio² (1/s² − 1)` (s = échelle des arbres) : couvert constant quand les arbres
  rétrécissent ; décroissance avec la distance au point visé, budget d'instances mesuré.

## État
- [x] Squelette, ressources, test dédié `tests/sz4b_colonies_forests_test.gd`, script de captures `tests/sz4b_shots.gd`
- [x] Maquettes : échelle continue vers l'emprise réelle, masquage par colonie (ville 1:1 affichée), étiquettes / picking / anneau
- [x] Moulins et panaches liés à l'échelle de leur colonie (poses de carte et réelle, sols interpolés)
- [x] `MapPropScale` : exagération commune E(d) (125 → 1 entre d = 28 et 8), échelle = min(1, ratio × E)
- [x] Rust : `DetailArea` (rectangle, `keep`, parties) dans `vegetation`, `detail_rect` dans le pont (+ test)
- [x] `ForestDetail` (cellules 16 u, part `full_scale² (1/s² − 1)`, budget, recalage groupé par tuile)
- [x] Défaut trouvé : pied des arbres enfoncé de 0,08 × hauteur **de carte** (80 m) → arbres à taille réelle
  enterrés ; corrigé dans `foliage.gdshaderinc` (enfoncement réduit avec l'arbre)
- [ ] Captures avant (à refaire avec le code de `main` et le script final) / après
- [ ] Mesures (i/s, instances)
- [ ] Tests de non-régression : zg6_towns, cv1_campaign_life, settlements_render, smoke, cargo test
- [ ] Doc `docs/godot-map.md` § SZ4b

## Prochaine étape
Captures après complètes, captures avant refaites (fichiers de `main` restaurés temporairement), mesures
de coût (`--fps-probe` / banc), doc.

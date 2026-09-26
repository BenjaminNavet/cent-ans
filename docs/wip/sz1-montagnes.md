# SZ1 — haute montagne au palier vallée (défaut S1 de ZG7c)

Branche `sz1-montagnes` (depuis `main`, worktree d'agent ; `main` fusionnée après SZ2). Liens
symboliques non versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target` puis copie dans `game/bin/`.
Attention : cible cargo partagée entre worktrees ; si `godot-bridge` ne voit pas les champs SZ1 de
`vegetation`, `touch core/crates/vegetation/src/lib.rs` et recompiler (artefact d'un autre worktree).

## Constat (captures `docs/img/sz1/avant_*`)
Pyrénées, Alpes, Galles au palier vallée (d = 6, ×3,41) : versants en murs, caméra au fond des
canyons. L'amplitude régionale (2 000 m et plus) × 3,4 fait 10 unités de haut pour une caméra à 6.

## Solution
Écrasement des montagnes dans la fonction unique de hauteur affichée :
`y = s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − fond, 0))`, `K = c(s)·k(x, z)`
- `base` : fond non plafonné (min + flou, déjà calculé par `ReliefFloor`) ;
- `k` : facteur par cellule selon l'amplitude régionale A = sommets − base (genou 350 m, pente 0,3,
  borne 0,55) : 0 pour collines, falaises, plaines (A ≤ 290 m mesuré) ;
- `c(s)` : 0 en vue stratégique → 1 dès ×3,5 (palier vallée et site).
Champs dans une texture RGBF (`campaign_relief_floor`) et la grille partagée (GDScript `MapData`,
Rust `vegetation` + `VegetationScatter.set_relief_fields`). Caméra : garde au-dessus des crêtes sur
des cercles de rayon 0,5 × distance autour d'elle et du point visé (`close_camera.tres`).
Réglages : `relief_exaggeration.tres` (`mountain_*`), `close_camera.tres` (`crest_*`). Drapeau
`--no-mountain-squash` pour les captures « avant ».

## État
- [x] Captures « avant » (`docs/img/sz1/avant_*`, `--no-mountain-squash --no-crest`)
- [x] Réglages, ReliefFloor (base, k, amplitude), MapData (formule, inverse, poids publié)
- [x] Shaders `campaign_relief.gdshaderinc` (gradient compris), `relief_quadtree.gdshaderinc`
- [x] Rust vegetation + pont (`set_relief_fields`, `relief_squash`), test Rust
- [x] Bornes AABB (quadtree, villes, E0 cuits)
- [x] Caméra au-dessus des crêtes voisines
- [x] Test `sz1_mountain_test.gd` ; zg2/zg4/zg8/smoke OK ; clippy OK
- [x] Docs : `godot-map.md`, addendum ADR 0036
- [x] Fusion de `main` (SZ2, VH4, SZ6) ; captures avant/après refaites sur le relief SZ2
- [x] Rouen (VH4) : relief près de l'échelle vraie autour des villes 1:1 (`true_scale_*`),
      captures `avant_rouen_*` (= `docs/img/vh4/`) / `apres_rouen_*`
- [x] Tests après fusion : sz1, zg2, zg4, zg8, vh4_landmarks, smoke OK
- [x] cargo fmt/clippy/test complet : 831 OK (cible privée ; la cible partagée mélange les worktrees)

## Prochaine étape
Lot terminé, en attente de fusion par l'orchestrateur (ff-only).

## Limites
- Galles (vallée de Conwy) : amplitude 960 m sur des vallées étroites, reste un paysage de montagne
  encaissé au palier site (caméra à 1,1 km d'un versant de 400 m) ; plus de murs jusqu'au bord haut.
- Alpes au palier site : relief adouci (≈ ×1,3 du vrai) ; `mountain_squash_max` règle le compromis.
- Arbres géants près de la caméra : défaut S4 (lot SZ4, fusionné), hors lot.
- Rouen : fumées blanches au-dessus de la ville et falaise résiduelle au bord de la Seine (bras de
  l'île Lacroix, limite VH4) hors lot.
- Villes 1:1 : le champ est calculé au chargement depuis `LandmarkV2Library` (toutes les villes v2,
  quelle que soit l'année) ; `--no-landmarks-1to1` le coupe.

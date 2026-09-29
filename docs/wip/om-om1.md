# OM1 — monde rectangulaire (ADR 0115)

Branche `feat/om-om1`, worktree `/Users/jean_hubert/dev/gp-om-om1`. Profil cargo `om1`.

## But
Plus aucune taille de monde codée en dur : `data/map/map.json` `size_px` [W, H] (multiples de 256).
Carte actuelle 4096² : comportement identique.

## Choix
- Quadtree de relief : l'échelle des profondeurs ne change pas (nœud de profondeur n = 4096 / 2^n,
  profondeur 4 = tuile E0 de 256). Le monde est pavé par une grille de racines à la profondeur
  `root_depth` = plus petite profondeur dont le côté divise W et H (4096² : 0, une racine, identique ;
  7168×6144 : 2, 7 × 6 racines de 1024).
- Pyramide : grille E0 de W/256 × H/256 tuiles ; `root_origin_tiles` [dx, dy] du manifeste :
  tuile de cache (k, c, r) = tuile monde (k, c + dx·2^k, r + dy·2^k). Les index/clés internes sont en
  coordonnées monde ; seul le chemin de fichier revient en coordonnées de cache.

## État
- [x] Rust relief-lod (grille rectangulaire, racines multiples, `cache_to_world`), tests 28 × 24
- [x] pont relief_lod_bridge (`set_pyramid(max, tiles, cols, rows)`, bornes monde)
- [x] navgrid : LAEA 3035 analytique (`MapMeta::lonlat_to_px`, écart pyproj < 5 cm), grille W/2 × H/2
- [x] vision : masque 4 cellules par texel (512 sur 4096², 896 × 768 sur 7168 × 6144)
- [x] sauvegardes : `STATE_VERSION` 8, `PreWideMapSave` (message français)
- [x] GDScript relief_pyramid / relief_quadtree / terrain_builder (chunks_x × chunks_y) / caméra /
      shaders (défauts neutres) / landmark_v2 / life_ambient / cache status / schéma pyramide
- [x] tests : `om1_wide_world_test.gd`, rs_k / zg4 / po5 lisent la taille
- [x] vérif : cargo test (160 ok), build.sh, import, smoke, om1_wide_world, rs_k, zg4, po5, pb3g, zg2, zg7b, zg7c, sz6, zg8, da7d, pb1 : OK

## Mémoire graphique estimée (textures de base, RGB8 promu RGBA8)
4096² (16,8 Mpx) ≈ 0,53 Go ; 7168 × 6144 (44,0 Mpx) ≈ 1,38 Go. Dont relief_shade (LA8, 2 px/unité,
mipmaps) 179 → 470 Mo ; province_ids, border_dist, wetlands, splat (4 o) 67 → 176 Mo chacun ;
heightmap R16 + mipmaps 45 → 117 Mo ; coast_dist, river_bed (L8) 17 → 44 Mo.

## Prochaine étape
Terminé ; reste l’intégration avec OM2 (vraies données 7168 × 6144).

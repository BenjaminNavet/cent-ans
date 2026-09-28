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
- [ ] Rust relief-lod (grille rectangulaire, racines multiples)
- [ ] pont relief_lod_bridge (bornes monde)
- [ ] navgrid (LAEA 3035 analytique, grille W/2 × H/2)
- [ ] GDScript relief_pyramid / relief_quadtree / terrain_builder / caméra / shaders / divers
- [ ] refus des anciennes sauvegardes
- [ ] tests 7168×6144 (Rust + GDScript)
- [ ] mesure mémoire textures

## Prochaine étape
Rust relief-lod.

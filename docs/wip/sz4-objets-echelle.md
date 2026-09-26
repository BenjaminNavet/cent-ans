# SZ4 — objets à l'échelle aux paliers intermédiaires, disque d'emprise des villes (suites ZG7c S4, S5)

Branche `feat/sz4-objets-echelle` (worktree d'agent, depuis `main` 7e1ac032). Liens symboliques non
versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib copiée de
`/Users/jean_hubert/dev/game_project/game/bin/` (aucun changement Rust prévu). Rendu seulement.

## Diagnostic (captures `docs/img/sz4/avant_*`)
- S4 : tailles de carte (1 unité ≈ 719 m) : moulin `WINDMILL_SCALE` 4,6 → corps de 2,3 km ;
  hameau `HAMLET_SCALE` 3,6 → ~1,4 km ; panaches de cheminée 1,3 × 4 unités (1 × 3 km), incendies
  3 × 13 ; arbres ~1,5 unité, réduits seulement par `(d / 22)^0,8` (0,35 à d = 6, 0,70 à d = 14).
  Moulins, fumées, hameaux : pleine taille jusqu'au palier site, puis masqués d'un coup.
- S5 : à d = 6 (~4 km), Amiens ZG6 est bien construite, mais les maisons (blocs HLOD) sont
  sous-pixel : on ne voit que le sol de terre battue (couche « Rubble ») au bord net, d'où le disque
  brun. Ce n'est pas un défaut de chargement (la racine n'apparaît qu'une fois tout construit).

## Plan
1. `MapPropScale` (`game/scripts/map/map_prop_scale.gd`, `resources/map_prop_scale.tres`) :
   échelle 1 au-delà de `shrink_start` (28), taille réelle en deçà de `shrink_end` (5), fondu
   `smoothstep` en log de distance ; rapports taille réelle / taille carte par famille.
2. Arbres : `campaign_prop_scale` = `tree_scale(d)` (remplace `ZoomTiers.prop_scale`).
3. Fumées : échelle par matériau dans `life_smoke.gdshader` (hauteur de levée encodée dans la
   colonne z de la base d'instance, origine au sol) ; plus masquées au palier site.
4. Moulins, hameaux : réécriture des instances quand l'échelle varie de plus de `rewrite_step`.
5. S5 : sol des villes ZG6 vu de loin : îlots bâtis teintés « masse de toits » selon la distance
   caméra (imposteur des maisons sous-pixel), bord fondu vers le vert des jardins.
6. Captures après, tests (sz4, zg6_towns, smoke, zg4_camera), doc `godot-map.md`.

## État
- [x] Squelette (ressource, test, script de captures, captures avant)
- [ ] 2-5
- [ ] 6

## Prochaine étape
Brancher `MapPropScale` sur les arbres, fumées, moulins et hameaux.

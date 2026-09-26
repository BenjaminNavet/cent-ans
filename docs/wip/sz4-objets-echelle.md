# SZ4 — objets à l'échelle aux paliers intermédiaires, disque d'emprise des villes (suites ZG7c S4, S5)

Branche `feat/sz4-objets-echelle` (worktree d'agent, depuis `main` 7e1ac032). Liens symboliques non
versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib copiée de
`/Users/jean_hubert/dev/game_project/game/bin/` (aucun changement Rust). Rendu seulement.
Doc : `docs/godot-map.md` § « Objets à l'échelle aux paliers intermédiaires (lot SZ4) ».

## Diagnostic (captures `docs/img/sz4/avant_*`)
- S4 : tailles de carte (1 unité ≈ 719 m) : moulin `WINDMILL_SCALE` 4,6 → corps de 2,3 km ;
  hameau `HAMLET_SCALE` 3,6 → ~1,4 km ; panaches de cheminée 1,3 × 4 unités (1 × 3 km), incendies
  3 × 13 ; arbres ~1,5 unité, réduits seulement par `(d / 22)^0,8` (0,35 à d = 6, 0,70 à d = 14).
  Moulins, fumées, hameaux : pleine taille jusqu'au palier site, puis masqués d'un coup.
- S5 : à d = 6 (~4 km), Amiens ZG6 est bien construite, mais les maisons (blocs HLOD) sont
  sous-pixel : on ne voit que le sol de terre battue (couche « Rubble ») au bord net, d'où le disque
  brun. Ce n'est pas un défaut de chargement (la racine n'apparaît qu'une fois tout construit), ni le
  seuil d'activation : l'abaisser ferait apparaître le disque plus tôt.

## État : terminé (à fusionner)
- [x] `MapPropScale` (`game/scripts/map/map_prop_scale.gd`, `resources/map_prop_scale.tres`) :
  1 au-delà de 28 unités, taille réelle sous 5 (arbres : 3), `smoothstep` en log de distance.
  `ZoomTiers.prop_scale*` retirés (remplacés).
- [x] Arbres (`campaign_prop_scale`), panaches (`life_smoke.gdshader` : `prop_scale`, levée dans la
  colonne z de la base), moulins et hameaux (réécriture par pas de 4 %, sol mis en cache) ;
  plus de masquage au palier site pour ces objets (taille réelle).
- [x] S5 : sol bâti des villes ZG6 teinté « masse de toits » de loin (`town_building.gdshader`,
  `roofscape_*` dans `town_render.tres`).
- [x] Captures avant/après `docs/img/sz4/` (Crécy, Paris, Val de Loire, Amiens × vallée, comté,
  stratégique ; zooms Amiens d = 6 et 3). Vue stratégique : identique.
- [x] Tests : `sz4_prop_scale_test` (courbe, carte headless près de Crécy, coûts), zg6_towns,
  zg4_camera, cv1_campaign_life, settlements_render, zg7a, zg8_relief, smoke : OK.

## Limites / suites
- Les maquettes des colonies (palier comté, jusqu'à d ≈ 8) restent à la loupe, puis cèdent la place
  aux villes 1:1 (ZG6) : saut de taille inchangé, hors lot. Entre d = 28 et 8, un moulin ou un hameau
  rétrécit donc à côté d'une maquette de village encore géante.
- Forêts au palier vallée : arbres à taille réelle, le semis (dimensionné pour des arbres de 1 km)
  est clairsemé ; la forêt se lit surtout par la teinte du terrain.
- Navires, bateaux et oiseaux (`LifeAmbient`) : non traités (masqués au palier site seulement).
- Amiens à 4 km reste une tache nette (enceinte), mais de toits et non plus de terre battue.
- Moulins et panaches de colonie gardent leurs positions de carte (autour du rayon de la maquette).

## Prochaine étape
Fusion par l'orchestrateur.

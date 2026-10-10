# VG — vie de la carte en style maquette (gens, charrettes, caravanes, moulins)

ADR : 0340. Worktree `../gp-vg`, branche `vg/main`. Coût cloud : 0 $.

## Diagnostic (2026-10-10)
- Figurines FK (gens, marchands, charrettes, caravanes, bêtes) : à l'échelle 1:1 (VT2), posées
  seulement sous `figure_max_distance` = 3 ; le plancher de caméra GC (`camera_floor_distance` 20,
  ADR 0158) empêche d'y descendre → jamais affichées en jeu.
- Moulins : déjà grossis par GC (`windmill_ratio` 0,65, portée 300), 756 posés et visibles, mais
  ≈ 1 unité de haut (moins qu'un arbre généralisé), brun sur brun : illisibles.

## Plan
1. FolkPool : mode maquette (style `town_style = maquette`) → échelle × `folk_scale`, portée
   `folk_range`, vitesse × `folk_speed`, LOD sous `folk_detail_distance`, exclusion des emprises
   des maquettes. Réglages dans `data/art/town_maquettes.json` § props (+ schéma).
2. Moulins plus grands (`windmill_ratio`).
3. Tests fk_folk (cas 1:1 en style real + cas maquette), smoke, captures (≤ 10).

## État
- [ ] squelette  - [ ] FolkPool  - [ ] moulins  - [ ] tests  - [ ] captures  - [ ] fusion

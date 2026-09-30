# VT2 — moulins, fumées et figurants FK à l'échelle 1:1

Suite de VT (ADR 0138, `docs/wip/vt.md`). Demande validée par le joueur : moulins, panaches de
cheminée et figurants FK (gens, bêtes, charrettes ; ADR 0122) à l'échelle réelle à toute distance
de la vue 3D. Incendies et arbres restent grossis (non touchés). Worktree `../game_project-vt2`,
branche `feat/vt2`. Coût cloud : 0 $.

## État
- [x] `MapPropScale` : `windmill_scale()` / `chimney_scale()` constants (1:1), portées
  `windmill_max_distance` (30), `chimney_max_distance` (25, fondu `visibility_fade` 20 %),
  `windmill_ratio` 0,008 → 0,0093 (mesuré : faîte 11 m, ailes 18 m), `chimney_ratio` 0,02 → 0,0098
  (panache ~9 × 28 m).
- [x] `LifeEffects` : panaches à taille réelle dans les instances (matériau `prop_scale` 1, levée
  réelle : faîte de la ville, 0,35 × `hamlet_scale` pour un hameau) ; moulins à échelle constante ;
  réécritures d'échelle des moulins et des panaches retirées (mortes) ; moulins masqués au-delà de
  leur portée.
- [x] `FolkPool` : `world_scale()` = 1 / `meters_per_px` ; plancher de lisibilité et hauteur de
  carte retirés ; `figure_max_distance` (3,0, `map_scenes.json` + schéma + miroir `MapSceneRules`
  du cœur) ; LOD 1 sous d = 1, 2 au-delà.
- [x] Tests adaptés : `sz4_prop_scale`, `fk_folk`.
- [ ] Passer les tests Godot (sz4, fk_folk, fk5, cv1, fc1, smoke).
- [ ] Densités FK à la portée réduite (rayon d'activité 6 u) : à juger.
- [ ] Captures `vt2_shots.gd` (≤ 3), docs `godot-map.md`, addendum ADR 0138.

## Prochaine étape
Lancer les tests Godot, corriger, puis captures et docs.

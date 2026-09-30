# VT2 — moulins, fumées et figurants FK à l'échelle 1:1

Suite de VT (ADR 0138, `docs/wip/vt.md`). Demande validée par le joueur : moulins, panaches de
cheminée et figurants FK (gens, bêtes, charrettes ; ADR 0122) à l'échelle réelle à toute distance
de la vue 3D. Incendies et arbres restent grossis (non touchés). Worktree `../game_project-vt2`,
branche `feat/vt2`. Coût cloud : 0 $.

## État — TERMINÉ (30/09), à fusionner par l'orchestrateur
- [x] `MapPropScale` : `windmill_scale()` / `chimney_scale()` constants (1:1), portées
  `windmill_max_distance` (30), `chimney_max_distance` (25, fondu `visibility_fade` 20 %),
  `windmill_ratio` 0,008 → 0,0093 (mesuré : faîte 11 m, ailes 18 m), `chimney_ratio` 0,02 → 0,0098
  (panache ~9 × 28 m).
- [x] `LifeEffects` : panaches à taille réelle dans les instances (matériau `prop_scale` 1, levée
  réelle : faîte de la ville, 0,35 × `hamlet_scale` pour un hameau) ; moulins à échelle constante ;
  réécritures d'échelle des moulins et des panaches retirées (mortes) ; moulins masqués au-delà de
  leur portée ; rayon bâti des villes v2 (Paris…) pris dans `extent_m` (moulins hors des rues).
- [x] `FolkPool` : `world_scale()` = 1 / `meters_per_px` ; plancher de lisibilité et hauteur de
  carte retirés ; `figure_max_distance` (3,0, `map_scenes.json` + schéma + miroir `MapSceneRules`
  du cœur) ; LOD 1 sous d = 1, 2 au-delà.
- [x] Tests : `sz4_prop_scale`, `fk_folk` adaptés ; `fc1_shadows`, `cv1_campaign_life`,
  `fk5_incidents`, `smoke` OK ; `cargo test` OK.
- [x] Captures `game/tests/vt2_shots.gd` (locales, `docs/img/vt2/`), docs `godot-map.md`,
  addendum ADR 0138.

## Points ouverts
- Densités FK : rayon d'activité 6 u sous d = 3 → ~20-30 figurants posés (fk_folk : 23), quelques
  uns à l'écran, ≈ 1 px en 1080p. La campagne paraît vide ; rééquilibrer (densités par unité) si
  la partie pilote le demande.
- Arbres grossis : à d = 15 un houppier vaut un quartier de Paris ; seule incohérence d'échelle
  restante (hors demande).
- Worktree sans `data/map/pyramid/` (ignoré par git) : lien symbolique vers celui du checkout
  principal pour les captures (sinon plancher de caméra à 7).
- `vt2_shots.gd` : la position à l'écran calculée par `unproject_position` ne correspond pas au
  rendu (moulin retrouvé par différence d'images) ; non élucidé, sans effet sur le jeu.
- Code mort SZ4b laissé (échelle de maquette `_scale_pair`, poses interpolées) dans `LifeEffects`.

## Prochaine étape
Fusion dans main par la session principale ; jugement du joueur en partie réelle.

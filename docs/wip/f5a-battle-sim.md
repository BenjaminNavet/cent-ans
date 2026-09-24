# F5a — Simulation de bataille, finition

Branche : `worktree-agent-af73653c66d612f3a`

## Plan (dans l'ordre)
1. [x] Collisions entre régiments amis (`src/sim/separation.rs`) : seuls les régiments à l'arrêt
   se repoussent (3 m/s max) ; ceux qui marchent/chargent traversent (passage des lignes).
2. [x] IA : formations + flanc coordonné (`src/formation_ai.rs`, appelé par `ai::plan`) ;
   coin : front de combat = 3 × files (au lieu de 2) ; bug `Unit::slot` du coin corrigé
   (débordement). Sonde ai : 20 rég. 4/10, 10 rég. 6/10.
3. [ ] Déploiement : zones, `deploy_unit`, `start_battle`, IA par rôles, pont Godot
4. [ ] Sièges : A* sur grille, tir des tours, sortie de garnison
5. [ ] Renforts échelonnés (> 20 régiments par camp)
6. [ ] Tests `tests/f5.rs`, équilibre (probe ai : 4-5/10), docs m7/m8 § F5

## État
Squelette posé.

## Prochaine étape
Point 1.

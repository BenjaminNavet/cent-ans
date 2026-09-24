# F5a — Simulation de bataille, finition

Branche : `worktree-agent-af73653c66d612f3a`. Spécification : `docs/design/m7-battles.md` § F5,
`docs/design/m8-sieges.md` § F5.

## État
1. [x] Collisions amies (`src/sim/separation.rs`).
2. [x] Formations IA + flanc coordonné (`src/formation_ai.rs`).
3. [x] Déploiement (`src/sim/deployment.rs`) + pont (`begin_deployment`, `is_deploying`,
   `get_deployment_zone`, `deploy_unit`, `start_battle`).
4. [x] Sièges : maisons, A* (`src/sim/pathing.rs`), tours et sortie (`src/sim/siege_extra.rs`),
   `get_siege().houses/sortie`.
5. [ ] Renforts échelonnés au-delà de 20 régiments : **non fait**.
6. [x] Tests `tests/f5.rs` (8 + 1 ignoré), sonde ai 4/10, docs.

## Prochaine étape
- Point 5 : réserve hors champ au-delà de 20 régiments par camp, entrée par le bord du camp.
- Rééquilibrage siège : l'échelade sans brèche gagne 6/6 (M8 : 3/6).
- HUD (autre agent) : phase de déploiement, rendu des maisons depuis `get_siege().houses`.

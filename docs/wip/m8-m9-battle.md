# WIP — M8 bataille de siège 3D + M9 IA de bataille

Branche : `worktree-agent-ac0b230ee276c88c2`.

## État
- [x] `sim-battle/src/siege.rs` : enceinte (pans, porte, tours), brèches de campagne.
- [x] `sim.rs` : déploiement de siège, blocage par les murs, échelles/tours, bélier, engins contre murs,
      place centrale 60 s, bélier synthétique (pas de pertes rapportées).
- [ ] Tests siège (`tests/siege.rs`).
- [ ] IA de bataille (`ai.rs`) + tests + rééquilibrage (durée 5-15 min).
- [ ] Campagne : `BattleRequest.siege`, assaut interactif, résolution.
- [ ] Pont : géométrie de siège dans `get_terrain()`.
- [ ] Godot : murailles, échelles, tours, smoke, capture.
- [ ] Docs.

## Prochaine étape
Tests de siège, puis IA.

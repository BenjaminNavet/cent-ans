# SC terrain (PF-08, BT7, GB6 partie terrain)

Branche `sc/terrain`, worktree `gp-sc-terrain`.

## État
- Rust : `battle_terrain_field.rs` (hauteurs, rivière, relief, maillages, pur) + `battle_terrain_kernel.rs`
  (classe `BattleTerrainKernel`) ; `stamp_map.rs` étendu (RGBA, `stamp_soft_disc`, `raise_pixel`, `raise_column`, `image`).
- `battle_terrain.gd` branché sur le noyau ; test d'égalité `game/tests/sc_terrain_golden_test.gd`
  (valeurs relevées sur l'ancien GDScript, 4 scènes dont horizon réel et côte).
- Reste : vérifier le golden après build, découper le GDScript (Terrain / Mesh / Decor), mesure avant/après.

## Avant (GDScript, machine chargée) : build terrain 842-1743 ms selon la scène.

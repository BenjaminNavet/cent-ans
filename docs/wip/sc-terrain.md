# SC terrain (PF-08, BT7, GB6 partie terrain)

Branche `sc/terrain` (worktree `gp-sc-terrain`), non fusionnée.

## Fait
- Rust : `battle_terrain_field.rs` (hauteurs, rivière prolongée, relief, maillages ; pur, testé) et
  `battle_terrain_kernel.rs` (`BattleTerrainKernel`) ; `stamp_map.rs` étendu (RGBA, `stamp_soft_disc`,
  `raise_pixel`, `raise_column`, `image`).
- `battle_terrain.gd` (2051 -> ~850 l) + `BattleTerrainSplat` / `BattleTerrainMesh` / `BattleTerrainScatter` ;
  l'état reste sur `BattleTerrain` (`host`).
- `game/tests/sc_terrain_golden_test.gd` : 4 scènes (plaine, côte, horizon réel, horizon + côte) comparées à
  l'ancien GDScript (splatmaps, textures, sol : octet pour octet ; anneaux et arbres : 5e-5 / 1e-2, le C++ de
  Godot fusionne les multiplications-additions).
- Mesure (même machine chargée, build du terrain 1200x800) : 709/664/946/840 ms -> 445/365/591/621 ms.

## Reste / non fait
- Les boucles de semis (`_build_trees`, haies, DA6) restent en GDScript : leurs tirages `RandomNumberGenerator`
  s'entrelacent ; les porter changerait les arbres (rendu non identique). Seul `world_height` y est passé en Rust.

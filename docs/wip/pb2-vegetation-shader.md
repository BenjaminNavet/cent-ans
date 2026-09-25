# PB2 — Semis natif de la végétation + shader du terrain

Demande (2026-09-25) : pistes 1 et 2 de PB1 uniquement : (1) porter le semis des tuiles de
végétation en Rust, (2) alléger le shader du terrain de près sans changement visible.
Worktree `/Users/jean_hubert/dev/gp-pb2`, branche `pb2-veg-shader`. Coût cloud : 0 $.

## État
- [x] Crate pure `core/crates/vegetation` (semis, haies, empaquetage ; opt-level 3 en dev), tests.
- [x] Classe `VegetationScatter` (godot-bridge) : fils natifs, `set_map`, `request`, `poll`.
- [ ] Branchement `Vegetation` / `VegetationTileJob` (grille grossière en GDScript hors fil,
      puis requête native depuis le fil principal), repli GDScript sans l'extension.
- [ ] Mesures (pb1_veg_job, pb1_bench --trace) contre main.
- [ ] Shader du terrain.

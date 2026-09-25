# PB2 — Semis natif de la végétation + shader du terrain

Demande (2026-09-25) : pistes 1 et 2 de PB1 uniquement : (1) porter le semis des tuiles de
végétation en Rust, (2) alléger le shader du terrain de près sans changement visible.
Worktree `/Users/jean_hubert/dev/gp-pb2`, branche `pb2-veg-shader`. Coût cloud : 0 $.

## 1. Semis natif — fait (ADR 0062)
- Crate pure `core/crates/vegetation` (semis, haies, empaquetage, recalage ; opt-level 3 en dev).
- `VegetationScatter` (godot-bridge) : fils natifs, `set_map`, `request`, `request_reground`,
  `poll`. `Vegetation` : grille grossière en GDScript hors fil (`coarse_only`), puis requête
  native ; recalages natifs ; repli GDScript sans l'extension ou avec `--no-native-vegetation`.
- Mesures (M4 Pro, machine partagée, charge 20-85) :
  - `pb1_veg_job.gd` : semis+haies+empaquetage 124-327 ms → 2-10 ms par tuile ; grille
    grossière inchangée (25-35 ms, hors fil). Nombres par essence à ±1-3 % (flux aléatoire
    différent, mêmes lois).
  - Démarrage à chaud (5 tuiles) : 1 008 → 158 ms (Orléans), 920 → 125 ms (Angers).
  - `pb1_bench.gd` A/B dans la même branche : vue initiale complète 11,8 / 14,1 s → 6,5 / 6,4 s
    (≈ chargement 4,2 s + relief fin) ; arrivée au zoom 491 357 / 453 → 88 / 311 ms.
  - Recalage d'une tuile : 438 → 6 ms au pire ; `settlements_render_test` tree_error 0,0.
  - Captures A/B (Orléans d=40, Angers d=25) : même aspect (densité, essences, haies, pose).

## 2. Shader du terrain — à faire
- Le shader est modifié en parallèle par la session ZG (ZG8, zg4b, `integration/zoom`) :
  partir de main à jour, changements localisés, captures comparées pixel à pixel.

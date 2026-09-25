# Lot EP3 — Eau et chemins (chantier « batailles épiques »)

Branche `ep3-eau-chemins` (worktree `.claude/worktrees/agent-a74358ec7ea9828be`). Suivi du chantier :
`docs/wip/epic.md`. ADR : `docs/decisions/0033-eau-ponts-routes.md`.

## Objet
Rivières variées (largeur 10-40 m, affluent, ruisseaux, berges escarpées ou marécageuses, bras
mort), gués nombreux ou rares, ponts de bois ou de pierre (goulots), routes qui comptent dans la
simulation, IA qui tient ponts et gués ; rendu (eau, berges, ponts du kit Blender, gués).

## État
- [x] Squelette : `core/crates/sim-battle/src/hydro.rs` (règles, types, génération, requêtes),
  `data/rules/battle_water.json` + schéma `battle_water_rules.schema.json` + test pytest.
- [x] Génération branchée dans `field.rs` (flux dérivés `HYDRO_STREAM`, `ROADS_STREAM`) + tests.
- [x] Règles de la simulation (`sim/water.rs`) : eau profonde infranchissable (cavaliers, engins),
  noyade, itinéraire par les passages, goulot du pont, tête de pont, route en colonne.
- [x] Pont GDExtension (`get_terrain` : largeurs, ponts, berges, ruisseaux, routes).
- [x] IA (relief_ai.rs + ai.rs) : défense des passages, choix du passage de l'attaquant ;
  tests `the_defender_holds_the_bank_at_a_crossing`, `a_weaker_defender_stays_behind_the_river`,
  `the_attacker_crosses_by_the_bridges_and_fords`.
- [x] Kit Blender : ponts de pierre et de bois.
- [x] Rendu Godot : routes de la simulation, rivière de largeur variable, ruisseaux, ponts, gués,
  écume aux piles.
- [x] Taille du champ : hydro et IA lisent les dimensions du champ (`hydro::battle_lines`,
  `ai::deployment_center(field, side)`), jamais 1200 × 800. À la fusion d'EP1, remplacer
  `battle_lines` par `field.attacker_line_z()/defender_line_z()` et `FIELD_WIDTH` de
  `generate_base` par `width`.
- [ ] Fusion de main, build, pytest, smoke, captures `docs/img/ep3/`.
- [ ] ADR 0033.

Note : le disque était plein (build cassé par des rlib tronquées) ; `cargo clean` fait dans ce
worktree. Construire avec `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0` pour limiter la place.

## Sonde IA contre IA
`cd core && cargo test --release -p sim-battle --test ep3_probe -- --ignored --nocapture`
(`EP3_SEEDS=0..32`). Avant EP3 : en cours (worktree détaché dans le scratchpad).

## Prochaine étape
Faire passer les tests IA, commit, puis `git merge main`, build, smoke, ADR 0033.

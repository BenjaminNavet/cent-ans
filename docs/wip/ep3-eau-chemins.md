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
- [x] Fusion de main (1f38c4ef, sans conflit), `core/build.sh`, fmt/clippy/`cargo test` verts,
  pytest 435 OK, import Godot OK. Captures `docs/img/ep3/ep3_river_deploy.png` (déploiement :
  rivière, pont, routes, ruisseau, minicarte) et `ep3_river_crossing.png` (2 min 30 : franchissement
  au pont de pierre, tête de pont).
- [x] ADR 0033 (`docs/decisions/0033-eau-ponts-routes.md`).
- Smoke : plante (code 138, « Message queue out of memory ») dans l'étape campagne, avant la
  bataille — régression connue de main (signalée dans `docs/wip/nuit.md`), sans rapport avec EP3.

Note : le disque était plein (build cassé par des rlib tronquées) ; `cargo clean` fait dans ce
worktree. Construire avec `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0` pour limiter la place.

## Sonde IA contre IA
`cd core && cargo test --release -p sim-battle --test ep3_probe -- --ignored --nocapture`
(`EP3_SEEDS=0..32`). Avant EP3 : en cours (worktree détaché dans le scratchpad).

## Prochaine étape
Lot terminé, prêt à fusionner dans `integration/epic`. Points ouverts : à la fusion d'EP1,
remplacer `hydro::battle_lines` et le `FIELD_WIDTH` passé à `shape_river` (voir plus haut) ;
en déploiement, une rivière proche de la ligne peut faire sortir des chevaliers de la zone
(message « doivent être placés dans votre zone ») — à vérifier avec EP1 (zones paramétriques) ;
équilibrage par la sonde `ep3_probe`.

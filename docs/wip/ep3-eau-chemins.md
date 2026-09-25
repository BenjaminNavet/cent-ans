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
- [ ] Génération branchée dans `field.rs` (flux dérivés `HYDRO_STREAM`, `ROADS_STREAM`) + tests.
- [ ] Règles de la simulation : eau profonde infranchissable (cavaliers, engins), noyade,
  itinéraire par les passages, goulot du pont, tête de pont, route en colonne.
- [ ] Pont GDExtension (`get_terrain` : largeurs, ponts, berges, ruisseaux, routes).
- [ ] IA (relief_ai.rs + ai.rs) : défense des passages, choix du passage de l'attaquant.
- [ ] Kit Blender : ponts de pierre et de bois.
- [ ] Rendu Godot : routes de la simulation, rivière de largeur variable, ruisseaux, ponts, gués,
  écume aux piles. Captures `docs/img/ep3/`.
- [ ] ADR 0033.

## Sonde IA contre IA
`cd core && cargo test --release -p sim-battle --test ep3_probe -- --ignored --nocapture`
(`EP3_SEEDS=0..32`). Avant EP3 : en cours (worktree détaché dans le scratchpad).

## Prochaine étape
Compiler, tests de génération, puis règles de la simulation.

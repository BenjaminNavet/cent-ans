# État de l'application

Dernière mise à jour : 2026-09-23 (fin de session 1).

## Où en est-on
- **M0 Fondations : terminé.** Prochain jalon : M1 Carte de campagne.
- Design validé : `docs/design/2026-09-23-cent-ans-design.md`.

## Ce qui fonctionne
- `core/` : workspace Rust (data-model, sim-campaign, sim-battle, ai, godot-bridge). 10 tests, clippy propre.
- `game/` : projet Godot 4.7 chargeant la GDExtension ; scène principale avec date et bouton « Fin du tour ».
- Smoke test headless : `godot --headless --path game --script res://tests/smoke.gd` → 10 tours joués, « Automne 1339 ».
- `tools/` : projet Python (uv) avec ledger de budget, client images OpenRouter (contrôle du plafond 50 $), pilotage Blender headless, CLI `cent-ans`. 12 tests.
- `data/` : 9 schémas JSON (draft 2020-12) et 137 fichiers de données 1337 validés (15 factions, 39 personnages réels, 13 unités, 25 bâtiments, 22 technologies, 9 ressources, 6 religions, 8 provinces d'exemple).
- MCP : serveurs Godot (`@coding-solo/godot-mcp`) et Blender (`mcp-for-blender`, addon installé dans Blender 5.2) configurés dans `.mcp.json`.

## Limites connues
- Le crate `data-model` ne charge pas encore les schémas réels de `data/` (types stubs) : à faire en M1/M2.
- Les provinces sont un échantillon de 8 ; la carte complète vient du pipeline géo (M1).
- L'addon Blender MCP exige Blender ouvert en mode graphique ; le fallback headless est `tools/cent_ans_tools/blender.py`.

## Commandes
- Build + tests : voir `CLAUDE.md`.
- Budget : `cd tools && uv run cent-ans budget show` (dépensé : 0,00 $ / 50 $).

# État de l'application

Dernière mise à jour : 2026-09-23 (session 1, fin de M1).

## Où en est-on
- **M0 Fondations : terminé. M1 Carte de campagne : terminé.** Prochain jalon : M2 Boucle de campagne.
- Design validé : `docs/design/2026-09-23-cent-ans-design.md`.

## Ce qui fonctionne
- `core/` : workspace Rust (data-model, sim-campaign, sim-battle, ai, godot-bridge). 10 tests, clippy propre.
- `game/` : projet Godot 4.7 chargeant la GDExtension ; scène principale avec date et bouton « Fin du tour ».
- Smoke test headless : `godot --headless --path game --script res://tests/smoke.gd` → 10 tours joués, « Automne 1339 ».
- `tools/` : projet Python (uv) avec ledger de budget, client images OpenRouter (contrôle du plafond 50 $), pilotage Blender headless, CLI `cent-ans`. 12 tests.
- `data/` : 9 schémas JSON (draft 2020-12) et 137 fichiers de données 1337 validés (15 factions, 39 personnages réels, 13 unités, 25 bâtiments, 22 technologies, 9 ressources, 6 religions, 8 provinces d'exemple).
- MCP : serveurs Godot (`@coding-solo/godot-mcp`) et Blender (`mcp-for-blender`, addon installé dans Blender 5.2) configurés dans `.mcp.json`.

- Carte réelle : `data/map/` (heightmap 4096² ETOPO 2022, Natural Earth), 132 provinces de 1337 avec polygones, voisins terrestres et maritimes (`cent-ans geo build`, ~20 s avec cache).
- Godot : scène `campaign_map.tscn` avec terrain par tuiles et LOD, frontières et teintes de faction en shader, mer animée, rivières, côtes, marqueurs de villes, caméra RTS, sélection de province, panneau parchemin. Captures : `docs/img/godot-real-map.png`.
- Rust : `GameDataStore` charge toutes les données typées et les expose à Godot.

## Limites connues
- Le panneau de province lit encore le GeoJSON, pas `GameDataStore` (capitale/terrain affichés « — ») : à brancher en M2.
- Premier chargement de la heightmap 16 bits ≈ 5 s (décodage PNG en GDScript, cache ensuite) : à déplacer côté Rust en M2.
- Factions manquantes (Anjou-Provence, Grenade, Hollande-Hainaut, Brabant, Gueldre, Venise, Florence…) remplacées par la faction la plus proche, voir `docs/design/provinces-1337.md`.
- L'addon Blender MCP exige Blender ouvert en mode graphique ; le fallback headless est `tools/cent_ans_tools/blender.py`.

## Commandes
- Build + tests : voir `CLAUDE.md`.
- Budget : `cd tools && uv run cent-ans budget show` (dépensé : 0,00 $ / 50 $).

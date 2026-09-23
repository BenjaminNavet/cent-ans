# WIP orchestrateur — finalisation (session 4)

Plan : `docs/design/v2-finalisation.md`.

| Lot | État | Branche / agent | Notes |
|---|---|---|---|
| F1 Règles inertes | lancé | worktree agent | |
| F2 Icônes et infobulles | **fusionné** (b1405fe) | | 144 ids → 106 SVG, RichTooltip, CREDITS.md ; reste : accesseurs GameDataStore pour retirer GameCatalog |
| F3 Écrans et flux | **fusionné** | | menu illustré, chargement, pause, réglages, emplacements + auto, rapport de saison, alertes, crédits |
| F4 Guerre vivante | à faire (vague 2) | | après fusion F1 |
| F5 Batailles | à faire (vague 2) | | après fusion F2 (battle_hud) |
| F6 Rendu carte | **transféré** | session parallèle « visual » | refonte visuelle semi-réaliste (shaders, terrain, marqueurs, modèles, battle_meshes/terrain) : ne pas toucher ces fichiers |
| F7 Contenu | factions faites (orchestrateur, 3a49557) ; événements à faire | main | 13 factions, 18 personnages, 6 listes de noms, meubles héraldiques |
| F8 Tutoriel / encyclopédie | à faire (vague 3) | | |
| F9 Recette | à faire (vague 3) | | |

Prochaine étape : attendre les rapports de la vague 1, fusionner dans main, tests, puis vague 2.
Clé OpenRouter bloquée jusqu'au 1er octobre (limite propre 100 $/mois) : pas de portraits cette session.

## Coordination avec les sessions parallèles
- **game-project-76 (« visual »)** : refonte semi-réaliste, possède shaders, terrain, côtes, rivières, mer, marqueurs (villes, armées, constructions, chemin), `battle_meshes.gd`, `battle_terrain.gd`, assets models/textures, `tools/geo`, réglages de rendu de `project.godot`. Fusionne lui-même dans main après rebase.
- **game-project-e3 (« ui-tw », audit UI Total War, `docs/design/2026-09-23-audit-ui-total-war.md`)** : nouveaux fichiers `game/scripts/ui/{army_strip,general_seal,end_turn_cluster,news_letters}.gd` ; ordres de chef en bataille dans `sim-battle` + `data/battle_orders`. **F5 attend sa fusion.** Son lot A (défauts 1-5, 10-15 : economy.rs, movement.rs, map_ui.gd, province_panel.gd, thème) après fusion de F1/F2/F3 → **le prévenir quand c'est fusionné.**

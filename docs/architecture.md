# Architecture

Vue d'ensemble du dépôt (état au 2026-10-09). Principe directeur : **toute règle de jeu vit dans `core/` (Rust)** ;
`game/` (Godot 4, GDScript) ne fait que le rendu, l'interface et les entrées ; les données vivent dans `data/`.
Décision fondatrice : ADR 0001. Index de toutes les décisions : [`docs/decisions/INDEX.md`](decisions/INDEX.md).

## `core/` — workspace Rust (`core/Cargo.toml`)

| Crate | Rôle |
|---|---|
| `data-model` | Types sérialisables (serde, `deny_unknown_fields`) miroirs des schémas de `data/schemas/`, chargement depuis le disque et vérification des références croisées (`GameData::load`). Aucune règle. |
| `sim-campaign` | Simulation de campagne au tour par tour, pure et déterministe (`état + ordres + graine → nouvel état`) : ordres, mouvement, économie, personnages, diplomatie, événements, sièges, sauvegarde JSON. |
| `sim-battle` | Simulation de bataille en temps réel avec pause, déterministe (ticks fixes, commandes, graine) ; reçoit un `BattleSetup` de la campagne et rend un `BattleOutcome`. |
| `ai` | IA de campagne (planificateur stratégique, diplomatie, doctrine, alignements) et IA de bataille. |
| `vegetation` | Dispersion des tuiles de végétation de la carte (support de rendu, aucune règle). |
| `relief-lod` | Sélection des nœuds du quadtree de relief streamé (CDLOD, résidence des pages) ; support de rendu en Rust pur. |
| `godot-bridge` | Point d'entrée GDExtension (gdext) : expose la simulation à Godot. Seules des valeurs simples franchissent la frontière ; Godot ne détient aucun pointeur vers la simulation. Une `#[func]` doit avoir un appelant dans `game/` ou `tools/`. |

Dépendances : `godot-bridge` → `sim-campaign`, `sim-battle`, `ai`, `data-model`, `vegetation`, `relief-lod` ; `sim-campaign` construit le
`BattleSetup` et applique le `BattleOutcome` ; `ai` dépend des simulations. `core/build.sh` compile et copie la bibliothèque
dynamique dans `game/bin/`. Profil de build : ADR 0079.

## `game/` — projet Godot

- `project.godot` : autoloads (`SimFacade` façade unique vers le pont, `Settings`, `AudioDirector`, `IconLibrary`, `CodexStore`, `UiLayout`, `MapPaths`, garde-fou headless).
- `scenes/` : `start_menu.tscn`, `campaign_map.tscn` (carte de campagne), `battle/` (bataille, dialogue d'avant-bataille), `map/` (marqueurs d'armée), `ui/` (panneaux, fenêtres, thème parchemin).
- `scripts/` : `sim/` (façade vers le pont), `map/` (relief, provinces, végétation, vie de la carte), `battle/` (scène de bataille et ses composants : rendu des figurines, bannières, rejeu, audio, captures), `naval/`, `ui/` (HUD, panneaux, infobulles, réglages), `codex/` (encyclopédie), `audio/`, `visual/` (matériaux, éclairage, post-traitement), `util/`, `debug/`, `dev/`.
- `shaders/`, `assets/`, `resources/` : rendu et ressources ; `tests/` : smoke, tests et sondes headless (`tools/run_godot_tests.sh`).

## `data/` et schémas

Toutes les données de jeu (factions, provinces, unités, bâtiments, technologies, événements, codex, art, audio…) sont des JSON/YAML
dans `data/<domaine>/`, validés par `data/schemas/*.schema.json` (draft 2020-12). Aucune donnée codée en dur : un nouveau paramètre
de règle ou de rendu est une donnée avec son schéma, relue par `data-model`.

## `tools/` — outillage Python (`uv`)

CLI `cent-ans` (`tools/cent_ans_tools/`) : construction de la carte (`geo`), génération et ingestion d'assets, icônes, héraldique,
audio, budget cloud ; scripts shell de lancement, d'export (macOS, Windows), de tests Godot et de captures en arrière-plan
(`godot_bg.sh`, `godot_shot.sh`) ; `adr_index.py` génère l'index des ADR. Tests : `uv run --project tools pytest`.

## Flux campagne → bataille

1. `SimFacade` (GDScript) transmet les ordres du joueur à `godot-bridge`, qui les valide dans `sim-campaign` ; la fin de tour fait jouer l'IA (`ai`) puis résout le mouvement, l'économie et les événements.
2. Quand deux armées ennemies se rencontrent : soit auto-résolution par phases dans `sim-campaign` (ADR 0013), soit bataille jouée. Pour la bataille jouée, `sim-campaign` construit un `BattleSetup` (armées, bonus de technologies et de bâtiments, site, saison) ; `game/` ouvre `battle.tscn`, qui avance `sim-battle` par ticks fixes et lui envoie les commandes du joueur (l'IA de bataille vient de `ai`).
3. Le `BattleOutcome` revient à `sim-campaign` (pertes, captifs, moral, prise de colonie) ; la campagne reprend. Les sièges suivent le même chemin avec murs, machines et assaut produits par le cœur.
4. Le rendu (figurines, relief, végétation, effets) n'écrit jamais dans l'état de jeu ; la physique Godot est réservée au rendu (ADR 0007).

## Où trouver quoi

- Spec de référence : `docs/design/2026-09-23-cent-ans-design.md` ; les designs de jalons `m1`-`m10` sont historiques.
- État synthétique : `docs/status.md` (historique : `docs/archive/status-historique.md`). Chantiers en cours : `docs/wip/`.
- ADR clés : 0001 (cœur Rust + GDExtension), 0003 (IA), 0007 (physique = rendu), 0013 (auto-résolution), 0079 (profil de build), 0153 et 0159 (lanceur Windows, mises à jour automatiques), 0239 (manifeste skinné cuit).

# Architecture

Vue d'ensemble du dépôt (état au 2026-10-17). Principe directeur : **toute règle de jeu vit dans `core/` (Rust)** ;
`client/` (Bevy 0.20, ADR 0357) ne fait que le rendu, l'interface et les entrées ; les données vivent dans `data/`.
Le client Godot (`game/scripts`, scènes, shaders, pont GDExtension) a été retiré le 17/10/2026 (ADR 0366).
Décision fondatrice : ADR 0001 (cœur Rust). Index de toutes les décisions : [`docs/decisions/INDEX.md`](decisions/INDEX.md).

## `core/` — workspace Rust (`core/Cargo.toml`)

| Crate | Rôle |
|---|---|
| `data-model` | Types sérialisables (serde, `deny_unknown_fields`) miroirs des schémas de `data/schemas/`, chargement depuis le disque et vérification des références croisées (`GameData::load`). Aucune règle. |
| `sim-campaign` | Simulation de campagne au tour par tour, pure et déterministe (`état + ordres + graine → nouvel état`) : ordres, mouvement, économie, personnages, diplomatie, événements, sièges, sauvegarde JSON ; `invariants.rs` : invariants d'état vérifiés par `seed-bots`. |
| `sim-battle` | Simulation de bataille en temps réel avec pause, déterministe (ticks fixes, commandes, graine) ; reçoit un `BattleSetup` de la campagne et rend un `BattleOutcome`. |
| `ai` | IA de campagne (planificateur stratégique, diplomatie, doctrine, alignements) et IA de bataille. |
| `game-api` | Façade de lecture/écriture que le client appelle (vues de campagne, tour, diplomatie, tech, recrutement, sauvegardes…). Aucune règle : tout vient de `sim-campaign`. |
| `vegetation` | Dispersion des tuiles de végétation de la carte (support de rendu, aucune règle). |
| `relief-lod` | Sélection des nœuds du quadtree de relief streamé (CDLOD, résidence des pages) ; support de rendu en Rust pur. |
| `seed-bots` | Studio de test autonome (ADR 0348) : 8 bots qui jouent des parties de campagne sur `sim-campaign` sans Godot, 4 familles d'oracles (invariants, paniques/lenteurs, ordres refusés/sauvegarde, bandes d'équilibre), rapports et rejeu. Binaire `seed-bots`. |

Dépendances : `client` → `game-api`, `sim-campaign`, `sim-battle`, `ai`, `data-model`, `vegetation`, `relief-lod` ;
`sim-campaign` construit le `BattleSetup` et applique le `BattleOutcome` ; `ai` dépend des simulations. Profil de build : ADR 0079.

## `client/` — client Bevy (workspace séparé, `client/Cargo.toml`)

Bibliothèque `cent_ans_client` et binaire `cent-ans`. Un plugin par domaine sous `client/src/` : `app` (fenêtre, états), `menu`
(menu principal, parties, sauvegardes, pause), `campaign` (terrain, provinces, eau, météo, armées, villes, panneaux, HUD), `battle`
(scène, figurines instanciées, ordres, sièges, HUD), `flow` (enchaînement campagne ↔ bataille ↔ résultats), `codex`, `audio`, `fx`,
`settings`, `ui` (kit d'interface), `sim` (accès au cœur), `agent` (dumps d'état, scripts, bancs de développement). Lancement
muet et hors écran pour les agents : `client/run_bg.sh` ; tests : `cargo test --features dev`, `client/tests/smoke.sh`. Détails et
sections par domaine : `client/README.md`.

## `game/` — racine d'assets

Plus de projet Godot. `game/assets/` (modèles glb, textures, icônes, héraldique, portraits, audio, enluminures…) est la racine d'assets
du client Bevy ; `game/resources/*.tres` sont des réglages lus tels quels par le client (`client/src/campaign/terrain/tres.rs` ; leur
conversion en JSON sous `data/` reste possible) ; `game/icon.*` sert au lanceur et à l'empaquetage.

## `data/` et schémas

Toutes les données de jeu (factions, provinces, unités, bâtiments, technologies, événements, codex, art, audio…) sont des JSON/YAML
dans `data/<domaine>/`, validés par `data/schemas/*.schema.json` (draft 2020-12). Aucune donnée codée en dur : un nouveau paramètre
de règle ou de rendu est une donnée avec son schéma, relue par `data-model`.

## `tools/` — outillage Python (`uv`)

CLI `cent-ans` (`tools/cent_ans_tools/`) : construction de la carte (`geo`, `compose`), génération et ingestion d'assets, icônes,
héraldique, audio, budget cloud ; `launch.sh` (lanceur, ADR 0117/0362), `compare_views.sh` (vues de référence du client) ;
`adr_index.py` génère l'index des ADR. Tests : `uv run --project tools pytest`.

## Flux campagne → bataille

1. Le client transmet les ordres du joueur à `game-api` / `sim-campaign` ; la fin de tour (sur un fil de travail) fait jouer l'IA (`ai`) puis résout le mouvement, l'économie et les événements.
2. Quand deux armées ennemies se rencontrent : soit auto-résolution par phases dans `sim-campaign` (ADR 0013), soit bataille jouée. Pour la bataille jouée, `sim-campaign` construit un `BattleSetup` (armées, bonus de technologie…) que le client place dans `PendingBattle` avant de passer à l'état `Battle` ; `sim-battle` simule en temps réel avec pause.
3. Le `BattleOutcome` revient à `sim-campaign` (pertes, captifs, moral, prise de colonie) ; la campagne reprend. Les sièges suivent le même chemin avec murs, machines et assaut produits par le cœur.
4. Le rendu (figurines, relief, végétation, effets) n'écrit jamais dans l'état de jeu.

## Où trouver quoi

- Spec de référence : `docs/design/2026-09-23-cent-ans-design.md` ; les designs de jalons `m1`-`m10` sont historiques.
- État synthétique : `docs/status.md` (historique : `docs/archive/status-historique.md`). Chantiers en cours : `docs/wip/`.
- ADR clés : 0001 (cœur Rust), 0003 (IA), 0013 (auto-résolution), 0079 (profil de build), 0153 et 0159 (lanceur Windows, mises à jour automatiques), 0357 (client Bevy), 0362 (lanceurs Bevy), 0366 (retrait de Godot).

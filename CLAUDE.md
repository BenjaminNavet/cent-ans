# Cent Ans — instructions projet

Jeu de grande stratégie (guerre de Cent Ans). Lire `docs/design/2026-09-23-cent-ans-design.md` avant tout travail.

## Règles
- Code, identifiants, commits en anglais ; docs, UI et communication en français.
- Toute règle de jeu vit dans `core/` (Rust). `game/` (Godot, GDScript) ne fait que rendu, UI, entrées.
- Données de jeu dans `data/` (JSON/YAML) validées par `data/schemas/`. Jamais de données codées en dur.
- Rust : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` avant commit.
- Python (`tools/`) : `uv`, `uvx ruff check --fix`, `uvx ruff format`.
- Godot : `godot --headless --path game --import` une fois après un clone, puis `godot --headless --path game --script res://tests/smoke.gd` doit passer.
- Garde-fou headless (`game/scripts/debug/headless_watchdog.gd`) : un processus Godot headless s'arrête (code 124) au-delà de 20 000 erreurs, un script `res://tests/` au-delà de 45 min. Variables `CENT_ANS_MAX_ERRORS` / `CENT_ANS_TEST_TIMEOUT_S` (0 = sans limite) ; ne les lever que pour un outil long connu. Origine : un test bloqué a écrit 206 Go de journaux (03/10).
- **Ne jamais voler le focus du joueur** : tout Godot lancé par un agent avec une fenêtre (bancs `--bench-map`, captures `*_shot.gd`, sondes) passe par `tools/godot_bg.sh <arguments godot>` (macOS `open -g` : fenêtre en arrière-plan, journal sur la sortie standard, pas de code de sortie). Jamais `godot --path game …` fenêtré en direct, jamais d'`osascript … frontmost`. Les tests `--headless` n'ouvrent pas de fenêtre. Seul `tools/launch.sh` (le joueur lance le jeu) prend le focus.
- Toute dépense cloud est consignée dans `docs/budget.md` (plafond 50 $ v1).
- Décisions d'architecture : un fichier ADR dans `docs/decisions/NNNN-titre.md`.
- Pas de ligne Co-Authored-By dans les commits.

## Commandes
- Build GDExtension : `core/build.sh` (build cargo + copie de la dylib/.so/.dll dans `game/bin/` si elle a changé)
- Lancer le jeu : `tools/launch.sh` (recompile et réimporte ce qui a changé, ADR 0117) ou `godot --path game`
- Tests : `cd core && cargo test` ; `uv run --project tools pytest`
- Animations (mocap vidéo, Muybridge, mesures sur vidéos libres, shaders) : `docs/animation.md`

## Règles de robustesse pour les agents (quota Claude Code)
- Commencer chaque tâche par le squelette (API publique, fichiers vides, tests désactivés) et le commiter avant d'implémenter.
- Écrire un fichier `docs/wip/<tâche>.md` (état, prochaine étape) mis à jour à chaque commit `wip:` pour qu'un agent de reprise sache où continuer.
- Pas plus de 10 agents en parallèle par vague ; les agents d'implémentation bien spécifiés tournent sur un modèle plus léger (Sonnet) quand la tâche est mécanique.

## Vérification visuelle et MCP d'éditeur (contexte = quota)
Coûts mesurés (essai godot-ai 2026-09-27, `docs/research/godot-ai-mcp.md`) : capture 640×400 ≈ 340 tokens ; liste d'UI complète d'un écran chargé ≈ 16 000 tokens ; schémas des 47 outils godot-ai ≈ 27 000 tokens (≈ 14 000 avec les seuls domaines utiles). Règles :
- **Texte ciblé d'abord.** Avant toute capture : tests (`cargo test`, `smoke.gd`), logs, propriétés d'un nœud précis. Une capture ne sert qu'à juger un rendu (UI, 3D, lisibilité), jamais à vérifier ce qu'un test ou un log établit.
- **Budget : 5 captures par tâche**, 10 au plus si l'utilisateur demande un travail visuel. Au-delà, s'arrêter et dire pourquoi.
- **Jamais en boucle.** Pas de capture après chaque retouche ni pour « attendre » un état (sonder `editor_state`, pas l'image) : regrouper les corrections, puis une seule capture de contrôle.
- **Résolution par défaut (640 px).** Ne pas monter au-delà sans besoin précis de lire un petit texte ; recadrer plutôt qu'agrandir.
- **Listes d'UI / arbre de scène toujours filtrés** (`root_path` + `max_depth` ≤ 3) : une liste complète coûte plus cher que 40 captures. Ne jamais déverser la carte de campagne ou une bataille entière.
- **Clics : coordonnées exactes** prises dans la liste d'UI filtrée sur le panneau concerné, jamais estimées depuis une capture réduite.
- **Sous-agents :** les agents d'implémentation (Sonnet, tâches mécaniques) n'utilisent pas les outils de capture ni d'éditeur ; la vérification visuelle revient à la session principale ou à un seul agent dédié.
- **Captures de preuve / doc** : scripts `game/tests/*_shot.gd` en headless écrivant un fichier ; ne lire l'image que si un jugement visuel est nécessaire.
- **MCP d'éditeur hors `.mcp.json` partagé** : ne l'activer que dans une session de travail visuel, avec les seuls domaines utiles ; jamais pour changer une règle (les règles vivent dans `core/`).

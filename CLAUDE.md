# Cent Ans — instructions projet

Jeu de grande stratégie (guerre de Cent Ans). Lire `docs/design/2026-09-23-cent-ans-design.md` avant tout travail.

## Règles
- Code, identifiants, commits en anglais ; docs, UI et communication en français.
- Toute règle de jeu vit dans `core/` (Rust). `game/` (Godot, GDScript) ne fait que rendu, UI, entrées.
- Données de jeu dans `data/` (JSON/YAML) validées par `data/schemas/`. Jamais de données codées en dur.
- Rust : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` avant commit.
- Python (`tools/`) : `uv`, `uvx ruff check --fix`, `uvx ruff format`.
- Godot : `godot --headless --path game --import` une fois après un clone, puis `godot --headless --path game --script res://tests/smoke.gd` doit passer.
- Toute dépense cloud est consignée dans `docs/budget.md` (plafond 50 $ v1).
- Décisions d'architecture : un fichier ADR dans `docs/decisions/NNNN-titre.md`.
- Pas de ligne Co-Authored-By dans les commits.

## Commandes
- Build GDExtension : `core/build.sh` (build cargo + copie de la dylib dans `game/bin/`)
- Lancer le jeu : `godot --path game`
- Tests : `cd core && cargo test` ; `uv run --project tools pytest`

## Règles de robustesse pour les agents (quota Claude Code)
- Commiter tôt et souvent : un commit `wip:` dès qu'un état compile ou qu'un fichier est complet, au plus tard toutes les 15 minutes de travail. Une interruption ne doit jamais perdre plus de 15 minutes.
- Commencer chaque tâche par le squelette (API publique, fichiers vides, tests désactivés) et le commiter avant d'implémenter.
- Écrire un fichier `docs/wip/<tâche>.md` (état, prochaine étape) mis à jour à chaque commit `wip:` pour qu'un agent de reprise sache où continuer.
- Pas plus de 3 agents en parallèle par vague ; les agents d'implémentation bien spécifiés tournent sur un modèle plus léger (Sonnet) quand la tâche est mécanique.

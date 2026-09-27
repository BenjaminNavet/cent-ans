# Contribuer à Cent Ans

Merci de votre intérêt ! Toutes les contributions sont bienvenues : signalements de bogues,
corrections historiques, équilibrage, traductions, portages (Linux notamment), code.

## Avant de commencer

- Lisez le document de conception : [`docs/design/2026-09-23-cent-ans-design.md`](docs/design/2026-09-23-cent-ans-design.md).
- Pour un changement important, ouvrez d'abord une issue pour en discuter.
- Les décisions d'architecture passées sont dans [`docs/decisions/`](docs/decisions/) ; une nouvelle
  décision structurante s'accompagne d'un ADR `docs/decisions/NNNN-titre.md`.

## Architecture en une phrase

Toute règle de jeu vit dans `core/` (Rust) ; `game/` (Godot 4.7, GDScript) ne fait que le rendu,
l'interface et les entrées ; les données de jeu sont dans `data/` (JSON/YAML) validées par
`data/schemas/`, jamais codées en dur.

## Mise en place

Voir [Compiler et lancer](README.md#compiler-et-lancer) dans le README.

## Conventions

- **Langue** : code, identifiants et messages de commit en anglais ; documentation et interface en
  français.
- **Rust** : `cargo fmt`, `cargo clippy -- -D warnings` et `cargo test` doivent passer.
- **Python** (`tools/`) : `uv`, `uvx ruff check --fix`, `uvx ruff format`.
- **Godot** : `godot --headless --path game --script res://tests/smoke.gd` doit passer.
- **Données historiques** : toute donnée ajoutée cite ses sources (champ `sources`).
- **Assets** : uniquement sous licence libre compatible (CC0, CC BY, CC BY-SA, OFL, domaine public),
  jamais NC ni ND ; ajoutez l'attribution dans [`CREDITS.md`](CREDITS.md).

## Proposer une modification

1. Forkez le dépôt et créez une branche (`fix/...`, `feat/...`).
2. Faites des commits petits et descriptifs.
3. Ouvrez une pull request en remplissant le modèle ; décrivez comment vous avez testé.

## Licence des contributions

En contribuant, vous acceptez que votre code soit publié sous [GPL v3.0](LICENSE) et vos assets et
données originaux sous [CC BY-SA 4.0](LICENSE-ASSETS.md).

## Note

Le projet est développé avec l'assistance de Claude Code ; `CLAUDE.md` et `docs/wip/` contiennent
les consignes et notes de travail des agents. Les contributions humaines restent les bienvenues !

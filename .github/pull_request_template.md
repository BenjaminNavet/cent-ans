## Résumé

<!-- Que change cette PR et pourquoi ? Lien vers l'issue : « Closes #… » -->

## Tests

- [ ] `cd core && cargo fmt && cargo clippy -- -D warnings && cargo test`
- [ ] `godot --headless --path game --script res://tests/smoke.gd`
- [ ] `uv run --project tools pytest` (si `tools/` modifié)
- [ ] Testé en jeu (décrire brièvement)

## Liste de contrôle

- [ ] Les règles de jeu sont dans `core/`, pas dans `game/`
- [ ] Les données sont dans `data/` et respectent `data/schemas/` ; les faits historiques citent leurs sources
- [ ] Les nouveaux assets sont sous licence libre et crédités dans `CREDITS.md`
- [ ] ADR ajouté si décision d'architecture

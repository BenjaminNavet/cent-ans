# CC C3 — provinces, lieux, factions (données)

Branche `cc/c3`, worktree `game_project-cc3`. Outil : `cent-ans compose rewrite`
(`tools/cent_ans_tools/compose/rewrite.py`), piloté par `data/map/compose/id_map.json` + `selection.json`.

## État
- Outil écrit et appliqué (443 -> 208 provinces, 1656 -> 400 lieux, 177 -> 166 factions).
- Reste : validation Rust (`GameData::load`, title_check), tests, régénération `starting_fit.json`,
  recalage des tests Rust/GDScript, ADR si besoin.

## Prochaine étape
- `cargo test -p data-model`, puis `cargo test` complet ; ignorer en `CC C4: geometry` ce qui dépend de px.

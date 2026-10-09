# RX — consignes communes des agents de lot (corrections)

Chantier RX : `docs/wip/rx-revue-experts.md`, rapports des critiques `docs/wip/rx/<rôle>.md`, lots `docs/wip/rx/lots.md`.

- Créer ton worktree : `git -C /Users/jean_hubert/dev/game_project worktree add ../gp-rx-<lot> -b rx/<lot> main`.
  Travailler UNIQUEMENT dedans. Ne jamais toucher le checkout principal ni un autre worktree. Pas de `git stash`,
  pas de `git add -A` / `commit -a` : `git commit -m … -- <chemins>`. Pas de push, pas de fusion (l'orchestrateur fusionne).
- Squelette d'abord si le lot est gros ; commits `wip:` réguliers ; mets à jour ta ligne dans `docs/wip/rx/lots.md`
  (FAIT / PARTIEL + ce qui reste) dans ton worktree.
- Règles de jeu dans `core/` (Rust), données dans `data/` validées par `data/schemas/` (mets à jour le schéma et
  `data-model` si tu ajoutes un champ). Godot ne fait que rendu/UI.
- ADR : uniquement les numéros du bloc donné dans ton brief, `docs/decisions/NNNN-titre.md` (contexte, décision,
  conséquences), puis `uv run --project tools python tools/adr_index.py` si l'index est généré ainsi.
- Rust : `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, `cargo fmt`, `cargo clippy -p <crate> -- -D warnings`,
  `cargo test -p <crate>` sur les crates touchées. `rm -rf core/target` en fin de lot.
- Godot : headless seulement, tests ciblés (`godot --headless --path game --script res://tests/<x>.gd`).
  Avant le premier test : si le lot ne change pas `core/`, `rm -f game/bin/libcent_ans.*.dylib` puis copier celles du
  checkout principal (jamais `cp` par-dessus un fichier existant : signature macOS cassée), puis
  `godot --headless --path game --import`. Si le lot change `core/`, `core/build.sh` dans le worktree.
  Aucune capture, aucun Godot fenêtré. Aucun process Godot de plus de 15 min.
- Python : `uv run --project tools pytest` ciblé, `uvx ruff check --fix`, `uvx ruff format`.
- Pas de dépense cloud sauf autorisation explicite dans ton brief (alors consigner dans `docs/budget.md`).
- Rapport final (15 lignes max) : branche, commits, fait / reste, tests lancés et résultats, ADR créées.

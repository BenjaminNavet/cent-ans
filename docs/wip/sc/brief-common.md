# SC — consignes communes des agents de lot (vague 15+)

Chantier SC (simplification) : voir `docs/wip/sc-simplification.md`, catalogue `docs/wip/sc/lots.md`,
état par lot `docs/wip/sc/restants.md`.

- Travailler UNIQUEMENT dans le worktree du lot `../gp-sc-<lot>` (branche `sc/<lot>`). Ne jamais toucher
  le checkout principal ni un autre worktree. Pas de `git stash`, pas de `git add -A` / `commit -a` :
  `git commit -m … -- <chemins>`.
- Chaque lot introduit une abstraction (classe de base, trait, table de données, stratégie…) et y fait
  passer tous les sites ; comportement et rendu inchangés sauf ADR.
- Rust : `cargo check -p <crate>` seulement (avec `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`) ;
  la vérification complète (fmt, clippy, test --workspace) est faite à la fin du chantier.
  `rm -rf core/target` en fin de lot (disque).
- Godot : headless seulement (`godot --headless --path game --script res://tests/<test>.gd`), tests
  ciblés sur les fichiers touchés. Aucune capture, aucun lancement fenêtré.
  Lot sans changement dans core/ : la dylib du worktree est souvent périmée → `rm -f game/bin/libcent_ans.*.dylib`
  puis copier celles du checkout principal (jamais `cp` par-dessus : signature macOS cassée, Godot tué en 137
  avec un journal vide), puis `--import`.
- Commits `wip:` réguliers ; mettre à jour la ligne du lot dans `docs/wip/sc/restants.md`
  (FAIT / PARTIEL + ce qui reste). ADR dans le bloc indiqué si une mécanique change.
- Rapport final concis : commits, ce qui est fait, ce qui reste, tests lancés et résultats.

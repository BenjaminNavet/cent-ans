# TW — consignes communes des agents de lot (corrections)

Chantier TW : `docs/wip/tw-total-war.md`, rapports des critiques `docs/wip/tw/<rôle>.md`, lots `docs/wip/tw/lots.md`.

- Créer ton worktree : `git -C /Users/jean_hubert/dev/game_project worktree add ../gp-tw-<lot> -b tw/<lot> main`.
  Travailler UNIQUEMENT dedans. Ne jamais toucher le checkout principal ni un autre worktree. Pas de `git stash`,
  pas de `git add -A` / `commit -a` : `git commit -m … -- <chemins>`. Pas de push, pas de fusion (l'orchestrateur fusionne).
- Squelette d'abord si le lot est gros ; commits `wip:` réguliers ; mets à jour ta ligne dans `docs/wip/tw/lots.md`
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
- Référence : Medieval II et Total War: Warhammer III, transposés au XIVe siècle (aucune magie). Les specs des rapports critiques
  sont un point de départ : vérifie dans le code avant d'écrire, adapte si le rapport s'est trompé (dis-le).
- Toute nouvelle valeur de règle va dans `data/` (jamais en dur) ; tout nouveau champ sérialisé avec `#[serde(default)]`
  (sauvegardes existantes). Textes UI en français, identifiants en anglais.
- Si tu changes un équilibre (économie, entretien, emplacements), lance la sonde `campaign_probe` (ADR 0246, voir
  `docs/decisions/0246-outil-de-mesure-de-campagne.md`) avant/après et note les chiffres dans ton rapport.
- Premier commit sous 5 min (squelette ou note wip), puis commits réguliers. Commandes > 1 min en arrière-plan si possible.
- Sessions parallèles : WR (`../gp-wr-*`, `docs/wip/wr-restes-wh.md` : IA agents/militaire/diplo, captifs, missions, renforts lointains, sortie jouée, loyauté) et UX5 (`../gp-ux5-*` : écrans diplomatie, savoirs, recrutement, colonies, unités). Ne refais pas leurs lots, garde tes diffs ciblés sur ces zones.
- Disque presque plein (≈ 60 Go libres, plusieurs sessions) : `rm -rf core/target` dans ton worktree dès que tes tests Rust sont passés, et en fin de lot ; jamais de gros fichiers générés commités.
- Pas de 3D navale. Pas de génération audio/image payante : si des clips (voix) manquent, liste-les dans ta note de lot.

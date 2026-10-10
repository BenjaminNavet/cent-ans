# UX5 — consignes communes des agents de lot

Chantier : `docs/wip/ux5-ecrans.md` (synthèse des 5 recherches, lots D/T/R/C/U).

- Worktree : `git -C /Users/jean_hubert/dev/game_project worktree add ../gp-ux5-<lot> -b ux5/<lot> main`.
  Travailler UNIQUEMENT dedans. Pas de `git stash`, pas de `git add -A` / `commit -a` : `git commit -m … -- <chemins>`.
  Pas de push, pas de fusion (l'orchestrateur fusionne). D'autres sessions travaillent en parallèle : diffs ciblés.
- Premier commit sous 5 min (squelette), commits `wip:` réguliers ; mets à jour ta section dans
  `docs/wip/ux5-ecrans.md` (FAIT / reste) dans ton worktree.
- Ne modifie PAS `game/scripts/ui/hud_style.gd` ni `ui_type.gd` (partagés par les 5 lots) : utilise leurs jetons
  existants (`INK`, `INK_SOFT`, `INK_FADED`, `RUBRIC`, `WAX`, `WAX_GREEN`, `GOLD`, `GOOD/FAIR/POOR`, `card_box`,
  `panel_box`, `illuminated_box`, `UiType.size(...)`). Une aide locale va dans ton propre fichier.
- Charte : parchemin, encre, cire, rubrique rouge pour le négatif/les alertes ; pas de pastilles saturées, pas de
  pourcentage de « chance », pas de rouge hors alertes/ennemis (ADR 0155). La couleur n'est jamais seule : un glyphe
  ou un mot double l'état. Textes UI en français d'époque sobre, identifiants en anglais.
- Toute règle de jeu dans `core/` ; Godot ne fait que l'affichage. Trier/filtrer/totaliser côté UI est permis,
  calculer une règle (ex. contres d'unités) ne l'est pas.
- Rust (si ton lot y touche) : `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, `cargo fmt`,
  `cargo clippy -p <crate> -- -D warnings`, `cargo test -p <crate>`, puis `core/build.sh` dans le worktree ;
  `rm -rf core/target` en fin de lot. Nouveau champ sérialisé : `#[serde(default)]`.
- Godot : headless seulement. Si le lot ne change pas `core/` : `mkdir -p game/bin`, copier les
  `libcent_ans.*` de `/Users/jean_hubert/dev/game_project/game/bin/` (jamais `cp` par-dessus un fichier existant),
  puis `godot --headless --path game --import`. Un test `game/tests/ux5_<lot>_test.gd` (headless, sur le modèle de
  `game/tests/wh_diploa_test.gd`) qui instancie ton écran, vérifie les nouveaux nœuds/tris/filtres et sort 0.
  Lancer aussi `godot --headless --path game --script res://tests/smoke.gd`. Aucune capture, aucun Godot fenêtré,
  aucun process Godot > 15 min.
- ADR seulement si décision non évidente, numéro imposé dans ton brief.
- Pas de dépense cloud.
- Rapport final (15 lignes max) : branche, commits, fait / reste, tests et résultats.

# WIP — Historien, suite (pistes du README histoire)

Session historien, 24 septembre 2026. Source : `docs/histoire/README.md` (« Pistes pour la suite ») et audit § 7.

| Lot | Contenu | Agent | État |
|---|---|---|---|
| S1 | 14 personnages manquants (audit § 7) + fiches Codex liées | worktree, sonnet | lancé |
| S2 | Prénoms arabes andalous (`names_ar`, cul_andalusi) + ressource étain `res_tin` (Cornouailles, Devon) | worktree, sonnet | **fait** : `data/names/names_ar.json` créé, `cul_andalusi` retiré de `names_es` (test Rust `dynasty::culture_names_tests`) ; `data/resources/res_tin.json`, Cornouailles/Devon passées à `res_tin`, `data/codex/cdx_tin_stannaries.json`. `cargo test`/`clippy` verts ; pytest vert sauf `test_every_data_id_has_an_icon` (attendu, icône `res_tin` du ressort de S3). |
| S3 | Icônes des 7 régimes + icône `res_tin` | worktree, sonnet | lancé |

Hors lot : portraits des nouveaux personnages (clé OpenRouter après le 1er octobre).

Fusion : worktree `../gp-historien-merge`, branche `integration/historien`, puis ff-only dans main.

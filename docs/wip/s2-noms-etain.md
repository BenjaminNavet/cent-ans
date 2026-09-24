# S2 — Prénoms arabes andalous & ressource étain

## État — terminé

- [x] `data/names/names_ar.json` créé (arabe andalou, `cul_andalusi`, 30 prénoms masculins, 20 féminins, épithètes nasrides).
- [x] `cul_andalusi` retiré de `data/names/names_es.json`.
- [x] Test Rust ciblé (`core/crates/sim-campaign/src/dynasty.rs`, module `culture_names_tests`) : Grenade tire ses prénoms de `names_ar`, jamais de `names_es` ; `chr_yusuf_i.house == "Nasrides"` (pas castillan).
- [x] `data/resources/res_tin.json` (modèle `res_iron.json`, prix 22 contre 9 pour le fer, `satisfies_classes` burghers/nobility).
- [x] `prov_cornwall.json` / `prov_devon.json` : `res_iron` -> `res_tin` (et note obsolète retirée à Cornouailles).
- [x] Débouché : `res_tin` satisfait la satisfaction en biens des bourgeois/nobles (vaisselle d'étain), comme les autres matières premières — mécanique déjà générique côté Rust (`population.rs`), aucune règle codée en dur à ajouter.
- [x] `data/codex/cdx_tin_stannaries.json` (186 mots, stannaries + charte de 1305), lié depuis `res_tin.description` par `[[cdx_tin_stannaries]]`.
- [x] `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` (core) : verts.
- [x] `uv run --project tools pytest -q` (tools) : vert sauf `test_icons.py::test_every_data_id_has_an_icon` (attendu : `res_tin` n'a pas encore d'entrée dans `icons_catalog.py`/icône, du ressort d'un autre agent — S3 ; le repli générique par catégorie de `IconLibrary` fonctionne, le jeu ne plante pas).
- [x] `docs/wip/historien-suite.md` : ligne S2 mise à jour.

Aucun fichier `game/` touché : pas de rebuild GDExtension ni de smoke test nécessaires.

## Choix de translittération

Convention "Yusuf" (comme `chr_yusuf_i.json`), sans diacritiques savants (pas de ʿ/ḥ), cohérente avec le Codex existant.

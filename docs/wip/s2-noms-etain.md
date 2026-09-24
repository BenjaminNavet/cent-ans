# S2 — Prénoms arabes andalous & ressource étain

## État

- [x] `data/names/names_ar.json` créé (arabe andalou, `cul_andalusi`, ~30 prénoms masculins, 20 féminins, épithètes nasrides).
- [x] `cul_andalusi` retiré de `data/names/names_es.json`.
- [ ] Test Rust ciblé (dynasty.rs) : Grenade tire ses prénoms de `names_ar`, pas de `names_es` ; le champ `house` d'un personnage grenadin généré reste basé sur la dynastie (`Nasrides`/capitale), jamais castillan.
- [ ] `data/resources/res_tin.json` (modèle `res_iron.json`).
- [ ] `prov_cornwall.json` / `prov_devon.json` : `res_iron` -> `res_tin`.
- [ ] Débouché sobre pour l'étain (satisfies_classes pewter, ou ressource additionnelle d'un bâtiment existant).
- [ ] `data/codex/cdx_tin_stannaries.json` (120-200 mots, stannaries + charte de 1305).
- [ ] Lien codex depuis la description de la ressource ou de la province.
- [ ] `cargo fmt && cargo clippy -- -D warnings && cargo test` (core)
- [ ] `uv run --project tools pytest -q` (tools, valide aussi le Codex)

## Prochaine étape

Ajouter le test Rust ciblé dans `core/crates/sim-campaign/src/dynasty.rs` (module `tests` local, `pick_name` est privé), puis créer `res_tin.json` + éditer les deux provinces + la fiche Codex.

## Choix de translittération

Convention "Yusuf" (comme `chr_yusuf_i.json`), sans diacritiques savants (pas de ʿ/ḥ), cohérente avec le Codex existant.

# Sonde de l'audit A2 (sources hors build)

Sources de la sonde utilisée par `docs/audit/a2-mecaniques.md`, gardées ici en `.txt` pour ne pas entrer dans un build ni dans le lint. Elles servent de point de départ au lot O1, qui les fera entrer dans `core/crates/ai/examples/` et `tools/`.

Reconstruction (hors dépôt) :
- `Cargo.toml.txt` → `Cargo.toml` d'une crate isolée (`[workspace]` vide, dépendances par chemin vers `core/crates/*`), avec une copie de `core/Cargo.lock`.
- `a2probe_main.rs.txt` → `src/main.rs` ; `matrix.rs.txt` et `rt.rs.txt` → `src/bin/`.
- `cargo build --release --offline`, puis `a2probe 200 1 2 3 4 5 6 7 8 > r200.jsonl` et `a2probe 464 11 12 13 14 > r464.jsonl`.
- `agg.py.txt` → `agg.py`. Il lit aussi `buildings.json`, `techs.json`, `events.json`, `event_titles.json` et `event_years.json`, qui listent les ids de `data/`  : les produire avec `extract_ids.py.txt`.

Sorties brutes : `agg200.txt` (8 graines × 200 tours), `agg464.txt` (4 × 464 tours), `matrix.txt` (auto-résolution), `rt.txt` (bataille 3D).

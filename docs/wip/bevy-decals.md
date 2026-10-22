# Lot decals (Bevy 0.20 `pbr_clustered_decals`)

## État
- Feature Cargo `decals = ["bevy/pbr_clustered_decals"]` (bloc « lot decals » de client/Cargo.toml), hors `dev`, hors défaut.
- `client/src/battle/effects/decals.rs` : pool borné (anneau, ≤ 1024 = limite de vue Bevy), sang (`Losses::falls`), boue
  (charge → mêlée : plaque de contact ; `Charging` : traces espacées). Sang aussi posé par `decor/blood.rs` (maillage) : doublon voulu pour l'essai.
- Réglages `data/fx/battle_decals.json` (schéma `fx_battle_decals`), `enabled: false`.
- `decor/ground.wesl` appelle `apply_decals` avant l'éclairage (les décals ne sont pas appliqués par `pbr_input_from_standard_material`).
- Dump section `decals` (count, slots, capacity, laid, recycled, dropped, feature_compiled). Tests unitaires : pool, durée de vie, fondu.

## Vérifications
- `cargo fmt` + `cargo clippy --features dev --all-targets -- -D warnings` : OK (sans la feature `decals`).
- `cargo test --features dev decals` : NON exécuté, disque plein (175 Mo libres) lors de l'édition de liens.

## Prochaine étape
1. Libérer du disque, relancer `cargo test --features dev decals` puis la suite complète.
2. Compiler avec `--features decals` (vérifier que le shader du sol compile avec `apply_decals`).
3. Bench `tools/bevy_bench/bench.sh decals --bench-battle --offscreen` avant/après (feature + `enabled: true`), puis décider du défaut.
4. Dump `decals` : vérifier le compte en bataille avec `enabled: true`.

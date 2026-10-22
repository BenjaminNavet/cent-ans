# Lot decals (Bevy 0.20 `pbr_clustered_decals`)

## État
- Feature Cargo `decals = ["bevy/pbr_clustered_decals"]` (bloc « lot decals » de client/Cargo.toml), hors `dev`, hors défaut.
- `client/src/battle/effects/decals.rs` : pool borné (anneau, ≤ 1024 = limite de vue Bevy), sang (`Losses::falls`), boue
  (charge → mêlée : plaque de contact ; `Charging` : traces espacées). Sang aussi posé par `decor/blood.rs` (maillage) : doublon voulu pour l'essai.
- Réglages `data/fx/battle_decals.json` (schéma `fx_battle_decals`), `enabled: false`.
- `decor/ground.wesl` appelle `apply_decals` avant l'éclairage (les décals ne sont pas appliqués par `pbr_input_from_standard_material`).
- Dump section `decals` (count, slots, capacity, laid, recycled, dropped, feature_compiled). Tests unitaires : pool, durée de vie, fondu.

## Prochaine étape
- Compiler/clippy/test (en attente du banc), bench avant/après, puis décider du défaut.

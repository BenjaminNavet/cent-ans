# TERRAIN2 : patchwork de rectangles du sol de campagne (ADR 0787)

Mandat 24/10 : constats 2, 3 et « Ressenti » de `out/juge/iter-04-art.md`.
Branche `worktree-agent-a5b8a4d63f0744bce` (base main 62aa11193).

## État : FAIT (à fusionner)
- Cause : parcelles (`gd_parcel_k` + patch de culture) trop contrastées, flou de bord < pixel ; pas le cache, ni la LOD, ni les masques (voir ADR 0787).
- Correction : `data/fx/ground_detail.json`, `data/art/ground_biome_mix.json`, `ground_detail.wesl` (warp 3e octave, rampe de bord bruitée).
- Mesures : bench pan d30 p50 10,72 (avant) / 10,77 ms (après) ; `cargo test --release --lib shader_check` vert ; 0 erreur de shader au lancement.
- Captures utilisées : 7 / 10 (montages out/t2/m*.png, non versionnés).
## Reste
Fusion par l'intégrateur ; smoke et clippy complets non lancés (consigne).

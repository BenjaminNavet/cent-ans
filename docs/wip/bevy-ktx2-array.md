# bevy-ktx2-array : sol de bataille en tableaux KTX2

Lot : cuire hors ligne les bandes albedo/normales/grain micro du sol de bataille en tableaux KTX2
(UASTC + zstd, mips inclus), chargés à la place de `strip_to_array` + `mip_chain`. Campagne exclue.

## État
- Outil `ktx` 4.4.2 : `~/dev/ktx-software/bin/ktx` (pkg GitHub dézippé, sans sudo).
- Script : `tools/cent_ans_tools/ktx2_array.py` (cache `out/ktx2_cache/`, gitignoré).
- Feature Bevy `ktx2`, `zstd_c`, `basis-universal` activées (client/Cargo.toml).
- À faire : chargeur dans ground.rs avec repli, mesures, captures.

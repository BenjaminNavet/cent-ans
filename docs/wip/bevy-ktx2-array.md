# bevy-ktx2-array : sol de bataille en tableaux KTX2

Lot : cuire hors ligne les bandes albedo/normales/grain micro du sol de bataille en tableaux KTX2
(UASTC + zstd, mips inclus), chargés à la place de `strip_to_array` + `mip_chain`. Campagne exclue.

## État (23/10)
- Outil `ktx` 4.4.2 : `~/dev/ktx-software/bin/ktx` (variable `KTX`, PATH, ou ce chemin).
- Cuisson : `uv run --project tools cent-ans textures bake-ktx2 [--only b02] [--force]` -> `out/ktx2_cache/`
  (gitignoré, 30 tableaux, ~228 Mio). `tools/cent_ans_tools/ktx2_array.py`.
- Bug corrigé : `ktx create` linéarisait les PNG (sRGB par défaut) ; `--assign-tf linear` ajouté.
  Vérifié : couche 0 décodée = tuile 0 de la bande JPG (diff moyenne 2,2/255, perte UASTC) ;
  mips 1-3 = filtre boîte du mip 0 décodé (diff 0,9-1,9).
- Client : source d'assets `ktx2cache://`, `ground_array` charge le `.ktx2` (repli JPG si absent ou échec,
  `CENT_ANS_NO_KTX2=1` force l'ancien chemin). Dump `battle_ground` : `ktx2_loaded`, `ktx2_fallback`.
- Mesures (debug) : CPU fil principal ~284 ms -> 0 ; GPU des 4 tableaux 127 -> 31,8 Mio (ASTC 4x4).
- Vérifié : clippy défaut et `--features dev`, `cargo test --features dev` ; bataille 300 images exit 0,
  ktx2_loaded 4, fallback 0 ; capture : sol herbeux, soldats, ciel normaux.
- Comparaison old/new par pixels : non pertinente (caméras différentes entre les deux captures).

## Reste
- Campagne 300 images, test du repli (renommer un .ktx2), puis suppression de client/target.

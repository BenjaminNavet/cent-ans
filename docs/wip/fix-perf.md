# Lot perf (correctifs post-playtest 17/10)

Branche `fix/perf`, bac à sable sb-perf (ultra, ui 1.1), release, 1280x800 hors écran, banc
`--bench-map --bench-pan-only --bench-seconds 12 --bench-distance D`. Charge machine 3-7 pendant les
mesures (uptime relevé à chaque ligne dans les journaux du lot) : seuls les deltas alternés comptent.

## Défaut 1 : campagne sous 40 i/s

Bissection `CENT_ANS_PERF_OFF` à d=60 (p50 ms) : base 53, sans ombres solaires 35, sans ombres des arbres
(`vegshadow`) 42, `veg` 46, `terrain` 41, le reste (eau, provinces, villes, vie, armées, repères, mer) sans effet.
Les ombres des arbres : les modèles proches (18 lots, 1 347 instances) coûtent 17 ms, les impostors rien.
Cause : le passe d'ombre dessine chaque modèle lod1 (jusqu'à 6 000 triangles pour les bosquets) dans les
4 cascades du soleil ; test « tout rejeté au vertex » = même coût, donc coût d'invocations de sommets, pas
de remplissage. Les rayons d'ombre par distance et le rejet par cascade n'ont rien changé (retirés).

Correctif : les modèles projettent leur ombre avec le maillage lod2 de l'espèce (`<id>_lod2.glb`, 5 fois moins
de triangles ; repli lod1 si absent), `VegBatch::shadow_mesh`, `DrawShadowBatch`. Interrupteur dev
`CENT_ANS_PERF_OFF=vslod1` (ombres en lod1, état d'avant).

| d | avant (vslod1) p50/p95 | après p50/p95 |
|---|---|---|
| 150 | 40,8 / 46,6 | 40,9 / 47,3 |
| 90 | 42,0 / 49,5 | 42,3 / 50,7 |
| 60 | 62,1 / 82,1 | 49,2 / 56,9 |
| 45 | 54,6 / 68,5 | 45,4 / 52,7 |
| 30 | 46,2 / 57,1 | 41,1 / 47,9 |

(Base de départ d=60 sur main : 63,7 / 85,4.)

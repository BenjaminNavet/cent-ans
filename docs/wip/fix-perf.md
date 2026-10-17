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

## Défaut 2 : temps d'image qui monte et ne redescend pas

Sonde ajoutée : section de dump `perf_probe` (`client/src/agent/perf_probe.rs` : temps d'image glissant sur
60 images, entités vivantes, archétypes les plus peuplés, tailles des collections d'assets).
Script `out/ui.txt` (panneaux diplomatie/tech/chronique/féodal/conseil/faction ouverts puis fermés, deux tours,
puis 8 tours de 2 `Enter`) : image moyenne 9-10 ms au repos, 13-14 ms panneau ouvert, **33 ms à la première
ouverture du Conseil et 28-30 ms ensuite, panneau fermé** (+18 ms durables). Entités et assets sans croissance
sur 18 tours (flux de tours seul : 20-22 ms stable, le saut 9 -> 22 ms au premier tour est la météo : brouillard
de départ puis temps clair à d=637, pas une fuite).
Cause : la salle du Conseil (3 bougies + projecteur à ombres, 9 lumières) restait dans le monde caméra coupée ;
les lumières à ombres sont extraites et rendues quoi que fasse la caméra (`Visibility::Hidden` sans effet,
testé). Correctif : `ensure_room` détruit la salle à la fermeture et la reconstruit à l'ouverture
(`campaign/council/mod.rs`) : repos après fermeture 10,7 ms (28-30 avant), réouverture identique à la première.

| étape | avant | après |
|---|---|---|
| Conseil fermé (moyenne 60 images) | 28-30 ms | 10,7 ms |
| Conseil ouvert | 32-34 ms | 32 ms |

### Fuite des vols d'oiseaux (trouvée par la sonde, `top_names`)

`life/stream.rs::update_targets` remettait chaque image la cible de fondu des vols (`Cat::Bird`) à 1,0 alors que
`birds::manage_flocks` les retire (autre cellule, saison close) : aucun vol n'était jamais détruit, 511 oiseaux au
tour 4, 1 730 au tour 8 (+4,4 k entités en 4 tours, caméra suivant le rejeu). Correctif : `Cat::Bird` ignoré par
`update_targets`. Après : entités 49,1 k -> 49,0 k du tour 4 au tour 8 (avant 50,3 k -> 54,7 k).
Les 8 059 murets / 4 194 cabanes de vigne apparaissent d'un coup entre les tours 3 et 4 puis restent stables
(file de création budgétée, pas une fuite).

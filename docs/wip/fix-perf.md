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

## Densité du relief : le préréglage n'était pas appliqué (ADR 0706)

Bissection à d=150 (p50) : sans terrain 22 contre 41 ms ; MSAA ultra (2x carte) +7,6 ms ; `veg` -11 ms ; ombres -7 ms.
Le client Bevy ignorait `relief_vertex_px` / `relief_items` du préréglage (4 px / 700 nœuds partout, ultra
prévu à 6 px / 900). Corrigé (`terrain/lod.rs::sync_relief_quality`, `settings/quality.rs`). A/B alterné :
d=150 30,5 -> 23,2 ms, d=60 45,8 -> 41,2 ms. Écart d'image 4 px contre 6 px : 1,2/255 en moyenne, invisible.

## Contrôle visuel (3 captures de 1280x800, écarts numériques + un crop)

Ombres lod1 contre lod2 à d=32 : écart moyen 1,6/255, 1,1 % des pixels à plus de 16, ombres des arbres équivalentes
sur le crop le plus différent. Relief 4 px contre 6 px : voir ADR 0706.

## Bilan (release, ultra, 1280x800 hors écran, p50 / p95 ms ; charge 3-7 dont un démon système à 99 % de CPU)

| banc | avant (main 5afcfbd08) | après |
|---|---|---|
| d=150 | 40,7 / 47,6 | 23,5 / 26,8 |
| d=90 | 42,0 / 49,5 | 37,5 / 45,5 |
| d=60 | 62,1-63,7 / 82-85 | 41,8 / 53,1 |
| d=45 | 54,6 / 68,5 | 38,2 / 46,5 |
| d=30 pan | 46,2 / 57,1 | 36,0 / 42,5 |
| carte complète, pan (d=30) | non mesuré | 37,9 / 42,4 |
| carte complète, descentes | 56-65 / 104-120 (rapport) | 35,7 / 47,0 |
| Conseil fermé après visite | 28-30 ms | 10,7 ms |
| bataille Crécy mêlée | 24,0 / 25,8 | 24,0 / 25,8 (inchangé) |
Préréglage « high » (cible de l'ADR 0404) à d=60 : 35,5 / 45.

## Non reproduits

- Pic de 678 ms au démarrage du banc, 546 ms à d=150, 444 ms en mêlée : max observés 70-82 ms (cartes), 38-41 ms
  (mêlée) sur 6 passes ; les pics du profil (`feature profile`) sont des chargements (envoi de textures 68 ms,
  `allocate_and_free_meshes` 28 ms, `reseat_ribbons` 34 ms au démarrage), pas de pic de 400 ms. Probablement dus à
  la charge machine du moment.
- Régression 7e0c4bb12 -> 3dd37ab5c : non rejouée (pas de build de l'ancienne version) ; les trois causes trouvées
  (ombres lod1, densité de relief, salle du Conseil) se lisent sur main seul.
- Surcoût des mips de `model_mips.rs` : `generate_queued_mips` moyenne 0,22 ms/image, max 16 ms (une fois par
  image chargée) ; copies de matériaux de roche non isolées, sans effet visible dans les pics.

## Restants (hors lot)

- Le 40 i/s (p95 <= 25 ms) n'est pas atteint en ultra à d=30-90 (p95 42-53 ms) : le coût est GPU (soumission
  `submit_pending_command_buffers` ~30 ms/image de 38) : fragment du terrain (13 ms à d=60), ombres du soleil
  (11 ms dont 5,6 d'arbres), végétation (7,6 ms), MSAA 2x de la carte en ultra (+7,6 ms à d=150).
  Pistes : MSAA carte, réglage ultra des ombres, shader du terrain (voir ADR 0169).
- Salle du Conseil ouverte : 32 ms contre 10 (la campagne se dessine derrière) ; couper la caméra de campagne
  quand le Conseil couvre l'écran.
- Clés de préréglage non lues par le client Bevy (à vérifier une par une) : `veg_density`, `veg_detail`,
  `terrain_near`, `ground_detail`, `clutter_density`, `grass`, `upscale_*`.

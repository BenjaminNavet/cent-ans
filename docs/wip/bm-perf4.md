# bm-perf4 : perf carte Bevy (d=30)

## Cause principale (corrigée)
`settings::quality` passait `shadow_atlas` (8192, atlas Godot partagé par les cascades) comme côté de la carte d'ombre de
CHAQUE cascade Bevy : 4 x 8192^2 de profondeur (1 Go). Le passage d'ombres des arbres (`vegetation/shadow_pass.rs`,
impostors a alpha-test dessinés dans chaque cascade) était limité par le remplissage : ~16-20 ms par image.
Correctif : `cascade_map_size(atlas, splits)` = atlas/2 pour 3-4 cascades (équivalent Godot : quadrants), atlas pour 1-2.

## Bissection (interrupteurs `CENT_ANS_PERF_OFF=shadows,post,veg,vegshadow,terrain,water,provinces,towns,life,armies,markers,psea`, `campaign/perf_switch.rs`)
Panoramique d=30, release, offscreen, p50 (ms) : base 28 ; sans ombres 7,7 ; sans ombres d'arbres 7,8 ; sans billboards 28,8 ;
sans post, terrain, eau, provinces : 27-28 (inchange). `CENT_ANS_SHADOW_SIZE=n` force le côté : 1024 10,0 / 2048 10,5 / 4096 12,3 / 8192 28,2.
Impostors = ~2/3 du coût d'ombre, modèles proches ~1/3 ; coût ~linéaire en nombre de cascades (le culling VS par cascade n'a rien changé : le coût est le remplissage).

## Chiffres (3 essais, bench complet `--bench-distance 30`, ms)
| | pan p50 | pan p95 | pan p99 | >50 ms | descente p50 | desc. p99 |
|---|---|---|---|---|---|---|
| avant (base, 3 essais) | 28,2 / 28,5 / 27,2* | 47,8 / 48,0 | 52,6 / 52,7 | 18 | 16,5-20 | 52 |
| après | 12,6 / 13,0 / 12,9 | 23,9 / 24,1 / 24,2 | 26,0 / 26,0 / 26,2 | 1-2 | 11,3-11,4 | 33,6-34,2 |
(* base1 = premier lancement a froid.) Godot : p50 19,8, p99 41.

## reseat_ribbons
Passe découpée en tranches (`ReseatJob`, budget 1,5 ms/image) au lieu d'un balayage de tout le ruban en une image (pics 50 ms -> max 4 ms estimes).

## Reste / énigme
- `extract_visibility_ranges` + `write_render_visibility_ranges` ~3,8 ms/image CPU (profil) : `markers/lod.rs::fade_maquettes` réécrit les 1176 `VisibilityRange` a chaque image.
  Les rendre constantes hors zone de fondu supprime ce coût CPU mais fait REPASSER le GPU a 28 ms (reproductible, valeur/diversité des plages sans effet, une seule plage modifiée par image sans effet, une mise a jour toutes les 4 images = bimodal 14 ms p50). Mécanisme non élucidé (suspect : ré-extraction des instances de mesh / ordre de dessin du terrain) ; laissé tel quel.
- Descente : une image a ~170 ms au début (calculate_bounds / check_entities_needing_specialization sur rafale de spawns, 94-130 ms) : a étaler.
- Le fil de rendu dépend encore du GPU (submit 3 ms) ; main thread 8 ms.

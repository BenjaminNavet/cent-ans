# AS8b — allures du cheval depuis des planches libres

Branche `feat/as8b`. ADR 0189. Remplace le keyframé « d'après Muybridge » d'AS3 pour le trot et
le galop (le pas garde l'action Quaternius ; les virages inclinent le trot).

## État
- [x] Sources hors dépôt (`~/dev/cent-ans-mocap-src/video/free/horses/`, un `.LICENSE.txt` chacune).
- [x] `tools/video_mocap/track_quadruped.py` (`fit`, `track`), points posés à la main
  `tools/video_mocap/data/horse_keypoints.json` (galop 12 images, trot 12 images = 1 foulée),
  `export_horse_gaits.py` -> `tools/blender_scripts/data/horse_gaits_free.json` (+ `SOURCE.md`).
- [x] `battle_skinned_gaits.py` : `horse_trot_free`, `gallop_free`, `free_rows` (rangées c_gallop,
  c_charge, c_std_gallop, c_std_charge) ; `AS8B_LEGACY=1` rétablit les clips AS3 (A/B).
- [x] Mesures : `tools/blender_scripts/as8b_measure.py` (appuis, glissement, articulations à l'envers).
- [x] Recuit fin (`battle_fine.py -- rigs`) et grossier (`as8b_rebake_coarse.py`).
- [x] Cadences `battle_gore.json` : trot 3,4, galop et charge 4,5 (vitesse d'appui mesurée).
- [x] Tests : `as8b_test.gd`, `as3_test.gd` (seuil galop relâché), `an1b_clips_test`, smoke.

## Mesures avant / après (appui des sabots dans le clip, m/s)
Glissement propre = écart moyen de la vitesse d'un sabot posé à sa médiane (0 = pas de patinage).
| Clip | avant (AS3) | après (AS8b) |
|---|---|---|
| c_trot | médiane 3,5 ; glissement propre 34 % ; 10 images en l'air ; appui 17 % | médiane 3,4 ; 5 % ; 4 images en l'air ; appui 33 % |
| c_gallop | médiane 3,8 ; 30 % | médiane 3,9 ; 12 % |
| c_walk (inchangé) | 1,1 m/s pour 1,8 de cadence : 49 % de patinage | idem |

## Points ouverts
- Pas non refait : la planche du pas est un cheval de trait attelé, pattes recouvertes.
- Cheval du jeu plus petit que la cadence historique : jambes courtes, foulée plus courte que
  mesurée (gain de foulée calculé, `stride_gain`), cadences recalées ; à juger en jeu (AS7).
- Poses au clic près (±0,1 longueur de jambe) ; suivi Lucas-Kanade validé seulement comme outil
  (erreur 0,26 longueur de jambe sur le galop : trop imprécis, points posés conservés).
- « Horses running in a pasture.webm » (CC BY 3.0) téléchargée, non exploitée.

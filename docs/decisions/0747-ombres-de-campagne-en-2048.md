# 0747 — Carte d'ombre de la campagne à 2048 par cascade

Statut : accepté (23/10)

## Contexte
Le soleil de la campagne utilisait la même carte d'ombre que la bataille : en Ultra, atlas 8192 sur
4 cascades, soit 4 cartes de 4096² (`cascade_map_size`). Après ADR 0746, à distance de caméra 30
(réglages du joueur, secteur, machine calme), les ombres des arbres plats restent le plus gros poste
(≈ 3,8 ms). Mesures, alternées deux fois :
- carte 2048 : 15,9 / 16,3 ms contre 19,8 / 19,3 ms en 4096 (−3,5 ms) ; d60 non mesuré en 2048 ;
- arbres ombrés dans moins de cascades (`CENT_ANS_VEG_CASCADES`) : 3 → −0,4, 2 → −1,1, 1 → −3,1 ms, mais
  à d30 la 2e cascade s'arrête vers 23 unités, avant le centre de la vue : rejeté (arbres sans ombre).

## Décision
- Nouveau champ `map_shadow_atlas` des préréglages (`data/fx/render_quality.json`, schéma à jour),
  `shadow_atlas` quand il manque ; le client prend `map_shadow_map_size` hors bataille.
- Valeurs : 2048 par cascade sur la carte pour tous les niveaux (legacy, high, ultra 4096 en atlas ;
  medium 2048 pour 2 cascades ; low inchangé). La bataille garde ses cartes.

## Conséquences
- Capture de contrôle à d25 (Paris) : ombres un peu plus douces et un peu moins sombres sur les murs,
  formes conservées.
- `CENT_ANS_SHADOW_SIZE` force toujours la taille pour un banc.

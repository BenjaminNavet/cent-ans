# Lot relief-ombre-ao (ombre de relief + occlusion d'horizon, campagne)

ADR 0745. Réglages : `data/fx/campaign_relief_shade.json` (schéma `fx_campaign_relief_shade.schema.json`).

## Conception
- Cuit dans le cache du sol par nœud (ADR 0742) : `relief_shade` (terrain.wesl) est appelé dans
  `bake_vertex` (un calcul par sommet de nœud, 65x65, interpolé), jamais dans le fragment principal.
- Marche du soleil + horizon (8 directions x 3 pas) sur `height_lod` (blocs 16 px, `textureLoad`
  seulement : aucun sampler ajouté). Sortie : atlas `Rg8Unorm` (r visibilité du soleil, g AO),
  3e cible de la passe de cuisson, binding 44 ; uniforme `ShadeParams` binding 43.
- Le soleil (quantifié 1 degré), l'échelle verticale (quantifiée) et les réglages entrent dans la
  signature du cache (`ground_cache::shade_signature`) ; recuisson au plus 1 fois/s. Compteur
  `ground_cache.shade_rebakes` dans le dump.
- `enabled:false` (ou cache du sol coupé, ou `height_lod` absent) : uniforme constant à zéro,
  aucune lecture dans le fragment, rendu identique.

## État
Fait : cuisson, réglages, dump (`ground_cache.shade_rebakes`, `shade_enabled`), clippy x2, tests
(1451 ok), bataille et campagne 300 images exit 0, 2 captures A/B (Alpes 6,9E 45,8N d=60,
`CENT_ANS_RELIEF_SHADE=0|1` force `enabled`). Effet modeste (ombres des crêtes dans les vallées,
plafonné par l'opacité max des ombres portées TS-T9 0,35-0,45). Réglage : `length_gain` allonge les
ombres, `ao_strength`. Reste : bancs release par la session principale.

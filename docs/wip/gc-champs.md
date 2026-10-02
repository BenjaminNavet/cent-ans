# GC5 — champs réduits (worktree `../gp-gc-fields`, branche `feat/gc-fields`)

Demande du joueur (02/10) : « on réduit aussi la taille des champs ».

## Fait
- `terrain.gdshader` : uniforme `field_scale` sur le parcellaire lointain (V2b) ; le motif est évalué
  en `p / field_scale`, haies, chemins et seuils d'apparition suivent. Défaut 0,3 (enclos ≈ 1 km au
  lieu de ≈ 3 km).
- `hb_ground.gdshaderinc` : `hb_cell_scale` 3 → 1,2 (parcelles par biome de 0,3 à 1,5 km au lieu de
  0,7 à 6,7 km ; haies de ≈ 20 m au lieu de ≈ 50 m).
- Planche `docs/img/gc/gc5_champs.jpg` (Paris, d 150 / 60 / 22, `tests/ss_shot.gd --param=…`) :
  A actuel, B ×0,4 / ×1,5, C ×0,25 / ×1. Retenu entre B et C : à d 22 un champ est nettement plus
  petit que Paris et qu'un village ; à d 60 le patchwork reste lisible ; à d 150 il devient une
  texture fine sous la carte de couleur.

## Ouvert
- Le semis des arbres de haie suit encore l'ancienne trame (`VegetationFields.LAYOUTS`, semis natif
  `core/crates/vegetation`) : relève de la session HC (arbres), signalé.
- Parcellaire de près ZG5b (`fine_parcels.gdshaderinc`, lanières réelles de 15-35 m) : sous le pixel
  au plancher de caméra visé (≈ 22) ; à retirer ou à grossir en GC6.
- Hameaux et moulins : après GC2 (tailles des maquettes).

## Prochaine étape
Fusion dans `feat/gc` après le retour de GC2.

# ZF — fondus au zoom (lot ZF-B : décor de campagne)

## État
- `game/scripts/map/cell_fade.gd` : rampe par cellule (`instance uniform cell_grow`) + LOD à hystérésis.
- Countryside / Fauna : cellule neuve = poussée en `spawn_seconds` ; cellule qui quitte le rayon se replie
  avant d'être masquée ; hystérésis `load_hysteresis` du rayon ; `lod_hysteresis` des seuils de LOD.
  Fauna : le test de rang binaire est remplacé par la croissance lissée (`vis`).
- Rocks : `spawn_seconds` / `view_hysteresis` (data/art/rock_outcrops.yaml) ; la part affichée de chaque
  tuile est lissée dans le temps : les instances (ordre aléatoire du tampon) apparaissent/disparaissent une
  à une (bord du rayon, chute à `far_share` au LOD2).

## Laissé de côté
- Pas de fondu tramé entre LOD (changement de maillage) : hystérésis seulement.
- Rocks : pas de tramage par pixel des instances retirées (granularité = 1 instance), shader inchangé.

## Prochaine étape
- Œil sur le zoom (captures par la session principale), réglage de `spawn_seconds`.

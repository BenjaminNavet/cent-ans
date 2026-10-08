# Lot AS2 — armées de la carte (pieds qui glissent, hampe qui suit)

Branche `feat/as2`. Rendu seulement, 0 $, aucun changement `core/`.

## État
- Squelette : `data/fx/campaign_army_walk.json` + schéma + test Python.
- A faire : (a) horloge d'animation par groupe dans `army_figures.gd` calée sur la vitesse
  écran / vitesse nominale du clip (`battle_gore.json`, `cadence`) ; (b) balancement/tangage de
  la hampe (`bearer_tilt`) + `map_banner.gdshader` (`pole_dir`, `gust`) ; drapeau `--no-as2`.

## Prochaine étape
Implémenter (a), puis (b), test `game/tests/as2_test.gd`, smoke, fusion de main.

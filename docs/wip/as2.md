# Lot AS2 — armées de la carte (pieds qui glissent, hampe qui suit)

Branche `feat/as2`. Rendu seulement, 0 $, aucun changement `core/`. Drapeau A/B : `--no-as2`
(après `--`) ou `enabled: false` dans `data/fx/campaign_army_walk.json`.

## État : fait
- (a) `army_figures.gd` : horloge propre à chaque groupe de figurines (`clock`), qui avance de
  `delta × facteur`. Facteur = vitesse au sol lissée du marqueur / (vitesse nominale du clip de
  `battle_gore.json` `cadence` × `FIGURE_SCALE` × échelle du marqueur), borné [0,35 ; 1,8] ;
  1 à l'arrêt (fondu idle/walk du shader inchangé, `apply_config` reçoit l'horloge du groupe).
- (b) Hampe : `bearer_tilt()` / `bearer_anchor()` / `bearer_gust()` ; à pied rebond + roulis +
  penché au rythme des pas (cadence incluse), en mer inclinaison réelle du navire amiral et
  pilonnement. `army_marker.gd` (`_follow_bearer`, `_process`) incline hampe, fleuron, drapeau
  autour du pied ; `map_banner.gdshader` : `pole_dir` (penche le tissu vu de la caméra) et
  `gust` (amplitude de claquement en marche).
- Tests : `game/tests/as2_test.gd`, `tools/tests/test_fx_campaign_army_walk.py`.
- Planche : `game/tests/as2_shot.gd` (fenêtre réelle, gitignorée).

## Points ouverts
- Vitesses nominales de `cadence` héritées de BV2 (estimées à l'oeil) ; groupe `cavalry_0`
  mélange le général agrandi et la cavalerie d'escorte (échelle du lord appliquée au groupe).
- Rendu non jugé visuellement (pas de capture lue) : à juger avec `as2_shot.gd`.

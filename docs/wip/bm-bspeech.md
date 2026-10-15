# bm-bspeech : discours du général, trajets en file, plan de victoire, touches

Branche `bm/bspeech`. Tout fait et prouvé (voir client/README.md § lot `bspeech`).

Restes :
- Actions `battle_*` à déclarer dans `settings::ACTIONS` (lot settings) et écran de réaffectation ; `battle_bindings` est lu en repli.
- Discours : durée d'une phrase estimée (`spoken_seconds`, pas de sonde du clip) ; choix des phrases par FNV (≠ Godot) ; le discours adverse n'a pas de sous-titre distinct de l'orateur ; la bulle du conseiller n'attend pas la fin des discours (`_advise_after_speeches`).
- Victoire : pas de bannière de l'étendard (orbite sur le général par `is_general`), jamais joué en jeu réel sur écran (preuves par dump seulement).
- Trajets : la destination en cours se recalcule quand le régiment avance de 2 m (cache) ; attaques en file restent des lignes droites rouges (comme Godot).

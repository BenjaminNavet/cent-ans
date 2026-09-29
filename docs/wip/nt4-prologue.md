# NT4 — bataille-prologue guidée

Branche `feat/nt4-battle-prologue` (partie de `integration/nt`). Spec : ligne NT4 de
`docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md`.

## Choix
- Entrée « Didacticiel de bataille » en tête de l'écran « Batailles historiques » (pas de ligne
  de plus dans la colonne du menu principal, tenue à 1280×720 par NT2).
- Bataille construite comme une bataille personnalisée (`BattleScene.custom_config` →
  `BattleSim.setup_custom`) + `BattleScene.prologue_data` : pas de déploiement, de discours ni de
  conseiller spontané ; titre du prologue.
- `game/scripts/battle/battle_prologue.gd` (`BattlePrologue`) : étapes de
  `data/tutorial/battle_prologue.json` affichées dans `TutorialOverlay` (parchemin du tutoriel de
  campagne, sans « Plus tard ») ; conditions pures `snapshot`/`evaluate` (caméra, sélection,
  déplacement, front/formation, état d'unité, pause, victoire) ; IA ennemie coupée (`set_ai`)
  jusqu'à l'étape `enemy_ai`. Aucun Rust touché.

## État
- [x] Données + schéma + pytest
- [x] Contrôleur, crochets de la scène, entrée de menu
- [x] Test Godot `game/tests/nt4_prologue_test.gd` (vert : étapes, évaluation pure, menu,
  bataille réelle jusqu'à la victoire anglaise en 146 s), smoke, ep7, nt2, pytest verts

## Prochaine étape
Terminé ; à fusionner dans `integration/nt`. Partie pilote à faire par le joueur.

## Points ouverts
- Aucune capture faite (consigne) : placement du parchemin sur le HUD de bataille et
  surlignage des cibles (`cards`, `pause`, `unit:*`) à juger visuellement.
- « Ennemi passif » = IA du cœur coupée : un régiment attaqué se défend et peut fuir (le test
  mesure jusqu'à 110 m de déplacement ennemi après la charge) ; pas de consigne « tenir » au cœur.
- Découvrabilité : l'entrée est dans « Batailles historiques » ; une invite au premier lancement
  (à la TWW3) n'est pas faite.
- Une défaite ferme le guide sans texte propre.

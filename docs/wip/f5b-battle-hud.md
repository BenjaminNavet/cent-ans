# F5b — HUD de bataille (audit UI § 3.2)

Branche : `worktree-agent-a9f2761a254f30730`.

## Plan
1. [x] `unit_card.gd` : cartes moitié moins larges (icône de classe, effectif, barres fines, infobulle formation, noms coupés sur un mot entier).
2. [x] `battle_groups.gd` : cartes groupées par bataille (avant-garde / corps / arrière-garde) ; Ctrl+chiffre enregistre, chiffre rappelle (raccourcis à vérifier).
3. [x] Suppression de la ligne d'aide permanente (F1 reste).
4. [x] Vitesses en boutons-icônes en bas à droite.
5. [x] `battle_minimap.gd` : minicarte cliquable.
6. [x] Smoke étendu, capture `docs/img/godot-battle-f5.png`, docs `m7-battles.md` et `status.md`.

## État
Terminé : smoke vert, capture relue. Reste hors périmètre : `help_controller.gd` (aide de la carte) mentionne encore « 1 / 2 / 3 : vitesse ».

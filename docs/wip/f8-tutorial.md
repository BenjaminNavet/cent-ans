# F8 — Tutoriel, encyclopédie, manuel (wip)

Tout est dans `main` (F8, puis UX2 « Plus tard »/sommaire, Q2, NT4 prologue de bataille).

## État
- [x] Réglages `tutorial/enabled|step|done` (`settings.gd`) + case « Tutoriel » (onglet Partie).
- [x] `game/scripts/ui/tutorial_steps.gd` : 14 étapes, conseils France/Angleterre/Bourgogne.
- [x] `game/scripts/ui/tutorial.gd` (parchemin + flèche/halo) + scène.
- [x] `game/scripts/map/tutorial_controller.gd` (objectifs, cibles, K, menu).
- [x] `game/scripts/ui/encyclopedia.gd` + scène.
- [x] Accroches `campaign_map.gd`, smoke « tutorial/encyclopedia ».
- [x] `docs/manuel.md`.

## Vérification du 2026-10-02
- Verts : `ux2_test.gd`, `nt4_prologue_test.gd`, étape tutoriel du smoke seule
  (`CENT_ANS_SMOKE_ONLY=tutorial`, ajouté ce jour, ~1 min au lieu du smoke complet).
- Essai hors dépôt sur 7 factions (Angleterre, Bourgogne, Novgorod, Albret, Aragon, Venise, Horde
  d'Or) : le guide démarre, aucun identifiant brut dans les textes, armée principale et cibles
  des flèches trouvées.
- Textes recalés sur l'interface actuelle : cloche « Fin de tour » en bas à droite ; chantier
  lancé d'un clic sur la ligne (plus de bouton « Construire »), une construction par ville et
  non par province ; « Votre faction : … » (l'ancien « Vous gouvernez Royaume de France »
  n'avait pas d'article).

## Points ouverts traités le 2026-10-02 (02f2f1509, 06b0597f2)
- Dirigeant, suzerain et objectifs lus dans la simulation (`get_feudal_sheet`) : les factions sans
  bloc `victory` affichent « Les objectifs de votre titre », celles sans `ruler` dans les données
  le dirigeant tiré par la simulation ; « Bienvenue » seul s'il n'y en a aucun.
- `TutorialSteps.GENERIC_ADVICE` : conseils d'époque pour les factions sans texte propre ;
  l'introduction reprend la `description` de la faction, la diplomatie nomme le suzerain.
- Le guide se range sous le menu pause (le contrôleur tourne arbre en pause, régression depuis
  Q8) et sous le rapport de saison hors de son étape (il couvrait « Continuer »).
- `label_safe_rect()` borné à une taille nulle : plus de « Rect2 size is negative » en headless.
- Parcours aux vrais clics `q1_playtest.gd --phase=q2tutorial` vert pour France et Albret
  (introduction → construction, pause, 3 fins de tour) ; il utilise désormais ses propres
  réglages et sauvegardes et écarte l'invitation au prologue de bataille. La fenêtre doit rester
  au premier plan (`osascript … set frontmost`).

## Points ouverts
- Le parcours aux vrais clics s'arrête à l'étape « recherche » : les étapes suivantes ne sont
  jouées que par appels directs dans le smoke (France).
- Conseils propres à une faction : toujours France, Angleterre, Bourgogne seulement.

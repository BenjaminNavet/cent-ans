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

## Points ouverts
- 146 factions jouables sur 149 n'ont pas de `victory` : l'introduction affiche « Vos objectifs
  historiques : • Survivre et prospérer. ». 22 n'ont pas de `ruler` : titre « Bienvenue, Sire »
  (faux pour les républiques). Conseils historiques seulement pour France/Angleterre/Bourgogne.
- Le smoke ne joue le guide que par appels directs (France) ; le parcours aux vrais clics est
  `q1_playtest.gd --phase=q2tutorial` (fenêtré, non relancé le 2026-10-02).
- Hors tutoriel, vu dans les logs headless : `settlement_layer.gd:1086` (`_declutter_step`)
  émet « Rect2 size is negative » à chaque passe quand la fenêtre est minuscule (51 px en
  headless) : `label_safe_rect()` devient négatif.

# Lot A3 — documentation (nuit critiques du 2026-10-22)

Branche : `worktree-agent-af6622fbae9f81299` (avancée en fast-forward sur `main` a8effdd93 avant travail).
Sources : `docs/wip/nuit-critiques/{ui-ux,production,animation-vfx,outils-build}.md`.

## État
- [x] (1) `docs/manuel.md` § 15 réécrit d'après `client/src/settings/keymap.rs` (`ACTIONS`).
      Corrigé aussi, hors § 15 et même défaut : lignes « Modes de carte » (mécontentement = Maj+M),
      « Barre du haut » (Diplomatie F3, Technologies F4), panneau Diplomatie (F3), médecine (F4).
- [x] (2) `docs/status.md` et `docs/roadmap.md` à l'état du 2026-10-22 : Bevy (ADR 0357, 0362, 0366),
      dépense 136,39 $ (sortie de `cent-ans budget summary`), plafond 50 $ dépassé, sans trancher.
- [x] (3) `docs/animation.md` réécrit pour la chaîne Bevy (tables code / shader / réglages).
- [x] (4) `docs/decisions/DOUBLONS.md` : 0117, 0146, 0149, 0205, 0357 (dix fichiers), rien renommé.

## Écarts à la spec
- Le fast-forward de la branche sur `main` (996 commits) a été fait avant travail : la branche d'origine
  était à `ef2bbfaae`, en retard de `main`, et ne contenait pas les notes de nuit.
- `docs/budget.md` non modifié : sa synthèse (117,39 $ au 09/10) est fausse par rapport à la sortie de
  l'outil (136,39 $). À régénérer par le propriétaire du budget.

## Points ouverts (joueur)
- Plafond budgétaire global : 136,39 $ dépensés contre 50 $ de plafond v1.
- Push : 996 commits non poussés.
- Critères de sortie d'une version jouable : non définis.
- `docs/manuel.md` § 16 (Crédits) cite encore « moteur Godot, godot-rust » : non modifié (hors périmètre).
- Test proposé par le critique UI/UX (vérifier que le manuel § 15 diverge de `keymap.rs`) : non écrit,
  car le lot est docs seules.

# ADR 0005 — Colonies prenables à l'intérieur des provinces

Date : 2026-09-24. Statut : accepté. Complète la section 4.1 du document de conception.

## Contexte

Avec une ville par province et un relief à 719 m/px, la carte se joue comme Crusader Kings. Le joueur
veut l'échelle de Total War : zoom jusqu'au comté, beaucoup plus de lieux habités et de places fortes.

## Options

- **A** : découper les provinces en ~550 comtés, chacun devenant l'unité atomique.
- **B** : garder les 132 provinces et y placer 3 à 6 colonies prenables séparément.
- **C** : mouvement continu des armées (façon Total War).

## Décision

**B**, choisie par le joueur. La province garde la terre et les gens (population, révolte, dévastation,
hérésie, régime) ; la colonie porte propriétaire, contrôleur, garnison, siège, bâtiments, recrutement
et construction. Le contrôle d'une province est celui de sa cité. Les armées se déplacent sur le
graphe des colonies, le long des routes. Relief porté à 8192² en tuiles ; hameaux réels en décor.
Détail : `docs/design/2026-09-24-echelle-colonies.md`.

## Conséquences

- Refonte de `ProvinceState` et de tout le code qui lit sa garnison, son siège ou ses bâtiments ;
  `Army.location` devient un `SettlementId`.
- `STATE_VERSION` 5 : les sauvegardes antérieures sont refusées.
- ~550 colonies à documenter historiquement, avec sources.
- Le mouvement continu (C) reste possible plus tard : le graphe des colonies en serait le squelette.

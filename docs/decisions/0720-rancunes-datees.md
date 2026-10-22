# 0720 — Rancunes datées : des missions ancrées dans un affront subi

Statut : accepté (22/10, lot M1 `tx/m1-rancunes`).

## Contexte
Les missions (ADR 0127, 0304) sont tirées de la situation présente ; aucune n'est ancrée dans un fait
passé. Total War: Warhammer III crée une mission « grudge » quand on subit un affront, et sa récompense
croît avec l'ancienneté de l'affront.

## Décision
- `MissionsState` garde une liste de rancunes (`Grudge` : faction fautive, lieu, tour, nature : place
  perdue, armée détruite, province pillée). Elles sont ajoutées par `capture::on_captured`, le rapport de
  bataille et le pillage.
- Une rancune alimente l'offre de 3 missions (poids par nature dans `data/missions.json`, section
  `grudge`) tant que la guerre dure ; annulée sans pénalité par paix/trêve ou reprise de la place par un tiers.
- Récompense multipliée par l'âge (seuils 8/16/24/32 saisons -> x1/x1,5/x1,5/x2/x2) ; place emblématique
  x2 en prestige ; pénalité d'échec séparée (prestige -2 x multiplicateur plafonné x1,5).
- Tous les nombres vivent dans `data/missions.json` (+ schéma) ; générateur dérivé, sans toucher `state.rng`.

## Conséquences
- Joueur seul (comme les autres missions). M2 (libérer) et M4 (facteur mission) s'appuient sur l'annulation
  et l'issue des rancunes.

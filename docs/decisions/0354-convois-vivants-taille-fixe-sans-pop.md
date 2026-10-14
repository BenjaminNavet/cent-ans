# 0354 — Convois vivants : taille monde fixe, aucun pop, modèles Meshy

Date : 2026-10-14. Statut : accepté. Révise ADR 0340 (VG2 : taille croissante avec la distance).

## Contexte
Le joueur juge les caravanes, charrettes et bêtes de la carte peu crédibles : modèles TRELLIS 1 ou
boîtes procédurales, apparitions/disparitions brutales, taille qui grossit avec le dézoom.

## Décision
- Taille monde fixe pour les figurines (`FolkPool`) et les accessoires routiers du calque campagne
  (`CountrysideLayer`) : plus de croissance avec la distance ; échelle maquette nettement plus grande.
- Plus aucun pop : semis ancré sur le monde (indépendant du point visé), éclaircie au dézoom par un
  aléa fixe d'instance comparé à un seuil continu en shader (fondu tramé), fondus au bord de la zone
  et à la portée au lieu d'un vidage instantané.
- Modèles : charrettes, caravanes, bouviers et bêtes régénérés en Meshy 6-lite (chaîne MX) ; les
  accessoires FK2 procéduraux sont remplacés par ces glb DN.

## Conséquences
- Densité à l'écran plus faible en vue lointaine (assumé : réaliste).
- Détails d'implémentation et réglages : `docs/wip/cv-convois-vivants.md`.

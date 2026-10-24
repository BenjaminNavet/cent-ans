# 0785 — Victoire par score de règne (amende 0245)

Statut : accepté (lot ACTES, 24/10/2026). Amende l'ADR 0245 (victoire féodale générique).

## Contexte

L'ADR 0245 faisait gagner la faction qui remplit tous ses objectifs et les tient `hold_turns` saisons (jusqu'à 40, soit dix
ans), ou qui atteint une victoire féodale générique (indépendance, premier vassal, couronne) après 1360. La partie est ainsi
un seuil à franchir et à garder, sans bilan ni récit.

## Décision

- Quand `data/rules/acts.json` est actif, la victoire est le **bilan pondéré des actes** : score de règne =
  somme des scores d'acte (0-100) pondérée par les poids des actes, ramenée à 0-100. Victoire si le score atteint
  `victory_min_score` (55) à la fin du dernier acte (1453) ; en dessous, la campagne se clôt (`Ended`). Une campagne courte
  se clôt à son année et compte l'acte en cours tel quel.
- Le score de l'acte est la part pondérée des objectifs remplis (un objectif non rempli rapporte la moitié de son avancement),
  corrigée des pressions, bornée à 0-100.
- Les victoires `hold_turns` (objectifs de `factions/*.json`) et la victoire féodale générique ne s'appliquent plus tant que
  l'arc est actif ; `feudal.generic_victory` et ses règles restent pour les données sans arc (tests de `feudal_titles`).
- Défaite : ruine de la maison (ADR 0784) ou perte de toutes les terres. `Outcome::score` porte le score de règne ; l'écran de
  fin liste le bilan de chaque acte, et les trois causes principales du journal (ADR 0754).
- Échelles de grade (`grades`) : règne ruineux, contesté (40), honorable (55), couronné (70), glorieux (85).

## Conséquences

- Plus de victoire passive ni de seuil unique : même une partie sans conquête a un bilan lisible, et une partie réussie
  l'est sur trois temps. L'IA n'est pas modifiée.
- Le score de campagne historique (`campaign_score`) reste utilisé hors arc.
- À rééquilibrer avec les seed bots : seuils d'objectifs et de victoire.

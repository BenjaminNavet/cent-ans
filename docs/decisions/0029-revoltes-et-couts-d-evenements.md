# 0029 — Révoltes à délai, troubles de province et coûts d'événements proportionnels

Date : 2026-09-25. Lot : EQ1 (équilibre). Statut : accepté.

## Contexte

Les sondes (`balance_probe` 8 × 200 tours, `century_probe` 5 × 464 tours) montraient :

- un mécontentement moyen de 9 à 11 (cible E2 : 15-35), mais 14 révoltes par partie ;
- ces révoltes venaient d'une poignée de provinces (Boulonnais, Savoie, Biscaye) qui se
  soulevaient **à chaque saison** : une fois le seuil de 75 dépassé deux saisons, le compteur
  n'était jamais remis à zéro ;
- deux jauges de trouble distinctes : `ProvinceState::unrest` (prises, pillages, régence, qui
  décroît seule) affichée par le panneau de province, et le mécontentement pondéré des classes,
  qui déclenche les révoltes. Le joueur voyait un chiffre bas et subissait une révolte ;
- des banqueroutes de mineures (Suisses, Grenade, Gueldre, Suède) causées par des événements
  génériques à coût fixe (−1 000 à −2 500 ₶) pour un revenu de 400 à 600 ₶ par saison, et par la
  faction virtuelle des rebelles, qui paie un entretien sans revenu.

## Décision

1. **Révolte à délai et à répit** (`data/rules/population.json`) : seuil (75), nombre de saisons
   consécutives (3, contre 2) et seuil de passage aux rebelles (90) deviennent des données ;
   **le compte repart de zéro après chaque révolte**. Une province tenue par les rebelles ne se
   soulève plus contre eux.
2. **Les troubles de province nourrissent le mécontentement** : la jauge `ProvinceState::unrest`
   entre dans la cible de chaque classe avec un poids `disorder_unrest_weight` (0,5). Poids de
   l'impôt 100 → 130, occupation 35 → 20 : le mécontentement de fond monte (impôt), les provinces
   occupées ne sont plus des foyers perpétuels.
3. **Le panneau de province affiche le mécontentement qui déclenche les révoltes** (pondéré par
   classe) et, dès qu'il dépasse le seuil, le compte à rebours « révolte dans N saisons ». La
   jauge de troubles reste disponible sous la clé `disorder`.
4. **IA** : pas d'impôt « Haut » tant qu'une de ses provinces est à moins de 10 points du seuil
   de révolte ; plafond de mécontentement moyen pour l'impôt Haut 18 → 30 (le fond est plus haut).
5. **Coût des événements proportionnel au revenu** (`data/rules/economy.json`) : un effet de
   trésor s'applique en entier à partir de 4 000 ₶ de revenu de saison, en proportion en
   dessous, jamais sous 25 % du montant écrit. L'infobulle de la décision affiche le montant réel.
6. **Les rebelles sont hors de l'économie** : ni impôt, ni entretien, ni banqueroute.

## Conséquences

- Mécontentement moyen dans la cible, révoltes rares, précédées de plusieurs saisons visibles.
- Les petits royaumes gardent la tension (un événement coûte toujours au moins une part du
  montant) sans banqueroutes à répétition.
- Les parties de campagne changent pour une même graine (règles et données) ; aucune empreinte
  de déterminisme de campagne n'est figée dans les tests (seule `sim-battle` en a, inchangée).
- Mesures avant/après : `docs/wip/eq1-equilibre.md`.

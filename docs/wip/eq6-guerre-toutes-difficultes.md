# EQ6 — guerre France-Angleterre 55-75 % à tous les niveaux de difficulté

Branche : `feat/eq6-war-all-difficulties` (main fusionné).

Décision du joueur (2026-09-26) : la cible « France et Angleterre en guerre 55-75 % du siècle »
vaut à tous les niveaux, pas seulement en Normale.

## État : en cours

- [x] Mesure de référence (century_probe 464 tours, mêmes graines qu'EQ4/EQ5) : identique à EQ5.
- [x] Sonde : tableau « EQ6 » (guerres ouvertes par l'Angleterre / la France, tours de paix et
      obstacles à la déclaration anglaise), `WAR_TRACE=1` (détail tous les 5 ans).
- [x] Diagnostic par niveau (ci-dessous).
- [x] Correctif v1 : `war.main_claim_first` + `war.claim_war_ignores_difficulty`
      (`data/ai/diplomacy.json`, cœur `diplomacy.rs::war_target`).
- [ ] Mesure v1, réglage, non-régression EQ4/EQ5.
- [ ] ADR 0077.

## Diagnostic (référence main)

Obstacles comptés à chaque tour de paix FR-EN (plusieurs par tour possibles) :

- **Facile** : l'attitude de l'Angleterre envers la France dépasse 20 (seuil de la guerre de
  prétention) dans 60-70 % des tours de paix. La raison « Niveau de difficulté » (+10 envers le
  joueur) s'ajoute à « Même foi » (+10), aux mariages (+30) ou aux ambassades de hérauts (+18) :
  sans elle, l'attitude reste sous 20. Graine 4 : paix de 1420 à 1449.
- **Difficile / très difficile** : le « repos » de 12 tours après toute déclaration de guerre
  bloque 50-80 % des tours de paix. L'Angleterre, plus riche, déclare sans cesse d'autres guerres
  (croisade contre Grenade, prétention héritée par mariage sur Vérone tous les 3 ans de 1403 à
  1448 en difficile graine 4) ; `war_target` classe les prétentions par rapport de forces, donc la
  plus petite couronne revendiquée passe avant la France. En très difficile, guerres courtes
  (15 trêves) : la France affaiblie cède vite des provinces.
- **Normale** : même mécanisme, moins marqué (l'Angleterre moins riche déclare moins ailleurs) ;
  graines 4 et 8 bloquées par l'attitude (mariages, hérauts).

## Prochaine étape
Lire la mesure v1 (`scratchpad/v1`), régler la durée des trêves ou la fatigue si la Normale
dépasse 75 %.

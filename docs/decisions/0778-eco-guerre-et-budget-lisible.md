# ADR 0778 — Éco-guerre et budget lisible (lot ECO2)

Date : 2026-10-23. Statut : accepté. Complète l'ADR 0777 ; amende les ADR 0183 (rançons, butin), 0257 et 0749 (mesure de référence).

## Contexte

Chantier C du juge (`out/juge/iter-03-design.md`) : « éco-guerre : logistique, rançon, butin ». La guerre coûtait de l'entretien mais ne rapportait presque rien (butin d'étendard 15 ₶, bagages 120 ₶), et l'économie restait une feuille de chiffres sans poste lisible pour le joueur.

## Décision

1. **Coût du siège** (`finance.json` § `siege`, ADR 0777 § 1) : chaque siège conduit coûte 150 ₶ par saison, +25 % par saison écoulée (plafond +200 %), au plus 15 % du revenu brut. Un siège long est un choix : l'affamer coûte du temps, le donner d'assaut coûte des hommes, le laisser durer coûte de l'argent. Il apparaît dans le budget (ligne `siege`) et dans la cause d'une banqueroute.
2. **Butin et rançons plus attractifs** (données seules) : `battle_outcome.json` § `spoils` : or par étendard pris 15 -> 40, bagages 120 -> 400 ; `capture.json` § `raid` : `loot_share` 0,5 -> 0,8 (chevauchée). Le plafond de rançon (une saison de revenu du payeur, ADR 0183) et le pillage (`sack`) sont inchangés. Le butin reste payé par le trésor de la victime : il déplace de l'argent, il n'en crée pas.
3. **Logistique** : le ravitaillement par saison (`supply_loss`, hiver, dévastation, ADR SV4) existait ; ce lot n'y ajoute rien. La suppression de l'entretien des bâtiments (idée du juge) est écartée : l'entretien reste, la maison du roi et les gages s'y ajoutent.
4. **Budget lisible par poste** pour chaque faction, dans `game-api` :
   - `EconomyView` : `household`, `wages`, `siege_costs`, `obligations`, `interest`, `debt` en plus des postes existants ;
   - `budget_lines` : recettes, impôts exceptionnels, armées, bâtiments, table, administration, maison du roi, gages, sièges, obligations, intérêts, emprunts nets, autres mouvements (prévu et saison passée) ; le solde « autre » n'absorbe plus les emprunts ni les impôts exceptionnels ;
   - `FactionPanelView.finance` (`FinanceView`) : banquiers (taux effectif, marge), prêts, impôts disponibles (rendement, grogne, délai), obligations, cote, risque de banqueroute.
5. **Client** (panneau `campaign/panels/faction`, seulement) : les nouveaux postes rejoignent le tableau de budget existant (`RUBRICS`), plus une ligne « Dette aux banquiers » avec le risque de banqueroute. Les ordres `Borrow`, `Repay` et `LevyExtraordinary` n'ont pas encore de boutons : à faire avec le kit UX2.
6. **Mesure de référence** : `campaign_probe --passive` donne désormais, en plus de la série `france_treasury`, les banqueroutes ordinaires, les banqueroutes royales, les impôts levés et la dette totale.

## Mesures (`campaign_probe --turns 40 --seeds 1,2 --passive --full`, France inerte)

| Mesure | Avant (ADR 0749) | Après |
|---|---|---|
| Net initial de la France | +20 k | +14,3 k (maison du roi 18 % + prestige, gages) |
| Pic du trésor France | 135-136 k (6,8 saisons du net initial) | 87,6 k / 80,3 k (6,1 / 5,6 saisons) |
| Trésor France, tour 40 | 81-83 k | 36 k / 37 k |
| Banqueroutes ordinaires, 177 factions | 4 / 5 | 9 / 18 (< 53, borne de l'ADR 0183) |
| Banqueroutes royales | 0 | 1 / 1 |
| Impôts exceptionnels levés par l'IA | 0 | 33 / 31 |
| Factions endettées en fin de partie | 0 | 10 / 7 |

France jouée par l'IA (sans `--passive`) : trésor de 80 k au tour 4 à 13-32 k au tour 40, solde de -15 à -27 k par saison à la fin : l'argent est rare dès qu'on dépense. Seed bots `--quick` : 8 parties, aucun constat.

## Conséquences

- La ponction d'épargne (ADR 0749) est resserrée (paliers à 1, 2 et 4 saisons de revenu) pour que l'équilibre du trésor passif suive les nouvelles charges ; elle reste un filet, la dette et les charges étant les vrais puits.
- `starting_fit::structural_balance` ne compte pas la maison du roi ni les gages : l'ajustement des armées de départ (ADR 0183) ne retire pas d'unités pour payer une cour.
- Points ouverts : boutons du client pour emprunter, rembourser et lever un impôt (lot UX2) ; tribut et subsides de guerre comme obligations (aucun producteur) ; passe longue des seed bots non lancée ; la part de milice et l'IA de campagne n'ont pas été rééquilibrées avec les coûts de siège.

# ADR 0777 — Le roi et sa dette : charges structurelles, crédit, impôts exceptionnels, banqueroute royale (lot ECO2)

Date : 2026-10-23. Statut : accepté. Amende les ADR 0183 (§ administration), 0257 (opulence) et 0749 (ponction d'épargne) ; complète l'ADR 0754 (journal des causes).

## Contexte

Le juge (`out/juge/iter-03-design.md`, économie 5/10, constat 5 et chantiers A et C) : le trésor de la France passe de 28 000 à 143 000 au tour 12 puis plafonne vers 133 000, « toujours aucun choix monétaire ». La ponction progressive de l'ADR 0749 règle la thésaurisation mais c'est un impôt, pas un dilemme : aucune décision n'y répond. Le jeu avait déjà la monnaie (`coinage.rs`, ADR H5), les rançons à tempérament et les édits, mais ni dette ni charge structurelle de la couronne.

Mesure de départ (`campaign_probe --turns 40 --seeds 1,2 --passive --full`, France inerte) : net initial +20 k, pic du trésor 135-136 k (6,8 saisons du net initial), 81-83 k au tour 40 ; banqueroutes 4 / 5.

## Décision

Toutes les valeurs sont dans `data/rules/finance.json` (schéma `finance_rules.schema.json`, `data_model::FinanceRules`) ; le mécanisme est `sim-campaign/src/finance.rs`. Le fichier est optionnel : sans lui, rien de ce qui suit n'existe.

1. **Charges structurelles** (comptées dans `TurnBudget::upkeep()` et donc dans la prévision, le budget affiché et la banqueroute) :
   - maison du roi : part du revenu brut selon le rang (paliers de provinces tenues : 3 : 5 %, 6 : 18 %, 12 : 24 %, 25 : 30 %), plus 1 point par 10 de prestige du souverain, plafond 40 % ; les comtés de moins de 3 provinces n'en paient pas ;
   - gages : 60 ₶ par gouverneur, 120 ₶ par armée menée par un général, plafonnés à 10 % du revenu brut ;
   - sièges : 150 ₶ par siège conduit et par saison, +25 % par saison de siège écoulée (plafond +200 %), au plus 15 % du revenu ;
   - obligations (`FactionState::finance.obligations`) : dot de mariage (la maison de l'épouse verse 1 saison de revenu brut en 4 versements au royaume de l'époux, hook `propose_marriage`), saisie des banquiers après une banqueroute, tribut ; chaque versement crédite le créancier en fin de saison (`resolve_finance`) ;
   - intérêts de la dette.
   Les soldes arriérées (`unpaid_seasons`), l'entretien des places et la rançon du roi captif (échéances annuelles de `ransom.rs`) existaient déjà ; la rançon restante est exposée dans la vue (`ransom_owed`).
2. **Crédit.** Deux maisons de banque (`lombards` : 4,5 % par saison, plafond 2 saisons de revenu brut, prêt minimal 500 ; `florentines` : 6 %, plafond 4 saisons, minimum 3 000). Ordres `Borrow { lender, amount }` et `Repay { amount }` (les prêts les plus chers d'abord). Une **cote de crédit** (0-100) majore le taux de 0,6 ‰ par point manquant et réduit le plafond ; elle remonte de 2 points par saison sans banqueroute.
3. **Banqueroute royale** (`maybe_default`) : trésor négatif en fin de saison et dette supérieure à 4 saisons de revenu brut. Effets (données `credit.default`) : 40 % de la dette effacée, trésor négatif ramené à 0, cote -50, crédit gelé 20 tours, saisie de 20 % du revenu pendant 8 saisons, grogne +10 dans chaque province pendant 8 saisons, -15 de prestige du souverain. C'est la faillite d'Édouard III en 1345 rendue jouable : emprunter est un pari sur la suite de la guerre.
4. **Impôts exceptionnels.** Ordre `LevyExtraordinary { levy }` : aide (0,5 saison de revenu, grogne +4 pendant 4 saisons, délai 6), gabelle (dès 1341 : 0,9, +7 pendant 8 saisons, délai 12), fouage (dès 1355 : 0,7, +6 pendant 6 saisons, délai 8). La grogne passe par `finance_unrest`, ajouté aux effets d'agitation de chaque province comme l'interdit (ADR 0326) : la jauge de tension des révoltes (ADR 0731) fait le reste.
5. **Mutation de monnaie** : inchangée (`coinage.rs`), sauf que l'affaiblissement du joueur est journalisé.
6. **Ponction d'épargne revue** (`economy.json`, remplace les paliers de l'ADR 0749) : 15 % de ce qui dépasse 1 saison de revenu, +20 % au-delà de 2 saisons, +25 % au-delà de 4 saisons. Avec des charges structurelles plus lourdes, le trésor passif de la France se stabilise à environ 5,6-6,1 saisons du net initial.
7. **IA** (`finance::ai_finance_orders`, branché dans la planification d'`ai`) : rembourse quand le trésor dépasse 1,5 saison de revenu, sinon, si le trésor prévu tombe sous 0,25 saison, lève l'impôt exceptionnel au meilleur rendement par point de grogne (si la grogne moyenne est sous 35), sinon emprunte au moins cher des banquiers qui a de la marge. La dévaluation reste décidée par `coinage::ai_choose_coinage`.
8. **Journal des causes** (ADR 0754) : trois `CauseKind` (`royal_default`, `extraordinary_tax`, `debasement`), constructeurs dans `cause_builders` (`default_cause`, `levy_cause`, `debasement_cause`), facteurs et textes dans `data/ui/causes.fr.json` ; la banqueroute ordinaire gagne les nouvelles lignes du budget (maison, gages, sièges, obligations, intérêts). Causes consignées pour le joueur seulement, comme les autres.
9. **Vues** (`game-api`) : `EconomyView` gagne `household`, `wages`, `siege_costs`, `obligations`, `interest`, `debt` ; les lignes de budget (`budget_lines`) et l'historique (`BudgetRecord`) distinguent maison, gages, sièges, obligations, intérêts, impôts exceptionnels et emprunts/remboursements (le solde « autre » ne les compte plus) ; `FactionPanelView.finance` (`FinanceView`) donne banquiers, marges de crédit, prêts, taux, impôts disponibles et leur rendement, obligations, risque de banqueroute.

## Mesures

Voir l'ADR 0778 (tableau avant/après du lot).

## Conséquences

- Aucun changement de `STATE_VERSION` : les nouveaux champs d'état sont `#[serde(default)]`, absents des anciennes sauvegardes.
- Les factions de 1 à 2 provinces ne paient ni maison ni gages hors plafond de 10 % : leur sort ne dépend que de la dette qu'elles choisissent (l'IA n'emprunte pas sous le prêt minimal).
- La dette est un pari : la banqueroute royale efface une partie de la dette mais gèle le crédit et assombrit les provinces.
- Points ouverts : tribut de guerre et subsides ne créent pas encore d'obligation (le mécanisme existe, aucun producteur) ; saisie de provinces par les banquiers non faite (remplacée par la saisie de revenu) ; la cote de crédit n'est pas encore liée au prestige.

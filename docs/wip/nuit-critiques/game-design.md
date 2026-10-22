# Critique game design (nuit du 22/10/2026)

Rôle : game designer (systèmes, équilibrage, progression, boucle). Sources : `CLAUDE.md`, design 2026-09-23, `docs/status.md`, `docs/wip/refonte-systemes.md`, `data/rules/*.json`, `data/buildings`, `data/unit_types`, `data/factions`, `data/balance`. Seed bots `run --quick` lancé : 8 parties, 67 s, « passe partielle (plafond de durée atteint) », 0 constat, **aucune métrique d'équilibre dans le rapport** (rapport : `core/target/seed-bots/2026-10-22_020349/report.md`). Les constats ci-dessous reposent donc sur la lecture des données, pas sur des mesures neuves ; les chiffres de révoltes viennent de `refonte-systemes.md`.

## 1. L'instrument d'équilibrage est aveugle
- **Problème** : les bandes cibles des seed bots sont inutilisables : `revolts_per_game` 0 à 112,5, `bankruptcies_per_game` 0,75 à 163,75, `factions_eliminated` 4,5 à 45, toutes « initialised » (min/max issus d'une première passe, pas d'une intention de design). Aucune borne ne peut échouer. Trois métriques de révoltes sont encore `to_initialise`. La passe `--quick` est coupée par le plafond de durée et ne sort aucun chiffre.
- **Preuve** : `data/balance/seed_bots_bands.json` l.2-9 ; rapport `--quick` ci-dessus ; bande 3-15 (seed bots) contre 4-10 (ancien `balance_probe`) dans `refonte-systemes.md` § 1A.
- **Gravité** : majeur (tout le chantier de refonte est conditionné par la mesure).
- **Correctif** : fixer des bandes de design écrites à la main (ex. révoltes 4-10, faillites 0-3 par partie, 4 grandes factions vivantes en 1400), faire échouer l'oracle « dérive d'équilibre » hors bande, et faire sortir le tableau des métriques même en passe partielle.
- **Effort** : S

## 2. Pas de boucle de rétroaction sur l'ordre public : le compteur de 5 saisons
- **Problème** : révolte seulement après 5 saisons consécutives au-dessus de 75, compteur remis à zéro à chaque rémission. Le joueur n'a aucun signal graduel : il passe de « tout va bien » à une révolte, et un décrochage d'une saison annule tout. Les bots oscillent donc autour du seuil sans conséquence ; l'impact varie de 0 à 4,3 révoltes par partie selon le bot (aggressor 4,3, diplomat 0).
- **Preuve** : `data/rules/population.json` (`revolt_unrest_threshold` 75, `revolt_seasons` 5, `disorder_decay_flat` 2) ; `refonte-systemes.md` § 1 (mesures du 22/10).
- **Gravité** : majeur.
- **Correctif** : lot B du plan (jauge de tension cumulée qui monte au-dessus du seuil et redescend lentement), avec paliers visibles (avertissement à 50 %, effets de production à 75 %, révolte à 100 %).
- **Effort** : M

## 3. Une révolte est quasi sans enjeu et sans contre-jeu
- **Problème** : une révolte coûte 25 % de garnison, la ville ne change de main qu'au-dessus de 90, sans armée ni bataille. Le joueur ne peut pas y répondre par le jeu militaire (le cœur du jeu), seulement par des curseurs (impôt, bâtiments). La province occupée est pourtant celle qui se révolte le plus (prime d'occupation 24).
- **Preuve** : `population.json` (`occupation_unrest` 24, `revolt_control_threshold` 90) ; `refonte-systemes.md` § 1 lots C et D non faits.
- **Gravité** : majeur.
- **Correctif** : lot C (armée rebelle proportionnée à la richesse et à la culture, que l'on réprime en bataille ou en auto-résolution pour regagner de l'ordre) puis lot D (choc ponctuel selon occuper, piller ou raser).
- **Effort** : L

## 4. Économie : l'armée écrase tout, les bâtiments n'offrent presque pas de choix
- **Problème** : entretien d'un chevalier 130 par mois, soit 520 par saison (4 mois) pour un coût d'achat de 1 400 ; une gendarmerie d'ordonnance 150/mois pour 1 600 : en 3 saisons l'entretien dépasse l'achat. Côté revenu, `domain_income` 150 par saison et `tax_efficiency` 0,082 : un bâtiment à 300-900 livres de coût rapporte peu en regard. Sur 90 effets de bâtiments, seulement 2 touchent le coût de recrutement, 1 la croissance, 4 l'armée : le joueur n'a quasi pas de « build » à choisir (ce qui confirme le constat du plan § 3).
- **Preuve** : `data/unit_types/unit_knights.json` (1400, upkeep 130), `unit_ordonnance_gendarmes.json` (1600, 150) ; `data/rules/economy.json` l.17, 30, 52 ; décompte des effets sur `data/buildings/*.json`.
- **Gravité** : majeur.
- **Correctif** : lots 0 (métriques éco), E (bâtiments qui réduisent coût et entretien des armées) et C (croissance par bâtiments agricoles/portuaires) ; ajouter 6-8 effets de bâtiment « armée » (arsenal, haras, caserne) et des groupes exclusifs (lot F).
- **Effort** : L

## 5. Entretien de bâtiment : peu de risque, donc peu d'arbitrage
- **Problème** : entretien moyen 16,6 par bâtiment pour un coût moyen de plusieurs centaines ; la démolition rembourse 10 % ; l'IA n'en rase qu'après plusieurs saisons de déficit. La décision « construire / ne pas construire » est surtout une question de trésor, pas de rentabilité. À l'extrême : `bld_artillery_bastion` 6 000 (+60/saison) sans retour économique ; `bld_banal_oven` 200 (+3).
- **Preuve** : `data/buildings/*.json` (champs `cost`, `upkeep`) ; `economy.json` (`demolition_refund_percent` 10).
- **Gravité** : mineur.
- **Correctif** : lot D (entretien faible ou nul, la vraie dépense restant l'armée) ou, à l'inverse, entretien proportionnel au niveau de la ville ; trancher dans un ADR.
- **Effort** : S

## 6. Emplacements fixes : progression de ville plate
- **Problème** : le nombre d'emplacements est fixe, sans gain par palier de taille ; la croissance démographique n'est portée que par un seul bâtiment. Une province prospère ne « débloque » rien : la progression est latérale (plus de provinces) et non verticale (mieux développer).
- **Preuve** : `refonte-systemes.md` § 3 A, B, C ; décompte des effets (`growth` : 1).
- **Gravité** : majeur (boucle de progression centrale).
- **Correctif** : lot A (emplacements par palier de taille) combiné au lot C (croissance par bâtiments), avec une réserve de développement par province (lot B) pour borner le total.
- **Effort** : M

## 7. Conditions de victoire : une seule échelle de durée pour des situations très différentes
- **Problème** : `hold_turns` est 20 pour la France, 12 pour l'Angleterre et la Bourgogne, mais 8 pour toutes les petites factions vérifiées (Pologne, Ottomans, Byzance, Mamelouks). Les objectifs de l'Angleterre mêlent `control_all` (Paris, Guyenne, Normandie, Poitou), un `treasury` de 30 000 et la soumission de l'Écosse : un objectif de trésor (30 000) peut être atteint par thésaurisation sans conquête (une seule saison de revenu de domaine = 150). Le trésor de départ de l'Angleterre est 12 000, donc ≈ 2,5 fois à atteindre.
- **Preuve** : `data/factions/fac_england.json` (`victory`), `fac_poland.json` et voisins (`hold_turns` 8).
- **Gravité** : mineur à majeur (selon que « tous les objectifs » est conjonctif, d'après `status.md`, donc le trésor n'est qu'une contrainte parmi d'autres ; à vérifier dans `core/`).
- **Correctif** : vérifier la sémantique conjonctive et, si le trésor est le facteur limitant, le remplacer par un objectif de prestige ou de revenu soutenu ; ajouter une métrique « tour de victoire » aux seed bots.
- **Effort** : S

## 8. La difficulté ne module que des chiffres, pas des comportements
- **Problème** : les niveaux de difficulté ajustent revenus, entretien, morale, bruit de décision et agressivité de l'IA (`ai_income_percent` 80 en facile, etc.). Rien ne change la lisibilité ni l'intelligence : la difficulté « difficile » sera surtout une IA plus riche, ce qui accentue un déséquilibre déjà large (IA à 4 scalaires de personnalité, lus à 3 endroits seulement).
- **Preuve** : `data/rules/difficulty.json` ; `data/factions/fac_england.json` (`ai_personality` à 4 scalaires) ; `refonte-systemes.md` § 4.
- **Gravité** : majeur.
- **Correctif** : lots F1 et F5 (personnalités composées, cibles par phase) avant de toucher aux niveaux ; ajouter à chaque niveau une dimension de comportement (tolérance aux traités, mémoire diplomatique, coordination d'armées).
- **Effort** : L

## 9. IA diplomatique « prudente » : la grande histoire dépend d'un scénario scripté
- **Problème** : `status.md` constate que l'IA est « volontairement prudente » et que la guerre de Cent Ans peut s'achever par une paix blanche ; l'équilibre France-Angleterre (54-76 % de guerre) repose donc sur des seuils réglés (le seuil de révolte 74 « poussait la guerre FR-EN au-delà de 75 % sur 4 graines sur 10 » selon `population.json`). Un couplage fragile : toucher l'ordre public déplace le scénario central.
- **Preuve** : `data/rules/population.json` (description) ; `docs/status.md` § Limites M5.
- **Gravité** : majeur.
- **Correctif** : exposer dans les seed bots une métrique « part de guerre FR-EN » avec bande 54-76 % comme garde-fou de non-régression avant tout lot de refonte ; lots F2/F3 (mémoire diplomatique, trahison) pour découpler.
- **Effort** : M

## 10. Combat : l'équilibre repose sur 7 000 impacts par bataille, et les règles d'élan sont neutres
- **Problème** : élan, plancher d'élan, immunité au choc et déclin de la charge sont codés mais neutres dans `data/rules/battle_impact.json`. La fatigue ne punit pas la mêlée (le repos récupère ≈ 2,7 fois plus vite qu'elle ne s'use). Donc la cavalerie n'a pas de rythme (charge, dégagement, recharge) ; la batterie de tests (Crécy [14,20), Poitiers [11,19)) fige l'état actuel.
- **Preuve** : `refonte-systemes.md` § 2 ; `data/rules/battle_impact.json`.
- **Gravité** : majeur.
- **Correctif** : C1 (budget d'impacts), C2 (bonus décroissant à intégrale égale), puis C5 et C3 ensemble pour que l'activation des règles d'élan reste neutre en moyenne.
- **Effort** : L

## 11. Mercenaires et faillites : le filet de sécurité n'est pas mesuré
- **Problème** : mercenaires à 150 % du coût et 175 % de solde, réserve régionale ; l'IA riche en embauche à partir de 4 000 livres de trésor. Pas de métrique de mercenaires ni de dette dans les seed bots, et la bande de faillites (0,75 à 163,75) ne contraint rien. Trésor négatif : −10 de moral, désertion 5 % après 2 saisons impayées. Un joueur criblé de dettes n'a pas de sortie douce (prêt, banquiers, vente de bâtiment) visible dans les données.
- **Preuve** : `data/rules/mercenaries.json`, `data/rules/economy.json` (`bankruptcy_morale_penalty` 10, `desertion_after_unpaid_seasons` 2).
- **Gravité** : mineur.
- **Correctif** : lot 0 du chantier économie (faillites, dépenses par poste) puis un mécanisme de crédit (emprunt aux marchands avec intérêt, lié à `bld_counting_house`).
- **Effort** : M

## 12. Technologie : 45 techs, 3 % de ralentissement par tech, effet peu perceptible
- **Problème** : 45 technologies pour 116 ans de jeu, avec un ralentissement de recherche de 3 % par tech possédée (`research_slowdown_percent_per_tech`), et seulement 8 effets de bâtiment qui produisent des points de recherche sur 90. Le joueur reçoit un nombre de techs plus limité par le rythme des bâtiments que par un choix : le « build » de recherche est quasi imposé (université 3 000 et 60 d'entretien).
- **Preuve** : `data/rules/economy.json` l.14-16 ; `ls data/technologies | wc -l` = 45 ; décompte des effets `research_points` = 8.
- **Gravité** : mineur.
- **Correctif** : mesurer le nombre de techs obtenues par faction à 1453 dans les seed bots ; si < 15, relever la production ou baisser les coûts de paliers ; sinon accepter.
- **Effort** : S

## 13. Contrôle qualité du design : ADR et bruit statistique
- **Problème** : la note du plan reconnaît que « l'écart actuel est dans le bruit (±0,8) » avec 3 parties par bot. Toute décision d'équilibre prise sur 3-8 parties est invalide. Le `--quick` fait 8 parties ; la passe longue ≈ 12 minutes.
- **Preuve** : `refonte-systemes.md` § 1A ; rapport `--quick` (8 parties).
- **Gravité** : mineur.
- **Correctif** : imposer N ≥ 30 parties par comparaison A/B (avec graines appariées) et rapporter un intervalle de confiance ; automatiser la comparaison avant/après dans la passe longue.
- **Effort** : M

## Top 3 prioritaire
1. **Constat 1 (S)** : bandes de design réelles et métriques toujours sorties : sans elles, aucun des quatre chantiers de refonte ne peut être validé.
2. **Constat 9 (M)** : garde-fou « part de guerre FR-EN 54-76 % » avant de toucher à l'ordre public, à l'économie ou à l'IA (le scénario central y est couplé).
3. **Constats 2 et 3 (M puis L)** : jauge de tension cumulée puis armée rebelle : le système d'ordre public est le seul à connecter économie, population et guerre, et il n'offre aujourd'hui ni signal gradué ni jeu de réponse.

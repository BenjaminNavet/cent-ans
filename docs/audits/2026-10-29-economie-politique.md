# Audit de fond — économie, politique et féodalité (29/10/2026)

Auteur : audit Opus en lecture seule (boucle juge 2, itération 2). Aucun code ni donnée modifiés.
Périmètre : finances royales, recrutement, vassaux, succession et légitimité, diplomatie, révoltes, peste,
leurs interactions et leurs ordres de grandeur. État de référence : `main` à `f913f4f9b`.

## 0. Sources et méthode

- Lecture : ADR 0100, 0102, 0110, 0183, 0749, 0777, 0778, 0780, 0788, 0789, 0793 ; `core/crates/sim-campaign/src/`
  (`economy.rs`, `finance.rs`, `coinage.rs`, `ransom.rs`, `feudal.rs`, `feudal/`, `negotiation/`, `diplomacy/`,
  `perjury.rs`, `dynasty.rs`, `characters.rs`, `revolt.rs`, `population.rs`, `plague.rs`, `recruit_pool.rs`,
  `mercenaries.rs`, `replenish.rs`) ; `data/rules/{economy,finance,revolt,feudal}.json`, `data/ai/{diplomacy,feudal}.json`,
  `data/events/*`, `data/factions/*`. Quatre lectures parallèles (finances, féodalité et recrutement, diplomatie et
  succession, révoltes et peste) recoupées ; les affirmations décisives ont été revérifiées dans le code (citées `fichier:ligne`).
- **Passe longue des seed bots** (ADR 0348) : 32 parties de 150 tours, 8 bots, 837 s
  (`out/audit/eco/sb-long/report.{md,json}`).
- **Sonde de séries par tour** écrite pour cet audit (hors dépôt suivi : `out/audit/eco/probe/`, crate autonome qui
  dépend de `sim-campaign` et `ai` par chemin) : l'IA joue toutes les maisons, France comprise, sur **un siècle
  complet (464 tours, 1337-1453)**, graines 1, 2, 3, plus une partie où la France reste **passive** (graine 1).
  Chaque tour : trésor, recettes, chaque poste de dépense, impôts exceptionnels, emprunts, dette, provinces,
  population, agitation, hommes sous les armes, vassaux, monnaie, de 10 maisons suivies, et les événements.
  Séries brutes : `out/audit/eco/{s1,s2,s3,p1}.jsonl`, résumés décennaux : `out/audit/eco/{s2,s3,p1}.txt`,
  script : `out/audit/eco/analyse.py`.
- Pas de partie scriptée via le client ni de capture : la question est de règles, les séries du cœur suffisent.

## 1. Synthèse en une phrase

Le jeu modélise avec soin **une cour qui gère une trésorerie** (charges, crédit, monnaie, ponction de l'épargne),
mais la guerre de Cent Ans est l'histoire **d'un État qui naît de la guerre** : l'impôt consenti, l'armée sous contrat,
les princes qui choisissent leur roi et la légitimité disputée. Ces quatre moteurs manquent ou sont décoratifs ;
l'argent n'est donc jamais une décision politique, seulement un solde.

## 2. Mesures clés (sonde séculaire, IA aux commandes de la France sauf mention)

| Mesure (France) | Graine 1 | Graine 2 | Graine 3 | France passive |
|---|---:|---:|---:|---:|
| Part de l'armée dans les dépenses, siècle | **11,9 %** | 15,0 % | 16,0 % | 21,7 % |
| Part maison du roi + administration | **55,3 %** | 48,3 % | 49,6 % | 71,1 % |
| Part des impôts exceptionnels dans les recettes | **1,1 %** | 2,5 % | 0,7 % | 0 % |
| Impôts exceptionnels levés en un siècle | 6 | 15 | 5 | 0 |
| Saisons à trésor négatif | 0 | 0 | 0 | 12 |
| Banqueroutes royales | 0 | 0 | 0 | 0 |
| Hommes sous les armes : départ → maximum relevé (pas de 10 ans) | 620 → 2 045 | 620 → 2 101 | 620 → 2 072 | 620 → 712 |
| Provinces possédées 1337 → 1447 | 10 → 13 | 10 → 11 | 10 → 5 | 10 → 10 (occupées) |
| Recette brute par saison 1337 → 1447 | 36,3 k → 63,2 k | 36,3 k → 52,1 k | 36,3 k → 34,9 k | 26,6 k → **0,4 k** |
| Population tenue, 1347 → 1357 (peste) | −18 % | −19 % | −19 % | −16 % |
| Rançon du roi Jean (capturé) | — | 31 680 ₶ (0,9 saison) | 25 300 ₶ (0,7 saison) | — |
| Guerre France-Angleterre (part des tours) | 58 % | 51 % | 47 % | 67 % |

Événements par siècle, toutes maisons (167) : ≈ 1 050-1 160 paix et autant de déclarations de guerre, 1 130-1 500 mariages,
185-337 actes de vassalité, **0 à 2 révoltes** quand l'IA joue toutes les maisons (contre 0,5 à 21,75 par partie de 150 tours
pour la maison d'un bot, passe longue). Peste : 20-34 frappes, **toutes entre 1347 et 1359** (aucune récurrence).

Passe longue des seed bots (moyennes par partie de 150 tours) : provinces de la maison du bot en fin de partie
1,5 (aggressor) à 7,5 (mirror, l'IA du jeu) pour 10-11 au départ ; part de carte du plus grand acteur 6-7 % ;
26-30 maisons éliminées ; score de règne 6,75-18,75 pour un seuil de victoire de 50 ; banqueroutes 2,5 (builder) à 38 (turtle).
12 dérives signalées (aggressor, sailor, turtle, random-legal), aucune casse.

## 3. Diagnostic classé par gravité

### Critique

**C1. La guerre ne coûte presque rien et ne se finance pas : l'État fiscal de guerre est absent.**
- Preuve : armée = 12-16 % des dépenses de la France sur un siècle de guerre (58 % des tours en guerre contre l'Angleterre),
  maison du roi + administration = 48-55 % (tableau § 2). L'impôt exceptionnel pèse 0,7-2,5 % des recettes ; la France ne
  passe jamais une saison à trésor négatif et ne fait jamais défaut.
- Cause racine 1 : le revenu ordinaire (`economy.rs:250-286`, assiette par tête × richesse × 0,07) paie déjà la guerre ;
  les impôts exceptionnels (`finance.json` `levies`) ne rapportent que 0,5 à 0,9 saison, avec délai, soit au plus
  ≈ 25 % du brut en régime permanent (aide 0,5/6 + gabelle 0,9/12 + fouage 0,7/8), sans lien avec la guerre ni consentement.
- Cause racine 2 : l'IA ne lève et n'emprunte que « fauchée » ou si sa doctrine l'y pousse
  (`ai/src/strategy.rs:150-151`) ; ses armées restent petites (≤ 2 100 hommes) parce que la **réserve de recrutement**
  (`recruit_pool.rs`, 1-2 unités par place, 150-350 millièmes par saison) borne l'effort bien avant la bourse (ADR 0793 l'a dit).
- Histoire : de 1337 à 1360, la couronne de France vit de son domaine et d'expédients (mutations, décimes, emprunts) ;
  la guerre se paie par des **subsides demandés aux États** (1343, 1346-47, 1355-57) contre des concessions, puis par les
  **aides, la gabelle et le fouage** créés pour la rançon de Jean II (Compiègne, 1360 ; Amiens, 1363), qui dépassent vite le
  domaine de plusieurs fois. En Angleterre, le Parlement vote quinzièmes et dixièmes et la **maltôte de la laine** ; c'est le
  vote, et la crise du vote (Bon Parlement 1376, capitations 1377-1380, révolte de 1381), qui rythme la guerre.
  Le jeu inverse le rapport : l'ordinaire paie la guerre, l'extraordinaire est un bonus marginal.
- Conséquence de jeu : aucune décision « faut-il convoquer les États ? », « que leur céder ? », « quelle province pressurer ? ».
  L'argent n'est pas le facteur limitant (déjà noté par le juge 2, constat 3, et non corrigé par ECO 0793, qui a seulement
  baissé les montants).

**C2. Les grands feudataires ne sont pas des acteurs : l'ost ne fournit aucun soldat, l'apanage n'existe pas.**
- Preuve : `feudal.rs:925-958` (`summon_host`) — le vassal qui répond « déclare la guerre », aucune troupe n'est
  transférée ni levée ; le refus ouvre une félonie. Il n'existe ni ordre « convoquer l'ost » ni arrière-ban
  (l'ost ne part qu'à l'acceptation d'un appel de protection). Aucun mécanisme d'apanage
  (le mot n'apparaît que dans des textes). La fidélité (`diplomacy/upkeep.rs:132-182`) dérive de 5 points par saison vers
  une cible d'attitude ; la révolte d'un vassal exige < 20. Changement de suzerain possible (`feudal/acts.rs:49-107`)
  mais rare (150 ‰ et fidélité < 30, `data/ai/feudal.json`).
- Mesure : la France commence avec 7 vassaux directs ; elle en garde 3 (graine 1), 0 (graine 2), 1 (graine 3) en 1447,
  sans qu'aucun de ces départs ne passe par un choix lisible du joueur.
- Histoire : la guerre est un jeu à trois et plus. Bretagne (succession 1341-1364), Navarre (Charles le Mauvais),
  Flandre (Artevelde, puis l'héritage bourguignon de 1384), Bourgogne (apanage de 1363 devenu puissance rivale),
  Foix, Armagnac : chacun vend son ralliement à l'un ou l'autre roi. L'apanage est le grand pari des Valois.
- Bon point à garder : la commise de 1337 émerge bien du système (graine 1 : « Félonie d'Angleterre envers France
  (asile donné à un banni) : la commise peut être prononcée », puis Guyenne confisquée) — l'architecture des titres est
  saine, c'est le contenu politique qui manque.

**C3. L'argent sans créancier politique : chaque levier est un bouton à délai, aucun n'a d'interlocuteur.**
- Preuve : levées (`finance.rs:434-520`) : grogne uniforme sur toutes les provinces, délai fixe ; édit « aide féodale »
  (`data/edicts/edict_feudal_aid.json:9`) : +20 % d'impôt permanent pour +6 d'agitation, sans coût, sans date ;
  tranche d'impôt « haute » +40 % pour +19,5 d'agitation (`population.rs:249`) ; mutations (`coinage.rs`) rentables deux ans
  puis spirale. Les drapeaux politiques posés par les événements (`grande_ordonnance_acceptee`,
  `guyenne_en_pleine_souverainete`) **n'ont aucun consommateur mécanique** (seul un test les lit,
  `sim-campaign/tests/campaign_life/ev_history.rs`).
- Cause racine : le modèle est celui d'un tableur (paramètres × délai × grogne) ; il manque l'acteur qui accorde
  l'argent et demande quelque chose en échange (États, villes, clergé, Parlement, banquiers nommés).
- Conséquence : les décisions sont des optimisations locales réversibles, sans irréversibilité ni arbitrage entre
  partis ; le conseiller (ADR 0789) ne peut dire que « le trésor fond ».

**C4. La légitimité, cœur de la guerre dynastique, n'existe pas.**
- Preuve : aucune valeur de légitimité dans le code ; la prétention d'Édouard III est une donnée statique
  (`data/factions/fac_england.json` `claims`), son seul effet en paix est un malus de −45
  (`negotiation/context.rs:163-184`, `pretender_reluctance`) tant que le score de guerre est > −40.
  La loi salique de la France (`dynasty.rs:1368-1390`) ne fait que choisir l'héritier. Troyes (`evt_troyes.json`)
  donne une paix et une prétention, pas de double monarchie ; la régence (`dynasty.rs:1054-1130`) n'a ni régent,
  ni conseil, ni choix ; la folie de Charles VI n'ajoute qu'un trait (+10 agitation, −15 prestige).
- Conséquence : la question « qui est le roi de France ? » ne se pose jamais en jeu ; la paix est bloquée par un nombre,
  pas par un enjeu (constat 4 du juge 2 : score de paix −79 au tour 1, sans chemin visible).

### Majeur

**M1. La peste noire est une saignée unique et sans suite économique.**
- Preuve : une seule vague (`evt_black_death.json`, 1347-1350), 15-30 % de pertes uniformes (`plague.rs:60-64`), aucune
  récurrence (sonde : frappes uniquement 1347-1359 dans les 4 parties). Pertes mesurées sur la population tenue :
  −16 à −19 % en dix ans (histoire : 30 à 50 %, puis rechutes 1361-62, 1369, 1374-75). Aucun salaire, aucune pénurie
  de main-d'œuvre : le recrutement ne dépend pas de la population (`recruit_pool.rs:70-100`), le revenu en dépend
  linéairement. Les options de l'événement ne changent pas la mortalité.
- Conséquence : la peste n'ouvre aucune décision (ordonnances sur les salaires de 1351, Statut des travailleurs) et ne
  nourrit pas les révoltes de 1358 et 1381, qui sont ses enfants.

**M2. Les révoltes historiques sont des tableaux datés, le moteur de révolte tourne à vide.**
- Preuve : Jacquerie, Étienne Marcel, Maillotins, Ciompi, révolte de 1381 sont des événements à date fixe et effets fixes
  (`data/events/evt_jacquerie.json` : 1358, seule condition « la France tient l'Île-de-France ») ; aucun ne lève l'armée
  rebelle d'ADR 0731. Quand l'IA joue toutes les maisons, le moteur ne produit que **0 à 2 révoltes par siècle**
  (sonde, 167 maisons) ; il ne s'emballe que pour un joueur qui conquiert (aggressor : 21,75 par 150 tours, dérive).
  Armée rebelle plafonnée à 6 régiments de 120 hommes quelle que soit la province (`revolt.json` `rebel_levy`).
- Cause racine : l'agitation n'a pas d'entrée « fiscale et politique » assez forte (impôt consenti refusé, rançon,
  défaite, peste) et l'occupation (+24, `population.rs`) en est le moteur dominant : la révolte punit la conquête,
  pas la politique intérieure.

**M3. Rançons et butin à une échelle sans rapport avec l'économie chevaleresque.**
- Preuve : la rançon du roi Jean vaut 25 300-31 680 ₶ en 4 annuités, soit 0,7-0,9 saison de recette de la France ;
  celle du Prince Noir 8 000 ₶, de David II 1 065 ₶ (sonde). Plafond général : une saison de revenu du payeur
  (`economy.json` `ransom.income_cap_percent` 100). Le texte de Brétigny dit « trois millions d'écus » et la donnée
  donne 40 000 (`evt_bretigny.json`), et la libération par événement contourne plafond et contrôle de fonds
  (`effects.rs:697-698`).
- Histoire : la rançon de Jean II (3 millions d'écus) a créé l'impôt permanent ; les rançons de Poitiers ont enrichi
  une génération de capitaines anglais. Ici elles ne changent rien.

**M4. Le recrutement est une file de magasin, pas une mobilisation.**
- Preuve : trois sources (réserve par colonie, compagnies de mercenaires, rencontres) ; ni semonce ni arrière-ban,
  ni **lettres de retenue** (contrat à terme), ni démobilisation. Armée maximale 40 unités (2 400-4 800 hommes) ;
  armée de départ française 620 hommes ; pic relevé 2 100 (Crécy : 20 000-30 000 Français, 12 000-15 000 Anglais).
  Les Grandes Compagnies (`enc_grandes_compagnies.json`, dès 1357) ne naissent pas de la paix ni des soldes impayées.
- Conséquence : la paix de Brétigny ne crée aucun problème de soldats licenciés ; la guerre n'a pas de saison
  (« l'ost de 40 jours », la campagne d'été, l'hivernage).

**M5. Diplomatie : beaucoup de bruit, peu d'enjeu, et quatre défauts prouvés.**
- Bruit : ≈ 1 100 paix, 1 100 guerres et 1 100-1 500 mariages par siècle (sonde) ; 344-371 propositions de l'IA refusées
  par partie de 150 tours (passe longue).
- Défauts :
  1. « Trêve de deux ans » (`game-api/src/diplomacy.rs:597`) vaut `TRUCE_TURNS` = 20 saisons, **cinq ans**
     (le libellé du cœur, `negotiation/label.rs:32`, dit bien 5 ans).
  2. `perjury.rs:41-45` (`truce_age`) suppose toute trêve longue de 20 saisons : une trêve de médiation de 8 saisons a
     déjà « 12 saisons » le jour de sa signature ; sa rupture n'est jamais la parjure majeure (−25).
  3. Or : `check.rs:49-55` contrôle chaque article `Gold` contre le trésor isolément, pas leur somme ; le tribut
     (`check.rs:39-47`) n'a aucun contrôle de fonds et se prélève ensuite sans plancher. Plafond de valeur de l'or
     (`livres_per_point` 250, `max_gold_points` 60) : empiler des articles plafonnés est la voie la moins chère vers un « oui »
     déterministe du joueur (ADR 0182).
  4. `evt_auld_alliance` (1371) ne crée aucune alliance (pas d'effet « alliance » dans `EventEffect`).
- Trêves génériques : Esplechin, Malestroit, Bordeaux, Leulinghen ont la même durée ; la papauté d'Avignon, grande
  médiatrice des trêves de 1340-1360, est absente de la diplomatie de l'IA (`negotiable()` l'ignore).

**M6. La laine et la Flandre, nerf économique du conflit, sont absentes.**
- Preuve : `res_wool` n'apparaît qu'en marchandise d'affichage des routes (`trade.rs:76, 170`) ; le commerce plafonne
  vers 1,8 k par saison (< 10 % de l'impôt). L'Angleterre ne tire rien de sa laine ; la Flandre ne dépend pas d'elle.
- Histoire : l'embargo de 1336 sur la laine, la révolte d'Artevelde et l'alliance flamande de 1340 sont la première
  manœuvre de la guerre ; la maltôte finance Crécy.

### Mineur

- **m1.** Dévastation comptée deux fois dans le revenu (`economy.rs:284` et `population.rs:224`).
- **m2.** Passivité : la France passive garde ses 10 provinces de droit mais sa recette tombe de 26,6 k à 0,4 k
  (occupation) sans que ni défaut ni crise ne se produise ; l'Angleterre passive de la graine 1 tient à −225 k de trésor
  et 89 k de dette en 1437 (5 défauts) : la dette n'a pas de fond.
- **m3.** Mutations monétaires : l'IA ne les utilise qu'en trésor négatif (35-49 mutations par siècle, toutes maisons) ;
  pas de lien avec la rançon ni avec la « monnaie forte » de 1360 (franc à cheval).
- **m4.** Documentation périmée : ADR 0100 (« 2 saisons », seuil 90) et ADR 0102 (« pas de dilution ») contredits
  par le code ; valeurs par défaut du code (`tax_efficiency` 0,082, opulence 6 saisons) contredites par les JSON.
- **m5.** `ObligationKind::Tribute` sans producteur (`finance.rs:34-48`) ; dettes de rançon non retranchées de l'épargne
  dormante (`economy.rs:693`).
- **m6.** Score de règne des bots 6,75-18,75 pour un seuil de 50 (ADR 0793) : tant que C1-C4 ne donnent pas de leviers
  politiques, le score ne peut se gagner que par la conquête, que le moteur de révolte punit.

## 4. Les systèmes produisent-ils des décisions ?

| Système | Décision offerte aujourd'hui | Ce qui la rend creuse |
|---|---|---|
| Impôt par province, édits | tranche basse/normale/haute, édits | continue, réversible, sans interlocuteur ; l'aide féodale domine |
| Impôts exceptionnels | bouton + délai | rapport recette/grogne fixe, aucun lien à la guerre |
| Crédit | emprunter, rembourser | utile seulement en crise ; l'IA s'en sert peu ; la dette n'a pas de fond (m2) |
| Monnaie | affaiblir/renforcer | rentable deux ans puis spirale ; personne ne proteste nommément |
| Vassaux | exiger l'hommage, commise | l'ost ne donne pas de troupes ; pas d'apanage à accorder |
| Diplomatie | traités d'articles | l'or plafonné achète tout ; trêves sans histoire |
| Succession | désigner l'héritier | aucune légitimité à défendre |
| Révoltes / peste | événements datés | effets fixes ; le moteur ne réagit pas à la politique |

Les interactions qui feraient le sel de l'époque sont toutes coupées : **guerre → besoin d'argent → États → concessions →
pouvoir des villes → révolte** ; **défaite → rançon → impôt permanent → armée permanente** ; **paix → licenciement →
compagnies → dévastation** ; **peste → manque de bras → salaires → révoltes** ; **légitimité → ralliement des princes →
provinces**.

## 5. Principes tirés du genre et de l'histoire (sans copier un jeu)

1. **Chaque livre a un créancier politique.** Une recette se demande à quelqu'un (États, villes, clergé, banquiers,
   vassaux) qui demande quelque chose en retour ; c'est ce prix, pas le montant, qui fait la décision.
2. **Peu de grandes décisions plutôt que beaucoup de curseurs.** Une convocation d'États tous les quelques ans, avec des
   conséquences durables, vaut mieux qu'un bouton toutes les six saisons.
3. **La guerre crée son financement et ses crises.** Elle doit coûter plus que l'ordinaire ne rapporte, se payer par
   des expédients politiques, et laisser des traces (soldats licenciés, rançons, dettes).
4. **Les puissants sont des acteurs à agenda, pas des modificateurs.** Un prince choisit son roi selon la légitimité,
   l'intérêt et la peur ; le joueur achète, ménage ou brise.
5. **Les crises historiques émergent des causes.** Les dates historiques deviennent des fenêtres et des seuils
   (ADR 0779 l'a fait pour les batailles) ; la Jacquerie arrive parce que la noblesse a été battue et l'impôt levé.
6. **Ordres de grandeur relatifs, pas absolus.** La livre du jeu n'est pas la livre tournois ; ce qui doit être juste,
   ce sont les rapports : guerre ≫ domaine ; rançon royale ≈ plusieurs années de recettes ordinaires ;
   peste ≈ un tiers ; armée de bataille ≈ 5-10 armées de départ.

## 6. Refontes proposées

### R0 — Correctifs et mesure (prérequis, petit)
- **Vision** : réparer les défauts prouvés et donner aux seed bots les métriques économiques qui manquent, avant toute refonte.
- **Contenu** : libellé et durée des trêves ; `truce_age` qui lit la durée réelle ; contrôle cumulé de l'or et du tribut ;
  effet d'événement « alliance » (Auld Alliance) ; sonde de séries économiques versionnée ; métriques seed bots
  (part armée, part extraordinaire, hommes sous les armes, rançons, provinces contrôlées vs possédées).
- **Coût** : 1 lot Sonnet (≈ ½ journée d'agent). **Risques** : faibles ; bandes rapides à réviser.
- **ADR touchés** : 0182 (acceptation déterministe : l'or reste additif mais borné par les fonds), 0721 (parjure), 0789 (bandes).

### R1 — « L'impôt consenti » : États, Parlement, subsides de guerre (clé de voûte)
- **Vision** : séparer **l'ordinaire** (domaine, droits, tranche d'impôt : paie la cour et la paix) de
  **l'extraordinaire consenti** (subsides de guerre : paie la guerre). On ne lève plus un impôt, on **convoque une
  assemblée** — États de langue d'oïl et de langue d'oc, Parlement anglais, états de Flandre ou de Bretagne pour les
  grands fiefs — qui accorde un subside proportionné à la **nécessité** (guerre défensive, invasion, rançon du roi,
  campagne offensive moins bien vue) et exige des **concessions** tirées d'un catalogue en données : contrôle des
  dépenses (élus), monnaie forte, renvoi d'un conseiller, fin d'une guerre, privilèges urbains, remise d'impôt
  provinciale. Une assemblée a une **humeur** (confiance) qui se souvient des promesses tenues ou trahies ; refus,
  subside rogné ou exigences lourdes (« Grande Ordonnance ») sont possibles. Aides, gabelle et fouage deviennent des
  **impôts que l'assemblée peut rendre permanents** (1360-1363) ; Charles V peut s'en passer de la consulter s'il a
  la légitimité, au risque des révoltes de 1382 si un successeur les rétablit.
- **Angleterre** : le Parlement vote quinzième et dixième ; la **maltôte de la laine** est un subside qui dépend des
  exportations vers la Flandre (lien avec M6 : embargo, étape de la laine).
- **Ordres de grandeur cibles** : en guerre, subsides = 1,5 à 4 fois l'ordinaire ; armée = 50-70 % des dépenses ;
  en paix, la cour reste le premier poste. Ordinaire de la France revu à la baisse (≈ −30 %) pour que la guerre ne
  se paie pas seule.
- **Coût** : 4 lots (≈ 3-4 jours d'agent, dont un Opus pour l'équilibre). **Risques** : équilibre complet à refaire
  (bandes de seed bots, `starting_fit`, tests à graine `f7_events`, `ev_history`) ; IA à adapter (quand convoquer,
  quoi concéder) ; charge d'interface (une fenêtre d'assemblée).
- **ADR touchés** : 0777 (impôts exceptionnels remplacés), 0749/0788 (ponction d'épargne réduite à un filet),
  0793 (montants), 0274 et 0780 (interface), 0754 (causes), 0789 (conseiller).

### R2 — « L'ost et la retenue » : mobiliser, payer, licencier
- **Vision** : l'armée devient un **contrat à terme**. Trois manières de lever : **semonce des vassaux** (le vassal
  fournit un contingent réel, gratuit 40 jours = 1 saison, puis à solde ; refus = félonie), **arrière-ban**
  (levée de masse paysanne et urbaine, médiocre, grogne forte, réservée à l'invasion), **lettres de retenue**
  (capitaines engagés pour N saisons à solde fixe, avance d'un quart à la signature). À l'échéance, le contingent
  rentre ou se réengage ; **une compagnie licenciée sans solde devient routière** (Grandes Compagnies émergentes après
  une paix, à la place de la rencontre datée de 1357). La réserve de recrutement des colonies reste pour les
  garnisons et les milices.
- **Ordres de grandeur cibles** : une armée royale de bataille = 8 000-20 000 hommes (plusieurs armées réunies),
  levée pour une campagne (2-3 saisons), coûteuse ; la plupart du temps, garnisons et chevauchées de 2 000-5 000.
- **Coût** : 3 lots (≈ 3 jours d'agent). **Risques** : performances de bataille (taille d'armée, ADR 0760), IA militaire
  (ADR 0301) à adapter ; interaction avec l'attrition d'ADR 0793 ; bandes `battles_per_game`.
- **ADR touchés** : 0102 (réserves), 0110/0114 (ost féodal), 0793 (réserve), mercenaires (`mercenaries.json`).

### R3 — « Princes et apanages » : la politique des grands fiefs
- **Vision** : 6-8 grands feudataires (Bourgogne, Bretagne, Navarre, Flandre, Foix, Armagnac, puis apanages) deviennent
  des **acteurs à agenda** : objectif dynastique en données (Bretagne : succession ; Navarre : couronne ; Flandre :
  laine et privilèges), opinion de chaque roi, et un **ralliement** (France / Angleterre / neutre) recalculé chaque
  année selon légitimité (R4), intérêt, peur et argent. Le joueur peut **accorder un apanage** (fidélité immédiate,
  puissance future du prince), arbitrer une succession, acheter un ralliement (pension), confisquer.
- **Coût** : 3 lots. **Risques** : IA féodale (ADR 0110) à réécrire en partie ; narration à alimenter.
- **ADR touchés** : 0110, 0114, 0307, 0720 (rancunes), 0779 (histoire conditionnée).

### R4 — « Légitimité et couronne »
- **Vision** : une **légitimité** par prétendant à la couronne de France (et d'Angleterre après 1399), alimentée par
  sacre (Reims), victoires, reconnaissance des princes et du pape, tenue de Paris, argument salique ; elle règle
  l'acceptation des subsides (R1), le ralliement des princes (R3), la résistance des villes, et la paix
  (« renoncer à la couronne » devient un article qui vaut de la légitimité, pas un −45 fixe). Troyes peut créer une
  double monarchie jouable ; régence avec régent et conseil (Charles VI, Henri VI).
- **Coût** : 3-4 lots. **Risques** : touche victoire et score de règne (ADR 0785/0788), acte III de l'arc (ADR 0784).
- **ADR touchés** : 0784, 0785, 0788, 0334 (héritier), 0779.

### R5 — « Crises qui naissent des causes » : peste, travail, révoltes
- **Vision** : peste en vagues récurrentes tirées (1347, puis risque décennal décroissant), mortalité 25-45 % ;
  **salaire** par province (pénurie de bras → coût des unités paysannes et de la retenue, revenu agricole en baisse,
  ordonnance de 1351 jouable) ; révoltes à **déclencheurs causés** : impôt consenti refusé ou trahi, rançon, défaite
  de la noblesse, peste récente, capitation ; armée rebelle proportionnée à la population (plafond relevé) ; les
  événements historiques (Jacquerie, Marcel, 1381, Maillotins) deviennent les **habillages** du moteur dans leur
  fenêtre de dates.
- **Coût** : 2-3 lots. **Risques** : bandes de révoltes (déjà en dérive pour les conquérants) ; lisibilité.
- **ADR touchés** : 0100, 0731, 0779.

### (Hors liste) Laine et Flandre
Intégré à R1 (maltôte) et R3 (agenda flamand) plutôt qu'en refonte propre ; si R1 est différé, un lot isolé
« étape de la laine » (recette anglaise liée aux exportations, embargo flamand) reste possible.

## 7. Ordre recommandé

1. **R0** (prérequis, mesures et défauts).
2. **R1** (clé de voûte : sans subsides consentis, R2 n'a pas d'argent à dépenser et R5 n'a pas de cause fiscale).
3. **R2** (l'argent de R1 devient des armées à terme ; la paix crée les compagnies).
4. **R5** (les crises se branchent sur R1 et R2 : refus d'impôt, compagnies, peste et salaires).
5. **R4** puis **R3** (légitimité d'abord : elle est l'entrée de l'agenda des princes).

R1 et R2 peuvent se recouvrir d'un lot (R2a ne dépend que de R0). Chaque refonte commence par son lot de mesure et se
termine par une passe longue des seed bots en arrière-plan.

## 8. Specs de lots implémentables

Conventions : règles dans `core/crates/sim-campaign`, données dans `data/` avec schéma `data/schemas/`, aucune valeur en
dur ; ADR `docs/decisions/XXXX-<lot>-titre.md` numérotée à la fusion ; `cargo fmt`, `cargo clippy -- -D warnings`,
`cargo test --workspace` ; passe rapide seed bots en fin de lot, passe longue en arrière-plan à la fin de R1 et R2.
« Sonde séculaire » = l'outil de R0-a, 3 graines × 464 tours, IA aux commandes de la France, plus 1 partie passive.

### R0 — Correctifs et mesure (1 lot, Sonnet)

**R0-a Sonde et métriques**
- Fichiers : nouvel exemple `core/crates/ai/examples/eco_series.rs` (reprendre `out/audit/eco/probe/src/main.rs`,
  sortie JSON lines, options `--seeds`, `--turns`, `--passive`, `--factions`) ; `core/crates/seed-bots/src/metrics.rs`
  (+ `METRIC_NAMES`) ; `data/balance/seed_bots_bands*.json`.
- Nouvelles métriques (maison du bot) : `army_spend_share`, `extraordinary_income_share`, `men_under_arms_peak`,
  `ransom_paid_max_seasons` (plus grosse rançon payée / recette brute), `provinces_controlled_end`
  (à côté de `provinces_held_end`, qui compte la possession de droit).
- Tests : `metrics.rs` — une partie de 20 tours remplit les cinq métriques ; la sonde tourne 4 tours en test
  (`#[ignore]` hors CI si trop lent).
- Acceptation : passe rapide 0 constat ; les bandes nouvelles sont initialisées sur la mesure actuelle (état « avant »
  de R1/R2 consigné dans `docs/wip/`).

**R0-b Défauts de diplomatie**
- `game-api/src/diplomacy.rs:597` : libellé tiré de la durée (« Trêve de 5 ans ») via `negotiation/label.rs` ; durée en
  données (`data/ai/diplomacy.json` `negotiation.truce_turns`).
- `FactionState` : mémoriser le début de trêve (`truce_started: BTreeMap<FactionId, u32>`, `serde(default)`) ;
  `perjury::truce_age` le lit (repli sur l'ancienne règle pour une vieille sauvegarde).
- `negotiation/check.rs` : somme des `Gold` et premier versement de `Tribute` contrôlés contre le trésor du donneur ;
  tribut suspendu (pas de prélèvement négatif) si le trésor ne couvre pas, avec événement et rancune.
- `data-model` `EventEffect::Alliance { a, b }` et `evt_auld_alliance.json` l'utilise.
- Tests : `mediation_truce_broken_at_once_is_major_perjury`, `stacked_gold_cannot_exceed_treasury`,
  `tribute_never_drives_treasury_negative`, `auld_alliance_event_creates_alliance`, libellé de trêve.
- Acceptation : tests verts ; `ai_declined_proposals` dans sa bande ; aucune autre bande déplacée.

### R1 — L'impôt consenti (4 lots ; R1-a et R1-d Sonnet, R1-b et R1-c Opus)

**R1-a Squelette : assemblées en données et en état (Sonnet)**
- Données : `data/rules/assemblies.json` + `data/schemas/assemblies.schema.json` : assemblées
  (`asm_etats_oil`, `asm_etats_oc`, `asm_parliament`, `asm_etats_bretagne`, `asm_membres_flandre`), maison, provinces
  couvertes, ordres représentés (clergé, noblesse, villes, comtés), humeur de départ, délai minimal entre deux
  convocations (≥ 4 saisons), **nécessités** (`defensive_war`, `invasion`, `ransom`, `offensive_war`, `crusade`) avec
  multiplicateur de subside, **catalogue de concessions** (id, coût en données : agitation, prestige, part de l'ordinaire,
  contrainte d'état, durée).
- État : `CampaignState::assemblies` (`serde(default)`) : humeur 0-100, promesses en cours (concession, échéance),
  subsides en cours (montant par saison, saisons restantes), impôts rendus permanents.
- Ordres : `ConveneAssembly { assembly, necessity, ask }` (`ask` = palier de demande en données),
  `AnswerAssembly { session, accept_terms }`. Refus du cœur expliqués (`AssemblyError`).
- Tests : chargement et références croisées (`data-model`), sauvegarde aller-retour, ordre refusé hors délai.
- Acceptation : compile, aucune règle active (le fichier sans assemblée = comportement actuel).

**R1-b Session d'assemblée : vote, subside, concessions (Opus)**
- `sim-campaign/src/assembly.rs` : à la convocation, l'assemblée calcule un **accord** = humeur + légitimité
  provisoire (prestige du souverain jusqu'à R4) + nécessité − poids des subsides en cours − agitation moyenne de
  ses provinces − promesses trahies ; elle répond la saison suivante par **accord**, **accord sous conditions**
  (1-3 concessions tirées par coût croissant) ou **refus**. Le subside est versé **par saison** pendant N saisons
  (perception, pas un forfait), réparti sur les provinces couvertes (grogne locale proportionnelle à la part
  prélevée, à la place de la grogne uniforme d'aujourd'hui).
- Concessions : contraintes d'état vérifiées par le cœur (ex. « monnaie forte » interdit `SetCoinage` affaibli pendant
  N saisons ; « élus » plafonne la part maison + administration ; « fin de guerre » exige une trêve proposée avant N
  saisons). Trahir une promesse : humeur −30, événement, agitation des villes ; le drapeau
  `grande_ordonnance_acceptee` devient une concession réelle.
- Remplace l'ordre `LevyExtraordinary` : aide, gabelle et fouage deviennent des **formes de subside** (assiette
  différente : ventes/villes, sel/tous, feux/campagnes) et, par concession « rendre permanent », des impôts
  ordinaires (gabelle 1360, fouage 1363) ; Charles V et ses successeurs peuvent les lever sans session, au prix
  d'un risque de révolte urbaine (branché sur ADR 0731 ; R5 l'affinera).
- Ordinaire rééquilibré (`economy.json`) : `tax_efficiency` et paliers de maison réglés pour que la France en paix
  dégage un net de 10-20 % de son brut, et qu'une armée de campagne (≥ 3 000 hommes) ne tienne pas plus de
  2 saisons sans subside.
- Journal des causes (ADR 0754) : `assembly_refused`, `promise_broken`, `subsidy_granted`.
- Tests : `defensive_war_subsidy_granted_by_trusting_estates`, `offensive_war_with_low_mood_is_refused`,
  `broken_strong_money_promise_angers_estates`, `subsidy_paid_per_season_over_its_term`,
  `permanent_gabelle_needs_the_concession`, `no_assembly_file_keeps_old_levies` (repli).
- Acceptation (sonde séculaire, IA France) : part de l'extraordinaire dans les recettes **25-60 %** sur le siècle
  (aujourd'hui 0,7-2,5 %) ; part de l'armée dans les dépenses **35-60 %** (aujourd'hui 12-16 %) ; **au moins une
  session refusée ou à concessions lourdes par 40 ans** pour la France ; France passive : recettes ordinaires seules,
  aucun subside, déficit si attaquée.

**R1-c IA et Angleterre (Opus)**
- `ai/src/campaign.rs`, `finance::ai_finance_orders` → `assembly::ai_assembly_orders` : convoque quand la prévision de
  guerre dépasse l'ordinaire sur 4 saisons ; accepte une concession si son coût (en données) est inférieur au manque à
  gagner ; tient ses promesses selon un trait (`ai_personality` honneur).
- Parlement anglais : quinzième et dixième (forme `lay_subsidy`), **maltôte** (`wool_custom`) dont la recette est
  `exportations de laine × taux` : exportations = somme des provinces anglaises productrices de `res_wool`, coupées par
  embargo ou guerre avec la Flandre (route de `economy/trade.json`), et revenu de la Flandre lié aux mêmes flux
  (± 20 % du revenu flamand). Capitation (`poll_tax`, dès 1377) disponible : forte recette, risque de révolte paysanne.
- Tests : `english_wool_custom_falls_under_flemish_embargo`, `ai_convenes_before_a_campaign`,
  `ai_keeps_promises_when_honourable`.
- Acceptation : bandes `bankruptcies_per_game` mirror ≤ 8, `fr_en_war_share` 0,45-0,76 (garde-fou ADR 0731) ;
  Angleterre IA : maltôte ≥ 30 % de ses subsides en guerre ; aucune maison ≤ 2 provinces ne convoque d'assemblée
  (comtés exclus par les données).

**R1-d Interface et textes (Sonnet)**
- Client : la fenêtre « Finances » (ADR 0780) gagne un onglet « États et Parlement » : humeur, nécessités recevables,
  paliers de demande, concessions exigées, promesses en cours ; décision par boutons nommés
  (`faction:asm:convene:<id>`), dump `assembly_panel`.
- Données : `data/ui/assembly_panel.json` (textes français), encyclopédie (`data/codex`) : États de 1355-1357,
  Grande Ordonnance, Parlement et maltôte.
- Conseiller (ADR 0789) : déclencheurs `war_without_subsidy`, `promise_due`.
- Tests : `cargo test --features dev` (dump), `CENT_ANS_MUTE=1 client/tests/smoke.sh` ; 1 capture de contrôle au plus,
  par la session principale.
- Acceptation : les trois ordres jouables au clavier ; textes sans abréviation ; aucune règle côté client.

### R2 — L'ost et la retenue (3 lots ; R2-a Sonnet, R2-b et R2-c Opus)

**R2-a Semonce féodale : l'ost donne des soldats (Sonnet, ne dépend que de R0)**
- `feudal.rs::summon_host` : un vassal qui répond **lève un contingent** (armée sous son drapeau, rattachée à l'ost
  du suzerain) proportionné à sa recette (`data/rules/feudal.json` `host.contingent_percent_of_income`,
  `host.unit_mix`) pour `host.free_seasons` (1) saisons, puis rentre ou passe à solde payée par le suzerain.
  Nouvel ordre `SummonHost { enemy }` (joueur et IA), délai en données.
- Tests : `answering_vassal_raises_a_contingent`, `contingent_goes_home_after_its_term`,
  `refusal_still_opens_felony`.
- Acceptation : sonde séculaire : hommes sous les armes de la France au pic ≥ 2 × aujourd'hui (≥ 4 000) en guerre ;
  `battles_per_game` dans sa bande.

**R2-b Lettres de retenue (Opus)**
- `sim-campaign/src/retinue.rs` (existant, à étendre) : ordre `Indenture { captain, units, seasons }` : avance d'un quart,
  solde par saison prélevée en priorité sur les subsides (R1), **échéance** ; à l'échéance, `Renew` ou licenciement.
  Les unités de retenue n'utilisent pas la réserve des colonies (qui reste pour garnisons et milices) mais un vivier de
  capitaines par région (`data/rules/retinue.json` : taille, regarnissage, coût).
- Arrière-ban : ordre `ArriereBan` (invasion seulement : ennemi dans une province de la maison), milices nombreuses et
  médiocres, 1 saison, grogne paysanne forte.
- Tests : `indenture_ends_and_units_leave`, `arriere_ban_needs_an_invasion`, `retinue_paid_from_subsidy_first`.
- Acceptation : sonde : armée royale française la plus grosse 8 000-20 000 hommes au moins une fois par guerre longue
  (≥ 5 ans) ; part de l'armée dans les dépenses en guerre 50-70 %.

**R2-c Compagnies nées de la paix et des arriérés (Opus)**
- Quand une retenue est licenciée sans solde soldée, ou à la paix (unités de retenue renvoyées en masse), une part
  (`mercenaries.json` `discharge.free_company_percent`) devient une **compagnie routière** (faction `fac_routiers`
  ou bandes de `mercenaries.rs` existantes) qui pille la région, rançonne (`patis`) et se loue ; la rencontre datée
  `enc_grandes_compagnies` devient sa forme narrative (fenêtre 1357-1395 conservée en données).
- Tests : `peace_discharge_spawns_free_company`, `paid_discharge_spawns_nothing`.
- Acceptation : sonde : au moins une compagnie en France dans les 8 ans qui suivent une paix FR-EN longue ;
  `rebel_takeovers_per_game` dans sa bande ; aucune compagnie sans paix ni arriérés.

## 9. Points ouverts

- La livre du jeu est une unité de jeu : ce rapport ne propose pas de convertir en livres tournois, seulement de rendre
  justes les rapports (§ 5.6). Les chiffres historiques cités sont des ordres de grandeur de l'historiographie courante.
- R1 rend probablement inutile la ponction d'épargne (ADR 0749/0788) : à mesurer avant de la retirer.
- La sonde de cet audit (`out/audit/eco/probe/`) n'est pas versionnée ; R0-a la reprend dans le dépôt.

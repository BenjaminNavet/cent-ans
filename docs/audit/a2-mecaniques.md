# Audit A2 — mécaniques de jeu et équilibre

Date : 2026-09-24 (session de nuit 7). Auditeur : A2. Audit en lecture seule (aucun fichier de `core/`, `data/` ou `game/` modifié), fondé sur des parties IA contre IA et sur la lecture des règles.
Code audité : `main` à `70741c1`, avec le mouvement actuel sur le graphe des colonies (C7a), avant la refonte « mouvement libre » (M2). Le mouvement n'est pas audité en détail.

## 0. Synthèse

Le moteur tourne vite et proprement : 0,2 % d'ordres refusés, 29 factions vivantes au bout de 50 ans, aucune panique. Mais la campagne IA contre IA montre une **boucle stratégique creuse**. Les chiffres (8 graines × 200 tours, plus 4 × 464 tours) :

1. **Une seule unité.** La milice urbaine représente 99,8 % des recrutements de l'IA (2 440 par partie, contre 3,5 arbalétriers et 1,4 sergent à cheval). Aucun chevalier, archer long, homme d'armes ni engin de siège n'est recruté en 50 ans. Deux causes se cumulent : l'auto-résolution n'a ni contres ni armure en mêlée, et l'IA classe les unités par « puissance par livre ».
2. **L'auto-résolution contredit la bataille 3D.** À coût égal, 20 milices battent 4 chevaliers dans 100 % des auto-résolutions, mais perdent 5 fois sur 6 en 3D (1 498 morts contre 17). Crécy rejouée tourne à la victoire française dans 82 % des cas, et même dans 100 % sur terrain plat.
3. **Une carte figée.** Il y a 213 prises de places par partie, mais seulement 12 changements de propriétaire de province en 50 ans, et aucune faction ne disparaît. Les guerres sont nombreuses et courtes : 117 guerres par partie, pour une durée médiane de 13 tours. Elles se soldent presque toujours par un retour au statu quo.
4. **Une gestion sans tension.** Le mécontentement moyen est de 0,5 sur 100, avec 1 révolte par partie et une dévastation moyenne de 0. L'impôt « Haut » est un choix gratuit : il est en vigueur dans 63 % des royaumes en 1387. La chevauchée, mécanique signature, n'est lancée que 0,9 fois par partie.
5. **Une recherche trop rapide.** La France possède les 45 technologies dès 1383-1387, imprimerie comprise (1450 historiquement). Côté bâtiments, 10 des 29 ne sont jamais construits : tous les bâtiments militaires, plus la cathédrale, le pressoir et la maison de fonte d'étain.
6. **Des asymétries non voulues.** L'Angleterre aligne environ 1 000 hommes en campagne pendant des décennies, et la France finit 2 à 3,7 fois plus puissante. La Bourgogne jouable plafonne à 3 provinces et 3 000 à 4 000 livres de revenu : elle reste vassale dans 8 parties sur 8 et ne remplit jamais un objectif. Parmi les événements historiques datés avant 1387, 8 ne se déclenchent jamais, dont Poitiers, Brétigny et Étienne Marcel.

Priorités proposées (§ 5) : refondre l'auto-résolution et la calibrer sur la 3D (N1), donner à l'IA des doctrines de recrutement (E1), rendre l'ordre public et l'impôt coûteux (E3), ralentir la recherche (E4), faire bouger la carte par des buts de guerre et des cessions (N2), puis recrutement sur réserve d'hommes et renforts (N3).

## 1. Méthode

- **Sonde** écrite hors dépôt, dans le scratchpad de la session : une crate `a2probe` qui dépend par chemin de `core/crates/{ai,sim-campaign,sim-battle,data-model}`. Elle comporte trois binaires :
  - `a2probe <tours> <graines…>` joue des parties complètes. `ai::plan_turn` pilote toutes les factions, y compris la France, qui a le statut de « joueur » : elle répond aux offres comme l'IA (via `diplomacy::evaluate`), comme dans `century_probe`, et prend l'option 0 des décisions de chronique. Les batailles sont auto-résolues (`interactive_battles = false`). La sonde journalise chaque ordre de l'IA (via une enveloppe du planificateur) et chaque événement, prend un instantané de toutes les factions tous les 20 tours et fait le diff des bâtiments et des propriétaires à chaque tour.
  - `matrix` : matrice d'auto-résolution (`sim_campaign::resolve_auto`), à budget égal, à entretien égal et à effectif égal, sur 400 tirages par case.
  - `rt` : les mêmes affrontements en bataille 3D (`sim_battle::BattleSim`, IA contre IA), sur 6 graines.
- **Parties** : 8 graines (1-8) × 200 tours (printemps 1337 → printemps 1387), puis 4 graines (11-14) × 464 tours (1337 → 1453). Il faut environ 17 s par partie de 200 tours et environ 100 s par siècle, sur une machine chargée (charge moyenne 18).
- **Limites** :
  - le « joueur » est joué par l'IA, et il n'y a pas de bataille 3D dans la boucle ;
  - le mouvement est l'ancien (graphe de colonies) ;
  - les événements aléatoires des factions IA n'apparaissent pas au journal (`chronicle.rs` : « noise »), donc seuls ceux de la France sont comptés ;
  - « province capturée » compte les places prises (niveau colonie), tandis que le « changement de propriétaire » compte la province de jure.

## 2. Chiffres des simulations (8 graines × 200 tours, 1337-1387)

### 2.1 Trajectoire des factions jouables (médiane des 8 graines)

| Faction | Tour | Trésor | Revenu/saison | Entretien | Hommes en campagne | Hommes en garnison | Provinces | Techs | Puissance |
|---|---|---|---|---|---|---|---|---|---|
| France | 0 | 60 000 | — | — | 620 | 19 960 | 27 | 7 | 10 600 |
| | 40 (1347) | 67 486 | 45 046 | 41 322 | 8 780 | 19 290 | 26 | 18 | 18 876 |
| | 100 (1362) | 86 596 | 53 608 | 44 339 | 8 804 | 19 685 | 28 | 32 | 18 808 |
| | 200 (1387) | 278 872 | 93 595 | 83 510 | 23 038 | 23 002 | 26 | **45/45** | 33 956 |
| Angleterre | 0 | 45 000 | — | — | 540 | 17 660 | 23 | 8 | 9 370 |
| | 40 | 7 476 | 22 542 | 22 236 | **960** | 17 088 | 24 | 17 | 9 477 |
| | 100 | 17 812 | 21 446 | 20 999 | **922** | 16 414 | 23 | 26 | 9 438 |
| | 200 | 36 151 | 29 331 | 28 324 | 5 472 | 16 364 | 24 | 36 | 13 784 |
| Bourgogne | 0 | 15 000 | — | — | 300 | 2 360 | 3 | 4 | 1 480 |
| | 100 | 1 447 | 2 620 | 2 142 | 0 | 1 880 | 3 | 12 | 1 075 |
| | 200 | 3 408 | 3 868 | 3 328 | 462 | 1 560 | 3 | 18 | 1 337 |
| Écosse | 0 | 6 000 | — | — | 240 | 2 100 | 3 | 3 | 1 290 |
| | 60 → 100 | ≈ 0 | ≈ 650 | ≈ 550 | **0** | 0-210 | 3 | 8-10 | 0-105 |
| | 200 | 471 | 532 | 510 | 0 | 580 | 2 | 17 | 290 |

- Le revenu français est 2,0 à 4,2 fois celui de l'Angleterre en 1387 (médiane 2,7), et sa puissance militaire 1,9 à 3,7 fois. L'écart vient surtout de la croissance économique : les revenus doublent grâce aux bâtiments. Les batailles n'y sont pour presque rien, puisque les frontières bougent à peine.
- Les **garnisons** pèsent plus que les armées : environ 19 000 à 23 000 hommes pour la France et 16 000 à 17 000 pour l'Angleterre, stables sur 50 ans. L'Angleterre dépense en garnisons ce qui devrait financer ses chevauchées.
- **Trésors** : ceux de la France, de l'Empire et de la Castille valent 2 à 3 saisons de revenu en 1387 (règle F4 respectée). Les banqueroutes se concentrent sur la Suède (33 par partie), les Suisses (24), la Gueldre (11), Grenade (9) et la Navarre (6).
- **Bourgogne** : vassale de la France dans 8 graines sur 8, aux tours 0, 100 et 200. Aucun de ses objectifs n'est atteint : indépendance, Pays-Bas (1/7 province), Lorraine (2/4).

### 2.2 Fréquence des événements (moyenne par partie ; par tour)

| Événement | Par partie | Par tour | Commentaire |
|---|---|---|---|
| Bâtiment achevé | 716 | 3,6 | dominés par palissade, jardin de simples, moulins, foire, marché |
| Technologie | 579 | 2,9 | arbre épuisé vers 1383-1387 par la France |
| Agent (C6) | 447 | 2,2 | environ 2 340 actions d'agents par partie |
| Recrutement (événement) | 453 | 2,3 | environ 2 445 ordres `Recruit`, dont 99,8 % de milice |
| Bataille | 334 | 1,7 | environ 0,5 par tour en 1350-1362 (creux de la peste et des trêves), 2,5 à 2,7 en fin de période |
| Attrition | 238 | 1,2 | |
| Place prise | 213 | 1,1 | 12 changements de propriétaire de province seulement |
| Siège commencé / levé | 206 / 83 | 1,0 / 0,4 | assauts ordonnés : environ 50 par partie |
| Guerre déclarée / paix signée | 119 / 110 | 0,6 / 0,55 | durée médiane d'une guerre : 13 tours (3 ans), moyenne 16 |
| Banqueroute | 95 | 0,47 | 0,16 par faction et par décennie, sur 5 factions surtout |
| Peste | 12 | 0,06 | toutes en France (Peste noire comprise) |
| Hérésie | 9 | 0,05 | 0 province hérétique en 1387 |
| Révolte | **1,1** | 0,01 | toutes en Bohême |
| Chevauchée (`Raid`) | **0,2** | 0,00 | 0,9 ordre `SetStance Raid` par partie |
| Armée détruite | 7 | 0,04 | |
| Général capturé | 7 | 0,03 | 34 événements de rançon |
| Faction détruite | **0** | 0 | 29 factions sur 29 vivantes en 1387, dans toutes les graines |

Mécontentement moyen des provinces en 1387 : **0,5/100**. Dévastation moyenne : **0**.

### 2.3 Guerres, paix et boule de neige

- **Nouvelles guerres** : 117 paires en guerre par partie, soit 0,6 par tour pour 29 factions. Les paires les plus fréquentes sont France-Angleterre, France-Empire, France-Milan, Bohême-Empire, Bourgogne-Angleterre et Angleterre-Écosse (21 à 28 guerres sur 8 parties).
- **Guerre France-Angleterre** : 56, 46, 39, 39, 42, 38, 47 et 32 % des tours selon la graine (moyenne 42 %). C'est sous la cible de 55-75 % de `docs/status.md` (G5), ce qui est cohérent avec les creux de la Peste et des trêves des années 1350-1360 : la cible est définie sur le siècle (voir § 2.7).
- **Batailles** : l'attaquant gagne 56 % des batailles. La France gagne 72 % des siennes (1 101 batailles sur 8 parties), l'Empire 65 %, l'Angleterre 47 %, la Bohême 23 %, le Hainaut 9 %.
- **Carte figée** :

  | Graine | 1ʳᵉ faction (provinces 1387 / 1337) | 2ᵉ | 3ᵉ |
  |---|---|---|---|
  | 1 | France 26/27 | Angleterre 25/23 | Empire 19/20 |
  | 3 | Angleterre 24/23 | France 24/27 | Empire 15/20 |
  | 5 | France 30/27 | Angleterre 24/23 | Empire 13/20 |
  | 7 | France 29/27 | Empire 21/20 | Angleterre 21/23 |

  Il y a 12 changements de propriétaire de province en 50 ans (0,06 par tour). Les places prises sont rendues à la paix, et aucune faction ne meurt. **Pas de boule de neige, mais pas de mouvement non plus** : le joueur ne verra jamais une IA s'effondrer ni un rival émerger.

### 2.4 Recrutement, bâtiments, technologies

**Recrutement par l'IA (ordres par partie)** : 2 439,9 milices urbaines, 3,5 arbalétriers, 1,4 sergent à cheval. Rien d'autre : ni chevaliers, ni hommes d'armes, ni archers longs, ni archers montés, ni piquiers flamands, ni arbalétriers génois, ni engins de siège.

| Composition totale (moyenne) | 1337 | 1387 |
|---|---|---|
| Milice urbaine (campagne + garnison) | 557 | 2 503 |
| Arbalétriers | 259 | 185 |
| Hommes d'armes à pied | 163 | 95 |
| Chevaliers | 32 | 2,6 |
| Archers longs | 2 | 0,6 |
| Engins de siège | 0 | 0 |

Causes :
- `ai/src/campaign.rs::unit_value` (`soldats × (attaque + armure/2 + moral/4) / coût`) classe la milice à 26,0, les piquiers flamands à 24,0, l'archer long à 22,5, l'arbalétrier à 15,2, les hommes d'armes à 11,2 et le chevalier à 5,4. L'IA prend toujours le maximum. L'Angleterre ne recrute donc jamais d'archers longs, alors que 97 % des factions (dont elle) ont `tech_longbow_drill`.
- L'auto-résolution confirme ce choix (§ 3), si bien que le classement de l'IA n'est pas « faux » dans son modèle : c'est le modèle qui l'est.
- Le recrutement ne consomme pas de population : `recruit_blocker` ne fait qu'un contrôle de seuil (`classe × part ≥ 10 × soldats`). Les unités de campagne ne se renforcent jamais (`reinforce_garrison` vaut pour les garnisons seulement). Les régiments usés sont licenciés (483 `DisbandUnit` par partie) et remplacés par de la milice neuve.

**Bâtiments achevés (moyenne par partie)** : palissade 385, jardin de simples 320, moulin à eau 276, moulin à vent 239, foire 227, marché 227, maison de guilde 197, comptoir 169, apothicaire 123, scriptorium 110, château 78, université 54, atelier de tissage 46, église 38, adduction 12, hôtel-Dieu 12, abbaye 12, murailles de pierre 7,5, port 2,5.

**Jamais construits en 50 ans** (10 sur 29) :
- **militaires** : `bld_archery_butts`, `bld_armoury`, `bld_forge`, `bld_muster_field`, `bld_siege_workshop`, `bld_stables` ;
- **autres** : `bld_artillery_bastion`, `bld_cathedral`, `bld_tin_blowing_house`, `bld_vineyard_press`.

Cause : `building_value` (IA) ne valorise ni `production`, ni `recruit_slots`, ni `recruit_cost`, ni `army_*`, ni `piety`, ni `prestige`, ni `goods_satisfaction`. Ces bâtiments valent donc `−1,5 × entretien` et ne sont jamais choisis. La palissade (500 livres, +1 fortification, +1 garnison, valorisée ×20 en frontière) est le meilleur achat du jeu.

**Technologies** : la France possède les 45 technologies en 1387 (médiane), l'Angleterre 36, la Bourgogne 18. Le coût total de l'arbre est de 14 230 points. Parmi les technologies anachroniques en 1387 que possèdent des factions : imprimerie (1450) 6 %, compagnies d'ordonnance (1445) 10 %, artillerie de campagne (1450) 8 %, francs-archers (1448) 22 %, harnois blanc (1400) 26 %. La surtaxe d'anachronisme (+25 % au-delà de 20 ans d'avance) ne freine rien.

### 2.5 Chronique

- **Événements historiques** : 57 sont datés jusqu'en 1387. 36 se déclenchent dans toutes les graines, **8 jamais** :
  - `evt_poitiers` exige Jean le Bon roi à l'automne 1356. Or Philippe VI meurt au hasard après 1350 (taux doublé seulement une fois passée l'année historique, `characters.rs::natural_death_permille`), en plus de l'exigence d'une guerre France-Angleterre ce semestre-là ;
  - en cascade de Poitiers : `evt_bretigny`, `evt_etienne_marcel`, `evt_rancon_du_roi` ;
  - `evt_lords_appelants` (1387, hors fenêtre de 200 tours), `evt_rancon_de_nevers`, `evt_alliance_anglo_bourguignonne`, `evt_sacre_de_reims`.
- **Événements aléatoires** : 6 des 40 ne sont jamais vus dans le journal français (`argent_de_kutna_hora`, `harengs_de_scanie`, `piquiers_suisses`, `galeres_de_flandre`, `banque_florentine`, `razzia_frontiere`). Ils sont probablement réservés à d'autres factions (F7b), à vérifier dans une sonde qui compte aussi les tirages de l'IA.
- **Les plus fréquents côté France** (8 parties) : « Le don d'un marchand » 57, « La défection d'un capitaine » 52, « L'incendie de la ville » 48, « Les prêteurs lombards » 48. C'est environ un événement tous les 3 ou 4 tours pour le joueur, un rythme correct.

### 2.6 Personnages

- 228 personnages vivants en 1387.
- **Compétences apprises** : `skill_agronome` 74, `skill_bon_justicier` 47, `skill_intendant` 17, `skill_archerie` 9, `skill_discipline` 3, `skill_hardiesse` 0,6, `skill_beau_parleur` 0,1. L'IA ne développe presque pas la branche Commandement ni la branche Cour.
- **Traits** : bonne variété (40 traits vus). Les traits militaires restent rares : vétéran 5,6, maître de siège 0,9, brave 0,9.
- **Victoire** : la France atteint une victoire au tour 98 (1361) dans 1 graine sur 8. L'Angleterre n'a atteint aucun objectif dans aucune graine (Reims 0/2, royaume 4/15).

### 2.7 Siècle complet (4 graines × 464 tours, 1337-1453)

_(à compléter à la fin des simulations)_

## 3. Auto-résolution et unités

### 3.1 La formule

`battle_auto.rs` : puissance = `Σ soldats/100 × attaque × (1 + xp/10)`, multipliée par le moral (`0,5 + moral/200`), le ravitaillement, le commandement, le terrain (+15 % au défenseur), le passage de rivière (−20 %) et les murailles (−30 %). Seul le **tir** est réduit par l'armure ennemie moyenne (`× (1 − armure/200)`).

Ce que la formule ignore :
- l'armure contre la mêlée ;
- la charge (`stats.charge`), le type d'unité (cavalerie, piques), la portée, les munitions, la météo ;
- la répartition des pertes : elles sont une **fraction uniforme** (15-40 % au perdant, 5-15 % au vainqueur) qui frappe autant les chevaliers en harnois que la milice.

### 3.2 Matrice à budget égal (9 000 livres ; % de victoires de l'attaquant, ligne contre colonne)

| Attaquant \ défenseur | Arbal. | Piq. fl. | Génois | Cheval. | Arc long | H. d'armes | Arch. mont. | Serg. | Milice |
|---|---|---|---|---|---|---|---|---|---|
| Arbalétriers | 52 | 0 | 100 | 100 | 0 | 0 | 99 | 100 | 0 |
| Piquiers flamands | 100 | 52 | 100 | 100 | 93 | 100 | 100 | 100 | 71 |
| Génois | 0 | 0 | 52 | 100 | 0 | 0 | 25 | 28 | 0 |
| **Chevaliers** | 0 | 0 | 0 | 52 | **0** | 0 | 0 | 0 | **0** |
| Archers longs | 100 | 9 | 100 | 100 | 52 | 100 | 100 | 100 | 45 |
| Hommes d'armes | 100 | 0 | 100 | 100 | 0 | 52 | 100 | 100 | 0 |
| Archers montés | 1 | 0 | 76 | 100 | 0 | 0 | 52 | 94 | 0 |
| Sergents à cheval | 0 | 0 | 75 | 100 | 0 | 0 | 9 | 52 | 0 |
| **Milice urbaine** | 100 | 33 | 100 | **100** | 57 | 100 | 100 | 100 | 52 |

**Rendement** (puissance face à une armure 50) :

| Unité | Coût | Entretien | Puissance pour 1 000 livres | Pour 100 d'entretien | `unit_value` IA |
|---|---|---|---|---|---|
| Milice urbaine | 300 | 25 | **112** | **134** | **26,0** |
| Piquiers flamands | 450 | 40 | 117 | 132 | 24,0 |
| Archers longs | 500 | 45 | 98 | 109 | 22,5 |
| Arbalétriers | 550 | 50 | 54 | 60 | 15,2 |
| Hommes d'armes | 900 | 90 | 54 | 54 | 11,2 |
| Archers montés | 700 | 60 | 52 | 61 | 12,5 |
| Sergents à cheval | 800 | 80 | 43 | 43 | 9,6 |
| Génois | 900 | 100 | 41 | 37 | 10,8 |
| **Chevaliers** | 1 500 | 140 | **27** | **29** | **5,4** |

Le chevalier est quatre fois moins rentable que la milice. Le génois, mercenaire d'élite, est le pire tireur du jeu à budget égal.

### 3.3 Auto-résolution contre bataille 3D (`sim-battle`, IA contre IA, 6 graines)

| Affrontement | Auto-résolution (attaquant gagne) | Bataille 3D (attaquant gagne ; pertes A / D) |
|---|---|---|
| Crécy : 8 chevaliers + 4 génois + 4 hommes d'armes attaquent 8 archers longs + 4 hommes d'armes sur colline | **82 %** (100 % en plaine) | **6/6** ; 522 / 461 |
| 10 chevaliers attaquent 10 archers longs, plaine | 67 % | 6/6 ; 117 / 246 |
| 4 chevaliers attaquent 20 milices (6 000 livres chacun) | **0 %** (puissance 0,24) | **6/6** ; **20 / 1 057** |
| 20 milices attaquent 4 chevaliers | **100 %** (puissance 4,15) | **1/6** ; 1 498 / 17 |
| 10 chevaliers attaquent 10 piquiers flamands (Courtrai) | 0 % | 0/6 ; 288 / 66 |
| 12 milices attaquent 12 archers longs | 57 % | **0/6** ; 781 / **0** |
| 12 milices attaquent 12 milices derrière des murs | 0 % (puissance 0,70) | — |

Constats :
- **Les deux modèles divergent du tout au tout** sur la cavalerie lourde et la milice. Toute l'économie militaire de l'IA (et de tout joueur qui auto-résout) repose sur le modèle qui ignore l'armure en mêlée, alors que la bataille 3D la modélise correctement.
- **Crécy et Azincourt sont impossibles**, même en 3D sans pieux. La victoire anglaise historique suppose pieux, pente, boue et charge désordonnée. L'IA de bataille ne plante pas forcément les pieux (sonde `stakes` existante). Le relief (+15 %) ne suffit pas en auto-résolution.
- Seuls Courtrai (piques contre chevaliers) et les murailles sont cohérents entre les deux modèles.

## 4. Comparaison aux boucles de Total War

| Boucle | Total War (Medieval II, Attila, Three Kingdoms, Pharaoh) | Cent Ans aujourd'hui (preuve) | Verdict | Décision intéressante pour le joueur ? |
|---|---|---|---|---|
| **Économie** | revenu des bâtiments, commerce, impôt ; arbitrage bâtir / recruter ; faillite = désertion | riche (classes, biens, monnaie, rançons, régimes) ; revenu ×2,6 en 50 ans | profondeur OK, rythme trop généreux | faible : trésor France 279 000 livres en 1387, l'argent n'est plus une contrainte après 1360 |
| **Ordre public** | pilier (impôts, culture, religion, garnisons, édits) ; révoltes fréquentes | formule complète (`population.rs`) mais cible ≈ 0 : impôt haut +20 contre biens, santé et garnison −20 à −30 | **creux** | aucune : l'impôt « Haut » est gratuit (63 % des royaumes), 1 révolte par partie |
| **Croissance** | croissance de la population, paliers de colonie qui débloquent des bâtiments | croissance par classe, capacité, surpopulation | correcte | peu visible ; aucun palier de colonie lié à la population |
| **Recrutement** | local par bâtiments, bassins de recrutement qui se remplissent (Medieval II : unités par ville et temps de recharge) ; renforts en territoire ami | local, places par tour (G1), seuil de classe, **pas de consommation d'hommes, pas de renfort en campagne** | **creux** | aucune : la milice domine, les unités d'élite n'ont aucune raison d'exister hors 3D |
| **Entretien** | l'entretien plafonne la taille des armées ; plafond d'unités par armée (20) | entretien présent, garnisons à 50 % ; pas de plafond par armée | moyen | les garnisons (≈ 50 % des hommes) sont un coût fixe que l'IA ne remet pas en cause |
| **Attrition** | hiver, désert, territoire ennemi ; marche forcée | ravitaillement −20/−35 par saison hors de chez soi, famine à 0 ; 1,2 événement par tour | correct | faible : la M2 (mouvement libre) écarte l'attrition par case |
| **Diplomatie** | propositions, alliances, vassaux, commerce ; buts de guerre et fatigue de guerre (Three Kingdoms) | très riche (prétentions, cobelligérance, vassalité, mariages, papauté) ; 117 guerres courtes par partie | riche mais **sans conséquence territoriale** | moyenne : on fait et défait des guerres sans rien gagner ; les traités ne cèdent presque pas de provinces |
| **Personnages** | généraux, traits, suite, famille, loyauté, assassinat | compétences (3 branches), 40 traits, suite (C7), dynasties, rançons, ordres de chevalerie | au niveau, voire au-delà | l'IA ne développe que l'intendance ; la loyauté des généraux n'a pas d'effet de défection |
| **Agents** | espion, assassin, prêtre, marchand, diplomate | espion, héraut, prédicateur (C6) ; environ 2 340 actions par partie | correct, récent | à évaluer côté joueur (hors sonde) |
| **Sièges** | durée selon vivres, engins, assauts ; tours de siège construites sur place | vivres, brèche, assaut (IA à > 65 %) ; engins recrutables mais jamais recrutés | le moteur 3D est riche, la campagne l'ignore | faible : la place tombe par famine, les engins sont inutiles en campagne |
| **Chevauchée / pillage** | pillage de province (Attila, Thrones) | stance `Raid` : dévastation, butin, mécontentement | **mécanique signature quasi morte** (0,9 par partie) | aucune côté IA ; le joueur n'a pas d'adversaire qui la pratique |
| **Victoire** | conditions courte / longue ; crises de fin de partie (Attila, WH) | objectifs historiques par faction, maintien 12-20 tours | correct pour la France, inaccessible à l'Angleterre et à la Bourgogne jouées par l'IA | voir § 2.7 |
| **Boule de neige / fin de partie** | la boule de neige est le risque classique ; crises, corruption, coalitions | aucune faction ne meurt ni ne s'effondre en 50 ans ; revenu français ×2,6 | **stagnation** plutôt que boule de neige | la carte ne raconte pas d'histoire |
| **Technologie** | arbre long (Three Kingdoms : réformes sur toute la partie) | 45 technologies épuisées en 50 ans | trop court | disparaît après 1385 |

Ce qui manque le plus au regard de TW :
1. **Contres et identité des unités hors 3D.** C'est le cœur du plaisir TW, et il n'existe qu'en bataille 3D.
2. **Bassins de recrutement et renforts** (Medieval II, Attila). Ils donnent un vrai choix de quoi et où recruter.
3. **Ordre public qui mord.** C'est l'arbitrage impôt contre révolte, pivot des TW.
4. **Conséquences territoriales des guerres** : buts de guerre, cessions, fatigue de guerre.
5. **Plafond d'unités par armée et général obligatoire.** Aujourd'hui, 11 armées françaises sans chef coexistent au tour 20.

Les mécaniques propres à Cent Ans (régimes, monnaie, rançons, chevalerie, papauté, chronique) sont riches et doivent rester (voir `2026-09-24-analyse-total-war.md` § 4).

## 5. Lots proposés

Effort : S (< 1 jour-agent), M (1-3), L (> 3). Impact : 1 à 5.

Toutes les mesures de validation passent par une sonde de campagne comme `a2probe`, qu'il faut faire entrer dans le dépôt (lot O1). Cible à tenir : `century_probe` (France-Angleterre 55-75 %, 4 majeures vivantes en 1400).

### 5.1 Correctifs d'équilibre (données, réglages, heuristiques IA)

_(voir version finale)_

### 5.2 Nouvelles mécaniques

_(voir version finale)_

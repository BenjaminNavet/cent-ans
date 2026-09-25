# ADR 0056 — Batailles décisives : armée brisée, bataille refusée

Date : 2026-09-25. Statut : accepté. Lot EP9 (suivi : `docs/wip/ep9-batailles-decisives.md`),
recette Q3 point 12 (`docs/audit/q3-recette.md`).

## Contexte

Recette Q3 : aucune bataille de campagne ne s'est décidée d'elle-même. Joueur sans ordre : 0 perte
au bout de 7 min 14 s, les archers anglais sur leurs pieux attendent. Avec des ordres : pas finie à
7 min 23 s, retraite générale obligatoire.

Deux causes, mesurées sur 12 graines × 3 paliers (escarmouche 10 régiments par camp, grande bataille
24, bataille rangée 60 × 120 hommes) × plaine / collines / rivière × IA contre IA / joueur attaquant
immobile / joueur défenseur immobile (`sim-battle/tests/ep9_decisive.rs`, sonde `survey`) :

1. **La fin exigeait qu'il ne reste plus un seul régiment en ordre** chez le vaincu (tous en déroute,
   morts ou sortis). À l'échelle EP1, les derniers régiments se rallient loin de l'ennemi et
   reviennent : IA contre IA en bataille rangée, médiane 971 s, jusqu'au plafond de 30 min ; joueur
   immobile en grande bataille ou bataille rangée : non terminée à 30 min dans 95 % des cas.
2. **Personne n'était tenu d'attaquer.** Depuis R4 (ADR 0046) et EP3 (ADR 0033), le défenseur attend
   sur sa hauteur, derrière son couvert ou sa rivière ; l'attaquant IA attaquait bien (au plus 150 s
   d'attente sur une hauteur), mais un joueur attaquant immobile face à un défenseur IA laissait la
   bataille figée jusqu'à `DEFENDER_PATIENCE` (8 min), puis le défenseur sortait de sa position et
   mettait encore plusieurs minutes à user une armée immobile. Démo 1337 (le cas Q3) : 744 à 911 s.

## Ce que disent les sources

- **Une armée cède avant d'être détruite.** Les batailles rangées du XIVᵉ siècle se jouent en une
  à trois heures et se terminent quand une armée lâche pied, pas quand son dernier homme tombe :
  à Crécy (1346) l'armée française se disperse à la nuit après quinze à seize charges ; à Poitiers
  (1356) la dernière bataille du roi Jean est enveloppée quand les deux premières ont déjà reflué
  (Sumption, *The Hundred Years War*, t. I-II ; Rogers, *War Cruel and Sharp*, 2000). Le jeu
  compresse le temps (5 à 12 minutes) mais garde le principe : au-delà d'une certaine perte de
  substance, la déroute gagne toute l'armée.
- **La bataille refusée.** Buironfosse (octobre 1339) : Édouard III et Philippe VI rangent leurs
  armées face à face ; les Anglais à pied derrière leurs archers attendent l'attaque, le roi de
  France, conseillé de ne pas assaillir une position préparée, ne vient pas ; au soir les deux
  armées se retirent sans combat (Froissart ; Sumption, t. I, ch. VII). Refuser la bataille à
  l'ennemi qui l'offre sur son terrain est ensuite la politique constante de Charles V et de
  Du Guesclin. Celui qui cherchait la bataille (l'envahisseur) ne peut pas la forcer ; il se retire,
  l'autre garde le champ.

## Décision

Règles dans `data/rules/battle_decision.json` (schéma `battle_decision_rules.schema.json`), code dans
`sim-battle/src/decision.rs` et `src/sim/decision.rs`. Batailles rangées seulement : les sièges
gardent leur fin (place tenue, garnison défaite ; lot SG4 en parallèle).

1. **Armée brisée.** La « part en état de combattre » d'un camp = soldats des régiments présents et en
   ordre (ni en déroute, ni en retraite, ni sortis) plus ses réserves EP1, rapportés à l'effectif
   initial. Sous `break_share` (0,40 ; 0,50 une fois le général tué ou pris) pendant
   `break_hold_seconds` (30 s : un ralliement immédiat sauve l'armée), l'armée est brisée : ses
   régiments encore en ordre partent en déroute, la bataille se termine
   (`BattleEnd::Broken`, « L'armée de France est brisée : déroute générale ! »). Les deux camps sous
   le seuil : celui qui garde la plus grande part l'emporte.
2. **Bataille refusée.** Une horloge d'engagement repart à chaque mêlée, à chaque fois qu'un camp a
   perdu 1 % de son effectif (`engagement_loss_share` : quelques flèches perdues à travers une
   rivière ne comptent pas) et à chaque approche de 20 m en 30 s (`approach_meters`,
   `approach_window_seconds` : la marche d'approche, même lente à l'échelle épique, relance
   l'horloge ; le piétinement non). Au bout de `refusal_seconds` (300 s) sans engagement et sans
   mêlée jusque-là : l'attaquant renonce et se retire en bon ordre, le défenseur garde le champ
   (`BattleEnd::Refused`, « Bataille refusée »). Après une mêlée, une accalmie de `lull_seconds`
   (150 s) suffit : le camp qui a le moins souffert garde le champ (le défenseur, sauf si
   l'attaquant a perdu nettement moins, `lull_loss_margin` 5 points) — `BattleEnd::Lull`.
3. **Qui doit attaquer.** L'attaquant de la bataille est celui qui l'a engagée en campagne
   (l'armée qui s'est portée sur l'autre). IA : l'attaquant ne tient plus le duel d'archers que
   180 s (`ATTACKER_DUEL_LIMIT`, au lieu de 480 s), son attente sur une hauteur reste bornée à
   150 s (R4). Le défenseur IA garde sa position au-delà de `DEFENDER_PATIENCE` tant que personne
   ne combat (`DEFENDER_QUIET`, 60 s) : il laisse la bataille être refusée au lieu de sortir. Un
   joueur défenseur immobile riposte comme avant (mêlée au contact, tir à volonté).
4. **Conséquences en campagne.** `BattleOutcome::end` (`rout`, `broken`, `refused`, `lull`,
   `nightfall`, `square_held`) accompagne le résultat. Bataille refusée : le défenseur est vainqueur
   (compte de guerre, repli de l'attaquant comme après une défaite), mais sans perte, sans déroute
   (`routed = false`, `withdrew = true`) et avec un moral de campagne de −5 pour l'attaquant, +2
   pour le défenseur (`refused_morale`) au lieu de −20 / +5 ; la chronique dit « Bataille
   refusée : l'ost de France n'ose attaquer l'ost d'Angleterre sur ses positions et se retire ».
   L'accalmie retire aussi le vaincu en bon ordre (moral −20 / +5 comme une défaite).

## Mesures (durée min / médiane / max en secondes, 12 graines)

| Palier, terrain | IA-IA avant | IA-IA après | joueur attaquant immobile avant → après | joueur défenseur immobile avant → après |
|---|---|---|---|---|
| escarmouche, plaine | 420/486/627 | 301/342/406 | 542/618/653 → 530/542/564 | 529/617/668 → 348/384/408 |
| escarmouche, collines | 346/542/704 | 338/429/522 | 621/886/1048 → 300/300/547 | 624/690/1094 → 385/435/539 |
| escarmouche, rivière | 212/309/560 | 208/268/573 | 743/958/1800+ → 300/348/673 | 322/935/1800+ → 251/564/780 |
| grande, plaine | 429/581/969 | 320/354/420 | 1193/1800+/1800+ → 563/590/611 | 1800+ (12/12) → 351/364/381 |
| grande, collines | 386/549/1554 | 349/415/512 | 1337/1800+/1800+ → 300/300/330 | 1800+ (12/12) → 377/413/457 |
| grande, rivière | 111/332/811 | 107/264/669 | 988/1800+/1800+ → 300/523/775 | 544/1800+/1800+ → 304/517/740 |
| rangée, plaine | 493/971/1800+ | 239/371/561 | 1800+ (12/12) → 366/371/652 | 1800+ (12/12) → 385/395/414 |
| rangée, collines | 497/893/1785 | 345/443/499 | 1800+ (12/12) → 300/300/362 | 1800+ (12/12) → 406/435/467 |
| rangée, rivière | 428/780/1199 | 251/303/418 | 863/1800+/1800+ → 300/493/870 | 718/1800+/1800+ → 341/484/771 |

Vainqueurs : IA contre IA toujours partagés (attaquant 3 à 8 sur 12). Crécy-like (archers et hommes
d'armes anglais sur une crête contre chevaliers, arbalétriers et piétons français) : Anglais 11/12
avant et après, 5 min 30 au lieu de 5 min 40 - 8 min. Démo 1337 sans ordre (le cas Q3) : 529 à
665 s, victoire anglaise (armée française brisée ou accalmie) au lieu de 607 à 911 s.

Après fusion de main (ADR 0052, panique des chevaux sous les traits), mêmes ordres de grandeur :
IA-IA 107 à 669 s selon les paliers ; seules des graines avec rivière dépassent 12 minutes (jusqu'à
870 s, et une graine épique « défenseur immobile » à 1302 s). Crécy-like : Anglais 11/12. En jeu
(`q3_playtest.gd` de la recette, partie France, bataille sans aucun ordre) : bataille refusée à
326 s, écran de résultat « Défaite · Bataille refusée ».

## Conséquences

- Toutes les batailles rangées mesurées se terminent ; hors rivière, en 5 à 11 minutes. Avec une
  rivière à passer, quelques graines vont à 13-14 min 30 (passage de l'IA sous les flèches, EP3).
- Les batailles de référence changent : digests de `b6.rs` (mêmes vainqueurs, 267 et 294 s au lieu
  de 314 et 472 s) ; `ai.rs` : durée moyenne attendue 4 à 12 min (5 min en moyenne) ;
  `ep1_scale::ai_handles_sixty_regiments_a_side` : l'armée qui cède est brisée avant que toute la
  ligne soit engagée, 11-12 régiments en mêlée au plus (seuil 12 → 10, fin exigée avant 12 min) :
  le seuil de 20 d'avant R4 ne peut pas revenir.
- Écarts connus : à l'échelle rangée, l'attaquant IA perd souvent contre un défenseur immobile
  (archers tirant à volonté) ; au passage d'une rivière profonde, l'attaquant IA se noie et se brise
  souvent. Ce sont des limites de l'IA d'attaque (R4, EP3), pas de la fin de bataille.

## EP9b (2026-09-26) : duel de l'attaquant gagné, milice en second échelon

Constat (SG4, ADR 0046 § Suite SG4) : sur un terrain plat sans pieux, 60 régiments × 120 hommes par
camp, armées miroir, IA des deux côtés, le défenseur gagnait 10 batailles sur 10. À 180 s,
`ATTACKER_DUEL_LIMIT` envoyait la ligne attaquante en avant, milice (moral 40) en tête avec les
hommes d'armes ; elle traversait 180 m de flèches, se débandait, et la déroute des voisins (contagion)
faisait passer l'armée sous le seuil de 40 % : armée brisée vers 340 s.

Ce que disent les sources : on n'envoie pas les milices communales en tête d'un assaut. À Crécy
comme à Poitiers, les « batailles » françaises attaquent avec les hommes d'armes (à cheval puis à
pied) devant ; les piétons des communes suivent ou restent en arrière (Sumption, t. I-II ;
Contamine, *Guerre, État et société à la fin du Moyen Âge*, 1972). Et un camp dont les tireurs
gagnent l'échange n'a pas de raison de le rompre : c'est à celui qui le perd de se résoudre à
charger (règle déjà suivie par l'IA depuis B4, `DUEL_LOSS_MARGIN`).

Décision (`data/rules/battle_duel.json`, schéma `battle_duel_rules.schema.json`, `sim-battle/src/duel.rs`,
`DuelRules` gardées dans `BattleSim`, remplaçables par `set_duel_rules`) :

1. **Duel gagné, duel prolongé.** L'horloge d'engagement relève à chaque période d'IA la part de
   l'effectif de chaque camp abattue par les traits (`recent_missile_losses`, fenêtre glissante
   `window_seconds` 60 s). L'attaquant gagne le duel quand l'ennemi a perdu sur la fenêtre au moins
   `winning_min_share` (0,5 %) de son effectif et au moins `winning_ratio` (1,5) fois ce qu'il a
   perdu lui-même. Tant qu'il le gagne, il tient la ligne jusqu'à `winning_duel_max_seconds`
   (300 s) ; sinon (il perd ou fait jeu égal) il marche à l'ennemi après `duel_limit_seconds`
   (180 s, l'ancienne constante `ATTACKER_DUEL_LIMIT`, désormais en données). Seules les pertes par
   les traits comptent : une escarmouche de cavalerie ne décide pas du duel.
2. **Pas de bataille refusée par un duel gagné.** Un duel gagné coûte au défenseur au moins 0,5 % de
   son effectif par minute : l'horloge d'engagement (`engagement_loss_share` 1 %) repart au moins
   toutes les deux minutes, la règle des 300 s sans engagement ne peut pas jouer pendant ce duel —
   c'est la borne de 300 s (et les munitions : le duel cesse quand les tireurs sont à court) qui y
   met fin, pas la bataille refusée. Le test pytest vérifie `duel_limit ≤ winning_duel_max ≤
   duel_limit + refusal_seconds`.
3. **Milice en second échelon.** Quand la ligne marche à l'ennemi après le duel (ennemi entre
   `DUEL_RANGE` 320 m et `second_echelon_closes_m` 120 m), les régiments de ligne au moral de base
   (`morale_cap`) inférieur à `second_echelon_morale` (50 : milices) avancent
   `second_echelon_depth_m` (35 m) derrière les troupes solides ; la ligne est mesurée sur son
   premier échelon. À 120 m, le second échelon serre sur la première ligne et suit l'assaut dans
   la mêlée. Une ligne en défense, ou qui tient pendant le duel, reste sur un seul rang.

Mesures : voir le tableau ci-dessous (à compléter).

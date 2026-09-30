# 0128 — Plafond d'unités par armée et engins de siège construits sur place (lot NT5)

Date : 2026-09-29. Statut : accepté. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md`
(ligne NT5) ; audit `docs/audit/a2-mecaniques.md` lots N6 et N7.

## Contexte

- N6 : `max_army_units` (20) ne bornait que les ralliements des rencontres (CV3). Les fusions, la
  formation d'armée depuis une garnison et l'engagement de mercenaires faisaient des osts de
  masse (58 000 hommes après 1400 dans l'audit A2), alors que la bataille ne déploie que 20
  régiments par camp.
- N7 : l'assaut était permis dès le début d'un siège, contre des murailles intactes, avec un bélier
  offert à chaque bataille de siège ; aucun engin n'était recruté en un siècle.

## Décision

1. **Plafond en donnée** : `data/rules/armies.json` `max_units` (20, schéma
   `army_rules.schema.json`), `GameData::army_rules`. Le champ `encounters.json`
   `max_army_units` est supprimé (une seule source). Le recrutement lève toujours en garnison ;
   le plafond borne l'entrée dans une armée : `CreateArmy` et `MergeArmies`
   (`OrderError::ArmyFull`), mercenaires (`blocked` « armée complète »), rencontres `join`. Une
   armée déjà au-delà (ancienne sauvegarde, armées de départ) le garde mais ne reçoit plus rien.
   IA : garnisons excédentaires découpées en armées de 20 au plus, fusion seulement si la somme
   tient (sinon l'armée reste distincte et reçoit les suivantes), mercenaires bornés par la place
   libre.
2. **Engins construits, pas recrutés** : `data/rules/siege_engines.json`. `SiegeState.engine_work`
   cumule à chaque tour de progression du siège `hommes des assiégeants / men_per_work_point`
   (au moins `min_work_per_turn`), majoré de la vitesse de siège (général, traditions). Les engins
   s'achèvent l'un après l'autre dans l'ordre du fichier : échelles (4), bélier (16), beffroi (40).
   Une armée de 2 000 hommes (20 points par tour) a échelles et bélier en 1 tour, son beffroi en
   3 ; 400 hommes (plancher de 4) : 1, 5 et 15 tours. Pas d'ordre de construction : l'armée qui
   assiège construit (tout siège de campagne en a besoin, l'IA comprise, sans arbitrage).
3. **Assaut** : derrière des murailles debout (fortifiée, brèche < 50, sans tour de siège recrutée
   ni beffroi construit), l'assaut exige au moins un engin prêt (`assault_blocker`,
   `AssaultError::NoEngine`) ; il devient donc possible au 2e tour de siège, bien avant la famine.
   L'IA garde sa règle de rapport de force (`ASSAULT_ODDS`) et attend en plus un engin.
4. **Bataille** : `SiegeSetup.engines: Option<SiegeEngineSetup {ram, ladders, towers}>`. `None`
   (rejeux, batailles faites à la main) garde l'ancien comportement (bélier offert, échelles).
   Sans bélier construit, pas de bélier ; sans échelles, l'escalade n'est possible que depuis un
   beffroi accosté ; les beffrois construits sont des régiments synthétiques (pas de pertes
   reportées en campagne). En résolution automatique, un beffroi prêt vaut une tour de siège
   (murailles sans effet) ; un bélier prêt y majore les coups de l'assaillant de
   `auto_assault_bonus_percent` (20 %, porte enfoncée ; NT9) tant que les murailles tiennent.

## Conséquences

- Les assauts partent un tour plus tard qu'avant (le tour d'arrivée ne compte pas, cf. lot M2) ;
  les sièges longs d'une grosse armée finissent par un assaut sans malus de murailles.
- Mesures (`century_probe 464`, 6 graines, normale) : guerre FR–EN 67,6 % → 66,9 % (6/6 dans
  55-75 %), prises 1188 → 1116, sièges réussis 44 % → 40 % ; batailles FR/EN par décennie
  77,5 → 144,6 (plus d'osts distincts sous le plafond). Garde-fous `ep7_historical`,
  `ep9b_duel`, `ai_beats_a_passive_ai_at_equal_forces` inchangés.
- Les sièges de démonstration (`debug_stage_*`) ont échelles et bélier, comme avant.
- UI : panneau de siège (liste des engins, tours restants, bouton d'assaut grisé avec infobulle),
  « Former une armée » grisé avec infobulle au-delà de 20 unités cochées.
- Les tests qui donnaient l'assaut au tour même du siège avancent la construction d'un tour.

## Révision NT9 (2026-09-30, `docs/wip/nt9-equilibre.md`)

- **Bélier en résolution automatique** : `siege_engines.json` `auto_assault_bonus_percent`
  (bélier 20) ; `BattleContext.assault_bonus_percent` multiplie `walls_attacker` (0,7 → 0,84) ;
  `assault_odds` (IA, UI) et la prévision de bataille (« Porte enfoncée par le bélier ») le
  prennent aussi. Effet mesuré faible (sièges réussis 39 → 39 %, guerre FR–EN inchangée).
- **Batailles ×1,9 : cause** : sous le plafond, un ost est plusieurs armées ; chaque armée de
  l'IA recevait son propre ordre `Attack` sur l'ennemi le plus proche : plusieurs armées
  attaquaient tour à tour la même armée ennemie ou ses restes (batailles « même province, même
  saison » 17 % → 31 %). **Correction (IA)** : une seule attaque par armée ennemie et par tour
  (`GridPlanner::attack_order_sparing` : un ennemi à portée d'engagement d'une cible déjà
  attaquée ce tour est laissé). Restreindre la règle aux armées placées avec le premier
  assaillant a été mesuré sans effet (132,5/déc.) : les armées convergent de plusieurs lieux.
- **Chiffres** (`century_probe 464`, normale, graines 1-6 | 7-12) — batailles FR/EN / déc. :
  avant N6/N7 (f36689196) 77,5 | 60,1 ; main 129,1 | 138,5 ; NT9 106,4 | 124,7 (×1,68 sur 12
  graines, main ×1,94). Guerre FR–EN : avant 67,6 | 69 % (12/12 dans 55-75 %), main 68 | 69 %
  (11/12), NT9 64 | 64 % (10/12 : 51 % graine 6, Angleterre tournée vers une autre revendication ;
  76 % graine 12).
- **Excès restant, admis** : il ne vient plus d'un artefact (répétitions revenues à 15-17 %,
  batailles à sens unique « 0 contre n » 50 % → 36-40 %) mais du nombre d'armées : à effectif
  égal, le plafond de 20 unités donne environ deux fois plus d'osts, qui se rencontrent
  séparément, en batailles plus disputées. L'excès est tardif : batailles de terrain par tranche de
  40 ans (graines 1-6) avant 616 / 982 / 1193, NT9 692 / 1288 / 1717 ; sur 1337-1377, armées
  FR+EN de campagne 6,7 → 9,1 par tour pour 354 → 327 batailles (graines 1-3) : le plafond ne
  mord qu'une fois les royaumes riches, là où l'ost unique d'avant atteignait 58 000 hommes. Effet
  voulu du plafond (TW a beaucoup de batailles) ; le compteur compte en outre les lignes de
  journal de chaque bataille (victoire héroïque, avis « pendant le tour », rang de tradition).

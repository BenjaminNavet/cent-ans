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
   s'achèvent l'un après l'autre dans l'ordre du fichier : échelles (8), bélier (16), beffroi (40).
   Une armée de 2 000 hommes a ses échelles en 1 tour, son bélier au 2e, son beffroi au 4e ; 400
   hommes : 2, 6 et 16 tours. Pas d'ordre de construction : l'armée qui assiège construit
   (tout siège de campagne en a besoin, l'IA comprise, sans arbitrage).
3. **Assaut** : derrière des murailles debout (fortifiée, brèche < 50, sans tour de siège recrutée
   ni beffroi construit), l'assaut exige au moins un engin prêt (`assault_blocker`,
   `AssaultError::NoEngine`) ; il devient donc possible au 2e tour de siège, bien avant la famine.
   L'IA garde sa règle de rapport de force (`ASSAULT_ODDS`) et attend en plus un engin.
4. **Bataille** : `SiegeSetup.engines: Option<SiegeEngineSetup {ram, ladders, towers}>`. `None`
   (rejeux, batailles faites à la main) garde l'ancien comportement (bélier offert, échelles).
   Sans bélier construit, pas de bélier ; sans échelles, l'escalade n'est possible que depuis un
   beffroi accosté ; les beffrois construits sont des régiments synthétiques (pas de pertes
   reportées en campagne). En résolution automatique, un beffroi prêt vaut une tour de siège
   (murailles sans effet) ; le bélier n'y a pas d'effet propre.

## Conséquences

- Les assauts partent un tour plus tard qu'avant (le tour d'arrivée ne compte pas, cf. lot M2) ;
  les sièges longs d'une grosse armée finissent par un assaut sans malus de murailles. Mesures
  d'équilibre (part de guerre FR–EN, garde-fous bataille) dans `docs/wip/nt5-plafond-engins.md`.
- UI : panneau de siège (liste des engins, tours restants, bouton d'assaut grisé avec infobulle),
  « Former une armée » grisé avec infobulle au-delà de 20 unités cochées.
- Les tests qui donnaient l'assaut au tour même du siège avancent la construction d'un tour.

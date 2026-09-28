# 0102 — Reconstitution des armées et réserves de recrutement (lot TW2-T2)

Date : 2026-09-28. Statut : accepté. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T2.

## Contexte

Avant T2, une armée de campagne ne regagnait jamais d'hommes : seules les garnisons se
renforçaient (`economy.rs`, effet `garrison`) et les blessés d'une bataille étaient en partie
soignés sur-le-champ (`medicine.rs`, H4). Une armée entamée restait entamée jusqu'à sa fusion ou
sa dissolution. Le recrutement n'avait d'autre plafond que les créneaux par tour et le trésor :
une ville pouvait lever la même unité d'élite à chaque tour.

## Décision

1. **Modules neufs**, pour ne pas toucher `economy.rs` (lot RS B) : `sim-campaign/src/replenish.rs`
   (reconstitution) et `recruit_pool.rs` (réserves). Chiffres dans `data/rules/replenishment.json`
   (schéma `replenishment_rules.schema.json`) ; `ReplenishmentRules::default()` lit le fichier
   embarqué à la compilation (`include_str!`, comme les règles de `sim-battle`) : pas de copie des
   chiffres dans le code.
2. **Reconstitution saisonnière** (fin de tour, après l'attrition, avant la fin des marches
   forcées) : taux = base du territoire (propre 20 %, allié 10, neutre 5, ennemi 0) × posture
   (marche 100 %, campement retranché 150, siège 50, chevauchée 25, embuscade 75, marche forcée 0),
   ou × 150 % dans une place amie, × 50 % l'hiver, × 50 % sous 30 de vivres, puis + intendance du
   chef (3 % par niveau de gouvernement, compétences et traits listés) + bâtiments de la province
   sur ses propres terres (5 % par point `garrison`, 10 % par point `recruit_slots`, 50 % au plus) ;
   plafond 40 % des hommes manquants. Coût : 40 % du prix de recrutement par homme (× le
   pourcentage de difficulté de l'IA, comme le recrutement). Trésor vide : rien ; trésor court :
   ce qu'il paie, jamais en dessous de zéro. Une armée qui a combattu dans la saison
   (`Army.fought_turn`, posé par `movement::apply_outcome`, point commun de toutes les batailles)
   ne regagne rien. Les unités détruites sont retirées après leur bataille et ne reviennent pas.
   Pas de dilution de l'expérience (écart assumé à Total War, à revoir avec T5).
3. **Réserves de recrutement** : `SettlementState.recruit_pool` (millièmes d'unité) ne garde que
   les réserves non pleines ; absente = pleine, donc anciennes sauvegardes et départ 1337 démarrent
   pleins, sans changer `STATE_VERSION`. Plafond = base du type de place (cité 2, château 2,
   ville 1, abbaye 1, village 1) + 1 dans la cité capitale + 1 par point `recruit_slots` des
   bâtiments ; recharge/saison = base (cité 0,35, château 0,30, ville 0,25, village 0,20,
   abbaye 0,15) + 0,025 par niveau de fortification + 0,1 par point `recruit_slots` + 0,15 en
   capitale, × catégorie (cavalerie 60 %, siège 50 %). Une place assiégée ne se recharge pas.
   Le refus « réserve épuisée (+1 dans K saisons) » vit dans le bloqueur de recrutement commun :
   joueur et IA y sont soumis. La boucle de recrutement de l'IA (`ai/src/campaign.rs`) compte ce
   qu'elle a déjà tiré de chaque réserve dans le tour pour ne pas émettre d'ordres refusés.
4. **Pont et UI** : `get_army_replenishment(army)` → `{percent, men, missing, cost, territory,
   blocked, factors[], tooltip}` ; `get_recruitable` gagne `pool_available`, `pool_cap`,
   `pool_seasons_to_next`, `pool_label`. Le sceau de l'armée du joueur affiche « +N % » avec
   l'info-bulle des facteurs ; chaque ligne de recrutement affiche « N disponibles, +1 dans K
   saisons ».

## Effet sur l'IA (sonde `ai/examples/t2_replenish_probe.rs`, 20 tours, IA partout)

`off` = règles d'avant T2 (taux nuls, réserves sans fond). Hommes en armées de campagne
(moyenne sur les 21 relevés) :

RESULTS_TABLE

L'IA ne s'effondre pas : les effectifs moyens sont du même ordre ou supérieurs (les armées
entamées se reconstituent en terres propres), les trésors restent comparables.

## Conséquences

- Nouveaux champs d'état en `serde(default)` ; `STATE_VERSION` inchangé.
- T5 (traditions « intendance ») pourra ajouter un bonus de plus à la liste des facteurs.
- T3 (mercenaires) : les mercenaires pourront avoir leur propre réserve régionale, distincte.

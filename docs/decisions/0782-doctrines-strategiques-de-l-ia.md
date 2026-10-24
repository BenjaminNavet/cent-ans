# ADR 0782 — Doctrines stratégiques de l'IA de campagne (lot IA-DOCTRINES)

Date : 2026-10-24. Statut : accepté. Complète l'ADR 0754 (journal des causes) et les ADR 0348/0350 (seed bots).

## Contexte

Le juge game design (`out/juge/iter-02-design.md`, `iter-03-design.md`) : « l'IA est un planificateur à bonus et la
difficulté ne modifie que des chiffres ». Toutes les factions jouaient la même partition (seuils de
`data/ai/campaign.json`), seul le recrutement avait une doctrine (`data/ai/doctrines.json`, ADR E1). Le joueur ne
pouvait donc ni deviner les intentions d'un adversaire, ni voir une personnalité historique : Édouard III ne
chevauchait pas, Charles V ne temporisait pas.

## Décision

1. **Catalogue en données.** `data/ai/strategic_doctrines.json` (schéma `ai_strategic_doctrine.schema.json`, types
   `data_model::ai_strategic_doctrine`) : huit doctrines nommées en français. `chevauchee_pillage` (Édouard III, Prince
   Noir), `attrition_fortresses` (Charles V, Du Guesclin), `chivalric_battle` (Philippe VI, Jean II), `feudal_marriage`
   (Habsbourg, Luxembourg, Bourgogne, Brabant), `merchant_credit` (Flandre, Gênes, Venise, Florence),
   `crusade_frontier` (Ordre teutonique, livonien, Hospitaliers, croisés, Hongrie), `passive_defence` (Suisses, Byzance,
   Grenade, Raguse, Novgorod) et `balanced`, exactement neutre : les ~100 autres factions jouent comme avant.
2. **Cinq familles de leviers par doctrine**, chacune lue par le planificateur :
   - *objectifs* (conquête, revanche, sécurité, trésor, prestige, 0-100, 50 neutre) : pondèrent la valeur des cibles de
     siège (villes, reconquête, revanche, capitale ennemie et trône revendiqué, revenu) et des défenses ;
   - *bataille* : supériorité exigée avant d'attaquer une armée (`engage_ratio_percent`, grille), seuil de repli
     (`retreat_percent`), supériorité exigée avant un siège, chances d'assaut ;
   - *guerre* : agressivité, rapport de forces exigé avant de déclarer la guerre, chevauchée avant le siège (`raid_first`) ;
   - *diplomatie* : fréquence de recherche d'alliances (`plan_alliances`) et de mariages (deux fois l'an ou un an sur deux) ;
   - *finances* : un royaume peu enclin à emprunter ou à lever une aide extraordinaire ne le fait que trésor vide
     (filtre sur les ordres de `finance::ai_finance_orders`, lot ECO2 inchangé).
3. **Affectation en données** (`assignments`, première ligne qui convient) : faction, souverain (`ruler`), années,
   défaites subies (`min_defeats`, décompte permanent `battle_tally`). La France suit la chevalerie jusqu'à trois
   défaites ou jusqu'à Charles V, puis l'usure : « la doctrine change avec le souverain ou après une défaite ».
   Résolveur pur : `sim_campaign::doctrines::active`.
4. **Revanche.** `battle_tally` retient la province de la dernière victoire de chaque paire (`last_province`). Un rival
   qui a gagné plus de batailles sur le royaume qu'il n'en a perdu (`revenge_min_net_defeats`) devient cible de revanche :
   rabais sur le rapport de forces exigé pour lui déclarer la guerre, bonus sur ses places au siège, proportionnels au
   poids de revanche de la doctrine.
5. **Journal des causes.** `AiReason` gagne `doctrine` ; la guerre déclarée par l'IA consigne la raison nommée
   (`revenge_at` : « Revanche d'une défaite subie en {lieu} », `revenge`, ou les raisons existantes) et la doctrine suivie
   (« Doctrine suivie : Guerre d'usure et de forteresses. »). Les textes sont dans `data/ui/causes.fr.json` ; le nom de la
   doctrine vient du catalogue. Le lieu est la province de la défaite (granularité du `battle_tally`), pas le nom de la bataille.

## Conséquences

- Aucune règle codée en dur : valeurs et affectations en données ; `balanced` est neutre au bit près (tests de neutralité).
- Pas de changement de format de sauvegarde incompatible : `last_province` et `doctrine` sont `serde(default)`.
- Les équilibres de l'histoire simulée bougent pour les factions dotées d'une doctrine (voir `docs/wip/ia-doctrines.md`
  pour les mesures) ; les tests dépendant de la dynamique (`ev_history`, `f7_events`) sont rejoués.
- Limite assumée : la doctrine n'est consignée que pour les guerres déclarées (pas encore sièges ou alliances rompues), et
  le changement de doctrine lui-même n'est pas un événement du journal.

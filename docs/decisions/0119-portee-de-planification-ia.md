# ADR 0119 — Portée de planification de l'IA (index partagés en lecture seule)

Statut : remplacé par l'ADR 0205 (le registre global devient un objet `PlanCache` explicite).

## Contexte
Sur la carte Oural–Méditerranée (443 provinces, 177 factions, ≈ 3 900 colonies), la
planification IA d'un tour de jeu coûtait ≈ 1 s contre 0,23 s sur l'ancienne carte (OM I1).
Le profil montre les mêmes questions posées des centaines de fois à un état qui ne change pas
pendant qu'une faction planifie : puissance militaire d'une faction (O(armées + colonies)),
voisinage de deux factions (O(provinces)), revenu, rivaux, provinces contrôlées. Ces fonctions
vivent dans `sim-campaign` et prennent `&CampaignState` ; les appelants sont nombreux (≈ 60 pour
`faction_power`). Invalider un cache stocké dans l'état à chaque mutation (champs publics
modifiés partout) est hors de portée et fragile.

## Décision
- `CampaignState::planning_scope()` renvoie une garde qui **emprunte** l'état ; tant qu'elle vit,
  des index calculés une fois (puissance par faction, voisins de chaque faction ; mémos du revenu,
  des rivaux, des provinces contrôlées) sont enregistrés dans un registre global sous l'adresse
  de l'état. Les accesseurs les lisent s'il y en a un, sinon font le calcul direct (`*_walk`,
  gardés publics comme référence des tests).
- Validité : l'emprunt interdit toute mutation ou déplacement de l'état pendant la portée
  (l'état n'a aucune mutabilité intérieure) ; l'entrée est retirée quand la dernière garde
  tombe ; un clone vit à une autre adresse et ne reçoit jamais ces réponses. Les mémos qui
  dépendent de `GameData` sont indexés aussi par l'adresse des données.
- `ai::plan_turn_in` ouvre la portée pour toute la planification d'une faction (y compris les
  fils du pool rayon, qui lisent le registre global).
- Données : `MovementGraph::index()` (graphe des colonies en indices denses, ordre des id) et
  `GameData::nearest_settlement` (grille de colonies) sont construits paresseusement au premier
  usage ; les données ne changent pas après le chargement (la grille retombe sur le parcours si
  le nombre de colonies ou de positions a changé).
- Règle : chaque cache a un test d'égalité avec l'ancien calcul, et l'empreinte `turn_digest`
  reste identique à graine égale.

## Conséquences
- Planification ≈ ×0,43 en temps CPU (OMR R1, `docs/wip/omr-r1.md`), décisions inchangées.
- Une nouvelle question répétée peut être ajoutée à `Derived` (`planning_scope.rs`) avec son
  calcul `_walk` et un test d'égalité. Ne jamais y mettre une réponse qui dépend d'autre chose
  que l'état et les données (règles globales installées, horloge…).
- Hors portée (fin de tour, interface), les accesseurs gardent leur coût d'origine.

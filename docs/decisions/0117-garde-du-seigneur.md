# 0117 — Garde du seigneur : l'unité de la capitale entretenue par le domaine (lot OMR R3)

Date : 2026-09-29. Statut : accepté. Mesures : `docs/wip/omr-r3.md`. Suite de l'ADR 0114
(point 5, banqueroutes des petites factions) et des points ouverts de `docs/wip/fe.md`.

## Contexte

Carte OM (443 provinces, 177 factions) : `century_probe` 50 tours × 5 graines donne
5,25 banqueroutes par faction et par décennie, 6,0-6,5 pour les petites factions (≤ 2 provinces)
contre 0,03-0,23 pour les 28 factions d'avant FE. Une banqueroute est un tour de trésor négatif.

Inventaire au tour 0 des 135 factions d'au plus deux provinces (exemple `r3_small_scan`) :
52 ont un revenu inférieur à l'entretien de leurs bâtiments plus une seule unité de garnison.
Le revenu d'un comté pauvre (20 000-40 000 habitants, Perm, Alanie, Desmond, Pallars, Makki,
Isles) est de 40-80 livres par saison ; l'unité la moins chère (milice urbaine, 25 livres par mois)
coûte 45-50 livres par saison dans une cité (taux de garnison de la cité, 50 %, allégé par les
fortifications). Or l'IA ne congédie jamais la dernière unité de sa capitale (`disband_for_debt`,
une place vide se prend sans siège) : cette unité, à elle seule, met ces factions en déficit
perpétuel (Perm, trace `ECON_TRACE` : revenu 47, entretien 64 dont 45 pour l'unité, trésor
négatif de 1341 à la fin). Ce n'est pas une donnée à corriger province par province : le coût
d'une unité est absolu, le revenu d'un comté est proportionnel à sa population, et relever la
population ou la richesse de trente provinces de l'Est et d'Irlande pour payer une compagnie
fausserait l'histoire (populations estimées, relues par le lot R4) et le reste de l'économie ;
baisser le taux de garnison des cités changerait l'économie des royaumes (40 cités pour la France).

## Décision

`data/settlements/rules.json` § `capital_guard` : les `units` unités les moins chères de la
garnison de la cité capitale d'une faction (la cité de sa province capitale, quand elle la
tient) forment la maisonnée armée du seigneur, entretenue par son domaine ; le trésor n'en paie
que `upkeep_percent` %. Réglage : 1 unité, 0 %. Absent : comportement antérieur.

Le calcul vit dans `sim_campaign::economy::garrison_share` (entretien de la simulation) et
l'estimation de l'IA (`ai::campaign::garrison_upkeep`) l'appelle aussi. La prime des
mercenaires en garnison n'est pas concernée.

Même lot, sans changer de règle :

- **IA en dette** (`ai::campaign::plan_demolitions`, `sim_campaign::agents::plan_agents`) : la
  démolition se déclenche aussi quand la dette ne se rembourse pas en `DEBT_REPAYMENT_TURNS` (8)
  au surplus net, tribut au suzerain et agents compris (le compteur de déficit de la simulation
  les ignore : Tarente, Kildare restaient en dette des décennies sans raser) ; un royaume en dette
  congédie son agent le plus cher, un par saison.
- **Révoltes** : `rules/agents.json` `incite_unrest` 15 → 12. Avec 177 factions, les espions
  postés dans une cité ennemie la soulevaient chaque saison (395 ordres « Soulever » en 50 tours) ;
  sans incitation, 0,8 révolte / 200 tours en début de partie contre 24.

## Conséquences

- L'unité que l'IA garde toujours ne peut plus, seule, ruiner un comté ; les grands royaumes
  économisent 45-50 livres par saison (moins de 0,2 % du revenu de la France).
- Un joueur ne peut pas en abuser : une seule unité, la moins chère, dans une seule place.
- Sonde `century_probe` 464 tours × 10 graines (avant → après) : banqueroutes 3,98 → 0,69 par
  faction et par décennie, petites factions 4,47 → 0,74, 28 factions d'avant FE 0,07 → 0,09 ;
  révoltes 16,0 → 8,1 / 200 tours (bande 4-10 : 2/10 → 7/10 graines) ; guerre FR-EN 68 % → 68 %.
  Détail : `docs/wip/omr-r3.md`.

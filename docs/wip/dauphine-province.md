# Lot DAUPHINE — province du Dauphiné (ADR 0732)

## État
- Fait : données (prov_dauphine, fac_dauphine, tit_dauphine, Humbert II, lieux Grenoble/Vienne/Grande Chartreuse), prov_provence allégée,
  événement 1349 (transfer_title, la province étant la capitale de fac_dauphine), objectif France, tests g1/m10 adaptés,
  fx (horizon, biome, décor), id_map + selection.json, chaîne géométrique rejouée (216 provinces), neighbors recalés,
  `data/rules/starting_fit.json` régénéré (`CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p sim-campaign starting_fit`).
- Testé : data-model et sim-campaign (lib, tests) verts sauf le correctif ci-dessous, non revérifié.
- Non testé : correctif Vienne (cathédrale retirée, interdite en `town`) ; assertion trésorerie de
  `the_dauphine_purchase_takes_the_capital_province_from_humbert` (la trésorerie absorbée de fac_dauphine revient à la France) ;
  clippy, pytest (test_province_neighbors, comptes de provinces 215 -> 216 ?), seed-bots rapide, client (tests, dump, capture).

## Prochaine étape
Relancer : cargo test -p sim-campaign ; pytest tools ; clippy ; seed-bots --quick ; dump de campagne (événement 1349) ; éventuellement
données par faction (portraits archetypes, voice barks, art catalog) si un test les exige pour fac_dauphine.

## Procédure carte
Voir ADR 0732 (id_map -> chaîne `geo provinces` … `cross_fixture`, `data/map/real/` copié du checkout principal, jamais cent-ans-raw).

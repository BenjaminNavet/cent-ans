# 0329 — Sièges : tours de muraille en données, contre-batterie, second point d'assaut

Statut : accepté (lot TW `siege`, rapport `docs/wip/tw/bataille-ia-sieges.md`, top 4, 6, 7).

## Contexte
Les tours de muraille et la sortie de la garnison vivaient en `const` dans `siege_extra.rs` (portée 180 m,
cadence 8 s, 5 tireurs, sortie sous 0,5 × la garnison après 120 s), contre la règle « pas de données en
dur ». Ni la garnison ni les tours ne visaient les béliers et engins assaillants (seule l'huile les gênait),
et l'IA assaillante ne battait qu'un seul pan de mur à la fois, même largement supérieure.

## Décision
Tout vit dans `data/rules/siege_works.json` (schéma `siege_works_rules.schema.json`, lu par `SiegeWorkRules`) :
- **`tower`** : `range_m`, `reload_s`, `shots`, `accuracy`, `accuracy_range_loss`, `lethality` (valeurs
  identiques à l'ancien code à fortification 3) et deux leviers `range_per_fortification_m` /
  `shots_per_fortification` (écart par niveau au-dessus de 3, à 0 dans les données livrées : aucun
  changement d'équilibre). **`sortie`** : `ratio`, `delay_s`.
- **`counter_battery`** (contre-batterie, dans les deux sens) :
  - les carreaux de tour visent aussi béliers et tours roulantes (`tower_engine_factor`, 0 = jamais) ;
    les hommes à portée passent d'abord, une machine à toit ne reçoit un carreau que faute d'autre cible ;
  - `roofed_hit_factor` (0,1) remplace la constante « toit et peaux mouillées » de `shooting.rs` ;
  - `engine_duel` : un engin de l'assaillant tire sur un engin de la garnison à portée
    (`engine_duel_range_factor`) avant de battre les murs, puis reprend son pan ;
  - `wall_shooters_target_engines` : l'IA de la garnison désigne bélier (tant que la porte tient), tours
    roulantes et engins à une part `wall_shooter_share` de ses tireurs de rempart (à tour de rôle, 0,25) ;
    les autres tirent à volonté sur les hommes (viser un bélier à toit gaspille des carreaux : à 0,5, l'escalade
    d'`f5d` passait de 21 à 24 victoires sur 24).
- **`second_assault`** : l'IA assaillante répartit ses engins (un sur deux) et ses échelles sur deux pans
  distincts du front quand le rapport de force dépasse `min_power_ratio` (1,5) avec au moins
  `min_foot_regiments` (4) régiments à pied et `min_engines` (2) engins ; le second pan est le plus faible
  dont le milieu est à `min_separation_m` (60 m) du premier. Sinon comportement inchangé (un pan).

## Conséquences
- Les carreaux de tour coûtent un peu de bélier ; `sg1::the_ram_strikes_the_gate_in_rhythm` tolère 10 % de
  perte d'équipage ; `f5d::a_ladder_escalade…` passe à 24 graines (la fourchette à 6 graines bougeait au
  moindre tir).
- Le bélier est une unité synthétique, absente de `View::enemies` : la contre-batterie de l'IA parcourt
  `view.units`.
- Pas de sape ni de mine, pas d'engins de garnison posés sur les tours (rapport § hors lot).

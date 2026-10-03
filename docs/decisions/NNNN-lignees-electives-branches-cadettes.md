# ADR NNNN — Pas de déshérence pour les sièges électifs, branches cadettes des lignées éteintes

Date : 2026-10-03 (lot LR-05). Numéro à attribuer à la fusion.

## Contexte
La sonde `century_probe` (464 tours × 10 graines) faisait disparaître une centaine de factions sur 177
avant 1453, une cinquantaine dès 1360. La trace `DEATH_TRACE=1` montre que toutes ces disparitions
viennent de la perte du dernier titre, aucune de la conquête : quand un souverain meurt sans héritier
dans sa faction ni parent dans une autre (`feudal::inherit_titles_on_extinction`, lot FE F3), ses titres
font retour au suzerain et la faction se fond dans l'Empire, la Horde d'Or, la Bohême…

Deux causes :
1. Un tiers des cas sont des factions à succession **élective** (archevêchés, évêchés, ordres,
   républiques) traitées comme des dynasties éteintes, ce qui n'a pas de sens historique.
2. 98 factions n'ont qu'un personnage dans `data/characters` (24 aucun) : la donnée ne modélise ni
   frères, ni cousins, ni branches collatérales. Le premier décès sans fils suffit à éteindre la maison.
   Compléter à la main les familles de ~120 maisons est hors de portée.

## Décision
- Succession élective : la mort du chef sans successeur désigné ne déclenche ni déshérence ni prétention
  par les femmes ; un nouveau chef est élu (`dynasty::spawn_ruler`), la faction garde ses titres.
- Succession dynastique, sans héritier dans la faction ni parent à l'étranger : avec la probabilité
  `feudal_rules.collateral_line_percent` (donnée, 75 %), une branche cadette de la même maison relève
  les titres (`dynasty::spawn_cadet`, `feudal::cadet_branch`, événement « branche cadette »). Sinon,
  déshérence comme avant. Les parents à l'étranger gardent la priorité (union personnelle, Bourgogne 1361).
- Défaut du code 0 (comportement d'avant) pour un arbre de données sans la clé.

## Conséquences
- Moins de factions disparaissent par extinction ; la déshérence reste possible (25 % des lignées éteintes
  sans parent) et les unions personnelles sont inchangées.
- Un tirage de `state.rng` de plus à chaque extinction dynastique : les trajectoires des graines changent.
- Les maisons générées par branche cadette portent le nom de la maison éteinte.

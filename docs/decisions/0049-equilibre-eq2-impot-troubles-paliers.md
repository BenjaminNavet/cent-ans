# 0049 — Impôt de guerre sur la réserve, troubles qui retombent, un palier par chaîne au départ

Date : 2026-09-25. Lot : EQ2 (équilibre, suite d'EQ1). Statut : accepté.

## Contexte

EQ1 (ADR 0029) laissait cinq points ouverts, mesurés avec `balance_probe` (8 × 200 tours),
`century_probe` (5 × 464 tours) et une sonde de diagnostic neuve, `eq2_probe` :

- **Impôt Haut** dans 42-45 % des échantillons (cible E2 < 40 %). Diagnostic : 82 % de ces
  échantillons sont des factions **en guerre**, 13 % en déficit, 3 % endettées ; la règle de l'IA
  mettait l'impôt Haut dès qu'une guerre était ouverte, trésor plein ou non.
- **Troubles de province bloqués à 100** (Lothian, Guyenne : jusqu'à 30 saisons d'affilée).
  Diagnostic : la jauge `ProvinceState::unrest` ne perdait que 2 points par saison, alors qu'une
  régence (minorité de Richard II, captivité de David II) en ajoute 5 par saison à **toutes** les
  provinces du royaume ; le bonus de province entière (−5) ne s'applique pas à une province dont
  une colonie reste à l'ennemi. Une province partagée sous régence montait donc de 3 points par
  saison jusqu'à 100, et y restait des années après la fin de la régence (50 saisons pour
  redescendre) ; les prises répétées (+20 la cité, +10 une autre colonie) faisaient de même.
- **Paliers empilés** dans les colonies de départ : 279 empilements (marché + maison des métiers
  + foire à Paris, église paroissiale + abbaye + cathédrale, murs + château), 88 paires de
  branches exclusives (abbaye et cathédrale), 65 bâtiments interdits dans leur type de colonie
  (cathédrale dans une ville, marché dans un village ou un château), 12 prérequis manquants
  (comptoir sans foire). Plusieurs de ces états sont impossibles à atteindre en jeu. En outre, un
  prérequis (`required_building` d'un bâtiment ou d'une unité) exigeait le palier exact : une
  maison des métiers améliorée en foire faisait perdre les couleuvriniers et les piquiers
  flamands, une église paroissiale devenue cathédrale bloquait l'hôtel-Dieu, et le marché
  redevenait constructible à côté de la foire.
- **Aide féodale** jamais choisie : son revenu comptait 4 points contre 15 de mécontentement.
- **Guerre France-Angleterre** : la guerre reprend bien après chaque trêve (le prétendant
  redéclare), mais les phases de guerre s'arrêtent dès la durée minimale de 5 ans.

## Décision

1. **Impôt de l'IA** (`ai::campaign::plan_economy`) : Haut seulement si la faction est endettée,
   si un déficit viderait le trésor avant la fin de sa marge (8 saisons en guerre, 3 en paix),
   ou si elle est en guerre avec un trésor sous `RESERVE_SEASONS` (3) saisons de revenu. Une
   guerre se paie d'abord sur la réserve.
2. **Troubles de province** : décroissance de `disorder_decay_flat` (2) plus
   `disorder_decay_percent` (10 %) de la jauge par saison (`data/rules/population.json`). Une
   jauge à 100 perd 12 points par saison (17 avec la province entière) ; sous régence, une province
   partagée s'équilibre vers 30 au lieu de 100.
3. **Un palier par chaîne** : les données de départ sont nettoyées (191 colonies, 95 provinces :
   on garde le palier le plus haut ; la cathédrale l'emporte sur l'abbaye dans une cité ; une
   cathédrale hors cité devient collégiale ; un bâtiment interdit dans le type est retiré ; un
   comptoir reçoit sa foire, un atelier de tissage son marché) et `setup_1337` normalise en plus
   la liste (`GameData::normalize_building_tiers`), ce qui protège des données futures. Un palier
   supérieur **satisfait** tout prérequis visant un palier qu'il a remplacé
   (`GameData::building_satisfies` / `has_building` : construction, recrutement, compagnons,
   diètes) et rend ce palier « déjà construit ». L'université exige une collégiale (ou mieux) et
   non plus une cathédrale, réservée aux cités : Padoue, Salamanque et Lérida redeviennent
   possibles.
4. **Aide féodale** : l'IA pèse le revenu d'un édit fiscal selon le besoin (1 en paix, 2 en guerre
   ou en déficit, 4 si le trésor est aussi sous 3 saisons de revenu, 6 en dette ; +0,25 par
   vassal, 3 au plus) ; l'édit coûte 6 + 4 (paysans) de mécontentement au lieu de 10 + 5. Il est
   levé dans les provinces calmes d'un royaume en guerre et à court d'argent, puis abandonné.
5. **Guerre France-Angleterre** : `pretender_reluctance` 35 → 45 (`data/ai/diplomacy.json`,
   poids de donnée lu par `negotiation.rs`, code inchangé) : le prétendant refuse plus longtemps
   une paix blanche.

## Conséquences

- Moins d'impôt Haut, sans banqueroutes supplémentaires ; les guerres puisent dans les trésors.
- Une province pacifiée retrouve un niveau normal en quelques saisons ; les troubles pèsent
  toujours sur le mécontentement (EQ1) tant qu'ils durent.
- Les colonies de départ sont dans un état atteignable ; les grandes villes perdent les effets
  cumulés des paliers inférieurs (Paris : 8 bâtiments au lieu de 13). Les sauvegardes d'avant EQ2
  gardent leurs listes empilées (effets toujours calculés), sans erreur.
- Mesures avant/après : `docs/wip/eq2-equilibre.md`.

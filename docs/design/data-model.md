# Cent Ans — Modèle de données

Date : 2026-09-23. Statut : v1 (jalon M0). Complète `2026-09-23-cent-ans-design.md`.

Toutes les données de jeu vivent dans `data/` sous forme de fichiers JSON, un fichier par entité,
validés par les JSON Schema (draft 2020-12) de `data/schemas/`. Le cœur Rust (`core/crates/data-model`)
les charge au démarrage ; aucune règle ni donnée n'est codée en dur.

## 1. Arborescence

```
data/
  schemas/            JSON Schema de chaque type + common.schema.json (définitions partagées)
  factions/           fac_*.json      15 factions (3 jouables)
  provinces/          prov_*.json     8 provinces d'échantillon (carte complète : pipeline géo, M1)
  unit_types/         unit_*.json     13 unités de bataille
  buildings/          bld_*.json      25 bâtiments en 6 catégories
  technologies/       tech_*.json     10 militaires + 12 civiles
  characters/         chr_*.json      39 personnages réels de 1337
  resources/          res_*.json      9 ressources
  religions/          rel_*.json      6 religions / obédiences / hérésies
```

Règle : **le nom du fichier est l'`id` de l'entité** (`data/factions/fac_france.json` contient `"id": "fac_france"`).

## 2. Conventions d'identifiants

| Préfixe | Entité | Exemple |
|---|---|---|
| `fac_` | faction | `fac_france`, `fac_england`, `fac_burgundy` |
| `prov_` | province | `prov_normandie`, `prov_kent` |
| `unit_` | type d'unité | `unit_longbowmen`, `unit_trebuchet` |
| `bld_` | bâtiment | `bld_hotel_dieu`, `bld_stone_walls` |
| `tech_` | technologie | `tech_bombards`, `tech_windmills` |
| `chr_` | personnage | `chr_philippe_vi`, `chr_edward_of_woodstock` |
| `res_` | ressource | `res_wheat`, `res_wool` |
| `rel_` | religion | `rel_catholic`, `rel_lollard` |
| `cul_` | culture (vocabulaire libre, pas d'entité) | `cul_french`, `cul_english`, `cul_flemish` |
| `trait_` | trait de personnage (vocabulaire libre) | `trait_chivalrous`, `trait_pious` |
| `sea_` | zone maritime (transport abstrait) | `sea_channel`, `sea_north_sea`, `sea_bay_of_biscay` |

- `snake_case` ASCII uniquement (`[a-z0-9_]`) ; les patterns sont imposés par `common.schema.json`.
- Identifiants en anglais ou en forme neutre ; les noms de lieux gardent la forme française usuelle
  (`prov_ile_de_france`, `prov_bourgogne`) pour rester lisibles côté modding.
- Les effets de jeu (`effect`) n'utilisent jamais un préfixe d'identifiant : les effets sur les
  troupes s'appellent `army_*` (`army_armor`, `army_morale`...) et non `unit_*`, afin qu'un outil
  puisse détecter toute chaîne `xxx_…` comme une référence.

Cultures utilisées dans les données de départ : `cul_french`, `cul_english`, `cul_welsh`, `cul_scottish`,
`cul_breton`, `cul_flemish`, `cul_occitan`, `cul_navarrese`, `cul_castilian`, `cul_catalan`,
`cul_portuguese`, `cul_savoyard`, `cul_german`, `cul_lombard`, `cul_ligurian`.

## 3. Types communs (`common.schema.json`)

| Type | Rôle |
|---|---|
| `localized_name` | `{display, local, local_language}` : nom français d'affichage + nom d'époque en langue locale (ancien français, moyen anglais, gascon, breton...). |
| `historical_date` | `{value, uncertain, place, note}` ; `value` est une date ISO partielle `AAAA`, `AAAA-MM` ou `AAAA-MM-JJ` (calendrier julien). **`uncertain: true` marque une date approximative ou disputée**, `note` explique. |
| `uncertain_integer` | même principe pour un entier estimé. |
| `percent` | entier 0-100 (jauges de population). |
| `social_class` | `peasants`, `burghers`, `clergy`, `nobility`. |
| `effect` | modificateur `{effect, class?, unit_category?, value, mode: add|percent}` ; liste fermée d'effets (`wealth`, `health`, `unrest`, `goods_satisfaction`, `tax_income`, `trade_income`, `research_civil`, `army_armor`, `fortification_level`, `siege_resistance`, `piety`...). |
| `cost` | `{money, resources?}` en livres tournois + ressources. |
| `sources` | liste de titres de pages Wikipédia ou d'ouvrages. |

Les autres schémas référencent ces définitions par `"$ref": "common.schema.json#/$defs/..."`. Chaque
schéma porte un `$id` `https://cent-ans.game/schemas/<nom>.schema.json` ; un validateur doit donc
charger tous les schémas dans un registre (Python : `referencing.Registry` ; Rust : `jsonschema` avec
un retriever) avant de valider.

## 4. Entités

### 4.1 Faction (`faction.schema.json`)
État de la carte : `government` (kingdom, duchy, county, republic, theocracy, empire, lordship),
`playable`, `ruler` / `heir` (personnages), `capital` (province) et `capital_city`, `religion`, `culture`,
`succession_law` (`salic`, `male_preference_primogeniture`, `cognatic_primogeniture`, `elective`),
`suzerain` (vassalité de départ), `heraldry` (**blasonnement français** + description + couleurs),
`titles`, `treasury`, `prestige`, `starting_technologies`, `relations` (statuts `war`, `peace`,
`alliance`, `truce`, `embargo`, `vassal`, `overlord`, `marriage_tie`, datés), `ai_personality`.

Factions de départ : France, Angleterre, Bourgogne (jouables) ; Écosse, Bretagne, Flandre, Navarre,
Castille, Aragon, Portugal, Savoie, Papauté d'Avignon, Empire, Milan, Gênes.

### 4.2 Province (`province.schema.json`)
`name` (nom d'époque + nom français), `region`, `terrain` (plains, hills, mountains, forest, marsh,
heath, bocage), `climate`, `neighbors`, `rivers`, `coastal`, `ports`, `sea_zones`, `resources`,
`capital_city` (nom localisé + lat/lon), `owner` (faction qui contrôle), `overlord` (suzerain
féodal, ex. Guyenne : owner Angleterre, overlord France), `holder` (personnage tenant la province en
fief ou apanage), `culture`, `religion`, `population.classes` (par classe : `count`, `unrest`,
`health`, `wealth`, `goods_satisfaction` 0-100), `buildings` de départ, `fortification_level`,
`base_supply`.

L'échantillon actuel : Île-de-France, Normandie (apanage de Jean), Guyenne (anglaise, confisquée
le 24 mai 1337), Flandre (comté vassal), Kent, Bourgogne (duché), Bretagne, Artois (tenu par
Jeanne III, épouse d'Eudes IV). `neighbors` ne liste que les voisins présents dans l'échantillon ;
le pipeline géo (`tools/geo`) régénérera la carte complète et les adjacences. Les capitales et
`starting_location` des factions hors échantillon (`prov_middlesex`, `prov_lothian`, `prov_toledo`,
`prov_barcelona`, `prov_lisboa`, `prov_navarra`, `prov_savoie`, `prov_comtat_venaissin`,
`prov_oberbayern`, `prov_milano`, `prov_genova`, `prov_norfolk`, `prov_northumberland`,
`prov_renfrew`, `prov_hainaut`) sont des **références avancées** : le chargeur doit les tolérer
(avertissement, pas erreur) tant que la carte complète n'existe pas.

### 4.3 Type d'unité (`unit_type.schema.json`)
`category` (infantry, ranged, cavalry, siege), `source_class` (classe sociale de recrutement :
archers chez les paysans, hommes d'armes et chevaliers chez la noblesse, milices et engins chez les
bourgeois), `mounted`, `mercenary`, `soldiers` (60-120 ; 4-8 pour les engins), `cost`, `upkeep`,
`stats` (`melee`, `ranged`, `range` en mètres, `armor`, `morale`, `speed`, `ammo`, `charge`,
`siege_attack`), `abilities` (stakes, pike_square, volley, pavise, charge_lance, dismount,
wall_breach, wall_assault, rain_penalty...), `required_technology`, `required_building`,
`required_culture`, `required_faction`, `equipment`.

### 4.4 Bâtiment (`building.schema.json`)
`category` : `production`, `commerce`, `military`, `religious`, `sanitary`, `fortification` (les six
familles du document de conception). `tier`, `upgrades_from` (chaîne d'amélioration : palissade →
enceinte de pierre → château → boulevard d'artillerie), `cost`, `build_time_turns`, `upkeep`,
`required_technology` / `required_building` / `required_resource`, `requires_coastal`,
`requires_river`, `effects`, `enables_units`.

### 4.5 Technologie (`technology.schema.json`)
`branch` (`military`, `civil`), `tier`, `cost` en points de recherche, `prerequisites`,
`historical_year` (date d'apparition historique, avec `uncertain`), `unlocks.units` /
`unlocks.buildings`, `effects`. Exemples de chaînes : poudre noire → bombardes → artillerie de
campagne → compagnies d'ordonnance ; moulins à eau → moulins à vent ; fiscalité royale →
comptabilité ; réforme hospitalière → hygiène urbaine.

### 4.6 Personnage (`character.schema.json`)
`name`, `epithet`, `sex`, `house`, `faction`, `role` (ruler, consort, heir, commander, noble,
prelate, burgher, exile, claimant, regent...), `birth` / `death` (dates historiques avec
`uncertain`, `place`, `note` ; la simulation peut devancer ou retarder la mort), `titles` (liste
datée), `skills` (`command`, `governance`, `court` de 0 à 10), `traits`, `piety`, `family`
(father, mother, spouses, children, siblings), `starting_location`, `status` (at_court, in_exile,
captive, on_campaign, minor). Les enfants (Édouard de Woodstock, 7 ans ; Philippe « Monsieur »,
13 ans ; David II, 13 ans ; Louis de Male, 6 ans) ont des compétences basses qui progressent.

### 4.7 Ressource (`resource.schema.json`)
`category` (food, raw_material, manufactured, luxury), `base_price`, `satisfies_classes`.
Blé, laine, vin, sel, fer, bois, pierre, poisson, drap.

### 4.8 Religion (`religion.schema.json`)
`kind` (church, obedience, heresy, other_faith), `parent`, `head_faction`, `seat`,
`available_from` / `available_until` (le Schisme de 1378 active `rel_catholic_rome` ; Lollards
vers 1381, Hussites en 1419), `effects`.

## 5. Ajouter ou modifier des données (mod)

1. Créer `data/<dossier>/<id>.json` avec le bon préfixe ; l'`id` doit égaler le nom du fichier.
2. Respecter le schéma correspondant (`additionalProperties: false` : toute clé inconnue est refusée).
3. Toute référence (`fac_`, `prov_`, `chr_`...) doit pointer vers un fichier existant, sauf les
   provinces avant la carte complète.
4. Pour surcharger une entité existante, un mod fournit un fichier de même `id` dans
   `mods/<nom>/data/<dossier>/` ; le chargeur applique `data/` puis chaque mod dans l'ordre, le
   dernier fichier de même `id` remplaçant intégralement le précédent (pas de fusion partielle, pour
   rester lisible et déterministe).
5. Valider : `uv run --project tools python -m validate_data` (script à venir dans `tools/`) ou,
   en attendant, avec `jsonschema` + `referencing` en chargeant les neuf schémas dans un registre.
6. Renseigner `sources` et poser `"uncertain": true` sur toute date ou valeur estimée.

## 6. Sources historiques (situation au printemps 1337)

Connaissances générales vérifiées contre les pages Wikipédia (fr/en) suivantes.

| Sujet | Pages Wikipédia | Points retenus |
|---|---|---|
| Déclenchement de la guerre | Guerre de Cent Ans ; Philippe VI de France ; Édouard III | Confiscation de la Guyenne le 24 mai 1337 ; défi d'Édouard III le 19 octobre 1337 ; revendication de la couronne en janvier 1340 |
| Maison de Valois | Philippe VI de France ; Jeanne de Bourgogne (1293-1349) ; Jean II de France ; Bonne de Luxembourg ; Charles II d'Alençon | Jean duc de Normandie depuis le 17 février 1332 ; Charles V naît le 21 janvier 1338 |
| Cour d'Angleterre | Édouard III ; Philippa de Hainaut ; Édouard de Woodstock ; Isabelle de France (1295-1358) ; Henri de Grosmont ; Guillaume de Bohun ; Jean de Hainaut | Duché de Cornouailles créé le 17 mars 1337 ; comtés de Derby et Northampton créés le 16 mars 1337 |
| Exilés et prétendants | Robert III d'Artois ; Édouard Balliol | Robert d'Artois banni en 1332, en Angleterre depuis 1334 |
| Bourgogne et Artois | Eudes IV de Bourgogne ; Jeanne III de Bourgogne ; Philippe de Bourgogne (1323-1346) ; Comté d'Artois | Jeanne III comtesse de Bourgogne et d'Artois depuis le 21 janvier 1330 |
| Écosse | David II ; Seconde guerre d'indépendance écossaise ; Andrew Murray (1298-1338) ; Robert II d'Écosse | David II à Château-Gaillard de 1334 à 1341 ; Andrew Murray Gardien 1335-1338 |
| Bretagne | Jean III de Bretagne ; Jean de Montfort (1295-1345) ; Jeanne de Penthièvre ; Charles de Blois ; Guerre de Succession de Bretagne | Mariage Charles de Blois – Jeanne de Penthièvre le 4 juin 1337 |
| Flandre | Louis Ier de Flandre ; Louis II de Flandre ; Jacob van Artevelde ; Comté de Flandre | Embargo anglais sur la laine (août 1336) ; Artevelde capitaine de Gand le 3 janvier 1338 |
| Navarre | Jeanne II de Navarre ; Philippe III de Navarre | Échange Champagne/Brie contre Angoulême et Mortain (1336) |
| Ibérie | Alphonse XI de Castille ; Pierre IV d'Aragon ; Alphonse IV de Portugal | Guerre luso-castillane 1336-1339 ; Pierre IV roi depuis le 24 janvier 1336 |
| Savoie, Papauté, Empire, Italie | Aymon de Savoie ; Benoît XII ; Papauté d'Avignon ; Louis IV du Saint-Empire ; Azzone Visconti ; Simone Boccanegra ; République de Gênes | Benoît XII pape depuis le 20 décembre 1334 ; alliance Louis IV – Édouard III à l'été 1337 ; Boccanegra doge le 23 septembre 1339 |
| Héraldique | Armoiries de la France ; Armoiries royales du Royaume-Uni ; Armorial des ducs de Bourgogne ; Armoiries de l'Écosse ; Armoiries de la Bretagne ; Armoiries de la Navarre ; Armoiries de l'Espagne ; Armoiries du Portugal | France ancien (semé) avant Charles V ; léopards d'Angleterre non écartelés avant 1340 ; hermine plain depuis 1316 |
| Armées et équipement | Arc long anglais ; Arbalétriers génois ; Bataille de Crécy ; Bataille des éperons d'or ; Goedendag ; Homme d'armes ; Trébuchet ; Mangonneau ; Bombarde ; Artillerie médiévale ; Armure de plates | Pot-de-fer attesté à Rouen en 1338 ; bombardes à Crécy en 1346 ; harnois blanc vers 1400 |
| Économie, villes, techniques | Économie médiévale ; Paris au Moyen Âge ; Rouen ; Gand ; Bruges ; Arras ; Cinque Ports ; Comté d'Artois ; Duché d'Aquitaine ; Duché de Normandie ; Duché de Bretagne ; Duché de Bourgogne | Ordres de grandeur de population pré-peste (Paris 200-250 000 hab.) |
| Religion | Papauté d'Avignon ; Grand Schisme d'Occident ; Lollards ; Jan Hus ; Hussites ; Histoire des Juifs en France ; Royaume de Grenade | Schisme le 20 septembre 1378, fin le 11 novembre 1417 |

### Incertitudes signalées (`"uncertain": true`)

- Naissances sans jour connu : Philippe VI (1293), Jeanne de Bourgogne (v. 1293), Charles de Blois et
  Jeanne de Penthièvre (v. 1319), Louis de Nevers (v. 1304), Jacob van Artevelde (v. 1290), Henri de
  Grosmont (v. 1310), Guillaume de Bohun (v. 1312), Jean de Hainaut (v. 1288), Robert d'Artois (1287,
  sans jour), Édouard Balliol (v. 1283), Andrew Murray (v. 1298), Jean de Montfort (v. 1295), Isabelle
  de France (v. 1295), Benoît XII (v. 1285), Simone Boccanegra (v. 1301), Raoul de Brienne (inconnue).
- Philippa de Hainaut : née entre 1310 et 1314 ; on retient le 24 juin 1310.
- Jeanne III de Bourgogne : née le 1er ou 2 mai 1308, morte le 10 ou 15 août 1347.
- Philippe « Monsieur » : jour de la mort au siège d'Aiguillon (août 1346) discuté.
- Jacob van Artevelde : tué le 17 ou le 24 juillet 1345.
- Robert d'Artois : mort en novembre 1342, jour inconnu ; comté de Richmond (1341) discuté.
- Alphonse XI : mort le 26 ou 27 mars 1350 ; Simone Boccanegra : 3 ou 14 mars 1363.
- Alliance franco-castillane (1336) et alliance anglo-impériale (août 1337) : dates de traité approximatives.
- Trêve Aragon–Gênes (1336) : approximative.
- Bordure des armes du Portugal : nombre de châteaux variable au XIVe siècle.
- Toutes les populations provinciales (`population.uncertain: true`) sont des ordres de grandeur
  pré-peste ; les capitales de Castille (Tolède), d'Aragon (Barcelone) et de l'Empire (Munich, résidence
  de Louis IV) sont des choix de jeu, ces États n'ayant pas de capitale fixe.
- Jean de Luxembourg, roi de Bohême, est rattaché à `fac_france` comme commandeur allié, faute de
  faction Bohême dans la v1.

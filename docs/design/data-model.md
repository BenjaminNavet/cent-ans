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
  buildings/          bld_*.json      26 bâtiments en 6 catégories (M6 : + bld_scriptorium)
  technologies/       tech_*.json     16 militaires + 17 civiles (M6)
  characters/         chr_*.json      50 personnages réels de 1337-1346
  resources/          res_*.json      9 ressources
  religions/          rel_*.json      6 religions / obédiences / hérésies
  traits/             trait_*.json    59 traits de personnage (M4)
  skills/             skill_*.json    30 compétences, 3 branches (M4)
  names/              names_<code>.json  7 listes de prénoms par culture (M4)
  events/             evt_*.json      48 événements de chronique : 26 historiques, 22 aléatoires (M10)
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
| `trait_` | trait de personnage | `trait_chivalrous`, `trait_pious` |
| `skill_` | compétence de l'arbre à trois branches | `skill_hardiesse`, `skill_bon_justicier` |
| `names_` | liste de prénoms par culture/langue (`names_<code langue 2 lettres>`) | `names_fr`, `names_en` |
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
| `effect` | modificateur `{effect, class?, unit_category?, value, mode: add|percent}` ; liste fermée d'effets (`wealth`, `health`, `unrest`, `goods_satisfaction`, `tax_income`, `trade_income`, `research_civil`, `army_armor`, `fortification_level`, `siege_resistance`, `piety`... ; M6 : `research_points`, `army_melee`). |
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

M6 : 33 technologies (16 militaires, 17 civiles, rangs 1-5), chacune datée (`historical_year`) et
sourcée ; ajouts : pavois, brigandine, bâtons à feu → exercice des couleuvriniers, compagnies
d'ordonnance (1445) → armée permanente (`tech_standing_companies`), francs-archers (1448),
comptabilité en partie double, lettres de change, quarantaine (Raguse 1377), commerce hanséatique,
gothique flamboyant. Les effets de recherche sont unifiés en `research_points` (points par tour) :
abbaye 0,25, cathédrale 0,25, maison des métiers 0,5, scriptorium (nouveau) 1, université 3 ;
technologies : universités 2, poudre 1, moulins à papier 2, imprimerie 5. `research_civil` /
`research_military` restent dans le schéma pour les traits et compétences (sans effet).

### 4.6 Personnage (`character.schema.json`)
`name`, `epithet`, `sex`, `house`, `faction`, `role` (ruler, consort, heir, commander, noble,
prelate, burgher, exile, claimant, regent...), `birth` / `death` (dates historiques avec
`uncertain`, `place`, `note` ; la simulation peut devancer ou retarder la mort), `titles` (liste
datée), `skills` (`command`, `governance`, `court` de 0 à 10), `traits`, `piety`, `family`
(father, mother, spouses, children, siblings), `starting_location`, `status` (at_court, in_exile,
captive, on_campaign, minor). Les enfants (Édouard de Woodstock, 7 ans ; Philippe « Monsieur »,
13 ans ; David II, 13 ans ; Louis de Male, 6 ans) ont des compétences basses qui progressent.

### 4.6bis Compléments personnages (M4)

`data/characters/` compte désormais 50 personnages. Ajouts pour couvrir les successions jusque
vers 1360 pour les maisons royales de France, Angleterre, Bourgogne, Écosse, Navarre, Castille et
Bretagne :

- France (Jean de Normandie × Bonne de Luxembourg) : Charles V (21/01/1338), Louis d'Anjou
  (23/07/1339), Jean de Berry (30/11/1340), Philippe le Hardi (17/01/1342) — `status: "unborn"`.
- Angleterre (Édouard III × Philippa de Hainaut) : Lionel d'Anvers (29/11/1338), Jean de Gand
  (06/03/1340) — `status: "unborn"`.
- Bourgogne : Jeanne Ire d'Auvergne (épouse de Philippe de Bourgogne « Monsieur ») et leur fils
  posthume Philippe de Rouvres (24/11/1346, `status: "unborn"`), dernier duc capétien.
- Navarre : Charles II de Navarre « le Mauvais » (10/10/1332, fils de Philippe d'Évreux et Jeanne II) ;
  `fac_navarre.heir` renseigné.
- Castille : Marie de Portugal (épouse d'Alphonse XI, fille d'Alphonse IV de Portugal) et leur fils
  Pierre Ier « le Cruel » (30/08/1334) ; `fac_castile.heir` renseigné.

Chaque ajout porte `historical: true`, des sources Wikipédia et des liens familiaux bidirectionnels
(`father`/`mother` sur l'enfant, `children` sur les deux parents, `spouses` sur les deux conjoints).
Le champ `status: "unborn"` marque un personnage historique dont la date de naissance (`birth`) est
postérieure à 1337 : la simulation (`sim-campaign`) le fait naître à cette date si les deux parents
sont vivants et mariés à ce moment-là, sinon l'histoire diverge et il n'apparaît jamais.

### 4.9 Trait (`trait.schema.json`)

`data/traits/` (59 fichiers) couvre l'intégralité des `trait_*` référencés dans `data/characters/`
plus les traits acquis en jeu (blessure, captivité, spécialisations militaires). Champs : `id`,
`name` (français), `category` (`personality`, `martial`, `governance`, `physical`, `acquired`),
`effects` (liste d'`Effect`, réutilisant `EffectKind`), `opposites` (traits mutuellement exclusifs,
ex. `trait_generous`/`trait_greedy`, `trait_chivalrous`/`trait_ruthless`), `description`. Les traits
ne sont plus un vocabulaire libre : toute référence `trait_*` (personnage, opposé) doit pointer vers
un fichier de `data/traits/` (erreur de chargement sinon).

### 4.10 Compétence (`skill.schema.json`)

`data/skills/` (30 fichiers) forme un arbre à trois branches — `command` (commandement),
`governance` (gouvernance), `court` — de 10 compétences chacune, réparties en 3 tiers avec
prérequis internes à la branche (`prerequisites`) et un coût en points égal au tier (`cost`).
Chaque nœud porte des `effects` (`Effect`, `EffectKind` étendu pour l'occasion de
`siege_speed`, `construction_speed`, `diplomacy`, `intrigue`, `fertility`, `battle_charge`,
`battle_ranged`, `battle_defense`), un nom français et une description.

### 4.11 Liste de prénoms (`names.schema.json`)

`data/names/names_<code>.json` (7 fichiers : `fr`, `en`, `nl`, `oc`, `es`, `it`, `de` — codes de
langue à deux lettres, distincts des identifiants `cul_*`) fournit, par langue/culture d'époque,
une trentaine de prénoms masculins et féminins (formes médiévales : Jehan, Guillaume, Aliénor,
Isabeau...) et quelques surnoms/épithètes neutres, plus la liste des `cultures` (`cul_*`)
associées. Utilisée par `sim-campaign` pour nommer les personnages générés à la naissance.

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

## 7. Chargement Rust (`core/crates/data-model`)

Statut : v1 (jalon M1, section 5 de `m1-campaign-map.md`). Le crate `data-model` charge `data/` en
structs typées et vérifie les références ; `godot-bridge` en expose une vue en lecture seule à Godot.

### 7.1 Structs

Chaque schéma a sa struct `serde`, toutes marquées `#[serde(deny_unknown_fields)]` : une clé
inconnue dans un fichier (ou un champ ajouté au schéma sans mettre à jour la struct) fait échouer
le chargement, ce qui rend visible toute dérive entre `data/schemas/` et le code.

| Module | Types |
|---|---|
| `ids` | Newtypes `FactionId`, `ProvinceId`, `UnitTypeId`, `BuildingId`, `TechnologyId`, `CharacterId`, `ResourceId`, `ReligionId`, `CultureId`, `TraitId`, `SkillId`, `NamesId`, `SeaZoneId`. La désérialisation valide le préfixe et l'alphabet `[a-z0-9_]` (`common.schema.json`) : un `prov_` placé dans un champ `owner` est refusé. |
| `common` | `LocalizedName`, `HistoricalDate` (`value`, `uncertain`, `place`, `note` ; `year()`), `UncertainInteger`, `Percent`, `SocialClass` (+ `ALL`, `key()`), `UnitCategory`, `EffectKind` (liste fermée, étendue M4 : `SiegeSpeed`, `ConstructionSpeed`, `Diplomacy`, `Intrigue`, `Fertility`, `BattleCharge`, `BattleRanged`, `BattleDefense` ; M6 : `ArmyMelee`, `ResearchPoints`), `EffectMode`, `Effect`, `Cost` (`resources: BTreeMap<ResourceId, u32>`), `Sources`. |
| `entities::faction` | `Faction`, `Government`, `SuccessionLaw`, `Heraldry`, `Relation`, `RelationStatus`, `AiPersonality`. |
| `entities::province` | `Province`, `Terrain`, `Climate`, `CapitalCity`, `Population`, `PopulationClasses` (`get(class)`, `iter()`, `total()`), `PopulationClass`, `ProvinceGeo` (`geo` optionnel : `capital_lonlat`, `seed_lonlat`, `voronoi_weight?`, écrit par `tools/geo`). |
| `entities::unit_type` | `UnitType`, `UnitStats`, `Ability`. |
| `entities::building` | `Building`, `BuildingCategory`. |
| `entities::technology` | `Technology`, `TechBranch`, `TechUnlocks`. |
| `entities::character` | `Character`, `Sex`, `Role`, `CharacterStatus` (M4 : + `Unborn`), `Title`, `Skills`, `Family`. |
| `entities::resource` | `Resource`, `ResourceCategory`. |
| `entities::religion` | `Religion`, `ReligionKind`. |
| `entities::trait` (fichier `trait_.rs`, module `r#trait`) | `Trait`, `TraitCategory`. |
| `entities::skill` | `Skill`, `SkillBranch`. |
| `entities::names` | `NameList`. |
| `map` | `MapMeta` (`data/map/map.json`) et `ProvinceGeometry` (une feature de `data/map/provinces.geojson` : `id`, `centroid`, `neighbors`, `capital_px`, polygone brut dans `geometry: serde_json::Value`). Ces deux types sont produits par un outil : les clés inconnues sont conservées dans `extra` au lieu d'être refusées. |
| `load` | `GameData` (un `BTreeMap<Id, T>` par entité — dont `traits`, `skills`, `names` — `map: Option<MapMeta>`, `province_geometry`), `Warning`, `ReferenceError`, `DataError`, `load_entities`. |

Les champs optionnels du schéma sont des `Option<T>` ; les listes optionnelles sont des `Vec<T>`
vides par défaut ; les booléens à `default` du schéma (`playable`, `mounted`, `historical`,
`population.uncertain`...) reprennent la même valeur par défaut.

### 7.2 `GameData::load(root) -> Result<(GameData, Vec<Warning>), DataError>`

1. Lit chaque dossier d'entités (`factions/`, `provinces/`, ... ; un dossier manquant est une erreur),
   fichiers triés par nom pour un chargement déterministe.
2. Erreur si le nom de fichier diffère de l'`id`, si un `id` est en double, si le JSON est invalide
   ou contient une clé inconnue.
3. Lit `map/map.json` et `map/provinces.geojson` s'ils existent (absents tant que le pipeline géo n'a
   pas tourné) ; avertissement pour un polygone sans province ou une province sans polygone.
4. Vérifie les références croisées :

| Sévérité | Références |
|---|---|
| **Avertissement** (`Warning`) | toute référence à une **province** inconnue : `faction.capital`, `province.neighbors`, `character.starting_location`, ids de `provinces.geojson`. Toléré tant que la carte est partielle (section 4.2). |
| **Erreur** (`DataError::References`, toutes listées d'un coup) | faction : `ruler`, `heir`, `religion`, `suzerain`, `starting_technologies`, `relations[].faction` ; province : `owner`, `overlord`, `holder`, `religion`, `resources`, `buildings` ; unité : `required_technology`, `required_building`, `required_faction`, `cost.resources` ; bâtiment : `upgrades_from`, `required_technology`, `required_building`, `required_resource`, `enables_units`, `cost.resources` ; technologie : `prerequisites`, `unlocks.units`, `unlocks.buildings` ; personnage : `faction`, `family.*`, **`traits` (M4 : chaque `trait_*` doit exister dans `data/traits/`)** ; religion : `parent`, `head_faction` ; **trait : `opposites` (M4)** ; **compétence : `prerequisites` (M4)**. |

Les cultures (`cul_`) et zones maritimes (`sea_`) restent un vocabulaire libre : seul le préfixe est
vérifié. Depuis M4, les traits (`trait_`) ne le sont plus : `character.traits[]` et `trait.opposites[]`
doivent pointer vers un fichier existant de `data/traits/`, de même que `skill.prerequisites[]` vers
`data/skills/`.

Tests : `cargo test -p data-model` charge le vrai dossier `data/` (test `real_data`, avertissements
imprimés avec `--nocapture`) et des fixtures minimales écrites dans un dossier temporaire pour chaque
règle de validation.

### 7.3 API GDExtension : `GameDataStore` (`core/crates/godot-bridge`)

`RefCounted` utilisable en autoload. Ne renvoie que des valeurs Godot simples ; un id inconnu donne un
`Dictionary` vide. Godot ne lit jamais `data/` directement (hors images de `data/map/`).

| Méthode | Retour |
|---|---|
| `load(data_dir: String) -> bool` | Charge `data_dir` (chemin absolu vers `data/`). `false` + `push_error` en cas d'erreur. |
| `is_loaded() -> bool` | Vrai après un `load` réussi. |
| `get_warnings() -> PackedStringArray` | Avertissements du dernier chargement (`entité.champ: message`). |
| `get_province_ids() -> PackedStringArray` | Ids triés. |
| `get_province(id) -> Dictionary` | `id`, `display_name`, `local_name` (repli sur `display_name`), `region`, `terrain`, `coastal`, `port` (au moins un port), `capital` (nom français), `owner`, `owner_display_name` (`short_name` de la faction), `overlord` (`""` si absent), `holder`, `population` (`{peasants, burghers, clergy, nobility}` en effectifs), `population_total`, `neighbors` (`PackedStringArray`) ; et, quand `provinces.geojson` existe, `centroid` et `capital_px` en `Vector2`. |
| `get_faction_ids() -> PackedStringArray` | Ids triés. |
| `get_faction(id) -> Dictionary` | `id`, `name`, `short_name`, `playable`, `color` (`Color` depuis `heraldry.primary_color`), `secondary_color` (blanc si absent), `blazon`, `capital`, `ruler`. |
| `get_character_ids() -> PackedStringArray` | Ids triés. |
| `get_character(id) -> Dictionary` | `id`, `name`, `epithet`, `birth` / `death` (`{value, uncertain, year}` ; vide si inconnu), `faction`, `role`, `titles` (`PackedStringArray`), `skills` (`{command, governance, court}`), `starting_location`. |

Vérification headless : `godot --headless --path game --script "$PWD/core/checks/data_store_check.gd"`
(après `core/build.sh`) charge `data/` et vérifie que `prov_normandie` appartient à `fac_france`.

### 7.4 `CampaignSim` (M2)

Objet GDExtension exposant une partie en cours (`core/crates/godot-bridge/src/campaign_sim.rs`).
Les données de jeu sont chargées une fois par processus et partagées entre instances, ce qui
permet `load_from_string` sur un objet neuf.

| Méthode | Retour |
|---|---|
| `new_campaign(data_dir, player, seed) -> bool` | charge `data/` et construit le départ 1337 |
| `save_to_string() -> String`, `load_from_string(json) -> bool` | sauvegarde JSON versionnée |
| `get_turn()`, `get_date_label()`, `get_player_faction()` | `-1` / `""` avant `new_campaign` |
| `get_faction_summary(id)` | `{treasury, income, at_war_with[], allies[], provinces_count, armies_count, alive, projected_income, army_upkeep, building_upkeep, tax_rate}` |
| `get_province_state(id)` | `{owner, controller, garrison[unit], siege{attacker, turns_left}?, unrest, devastation, population_total}` |
| `get_province_city(id)` (M3) | `{classes: {peasants, burghers, clergy, nobility: {count, unrest, health, wealth, goods_satisfaction}}, buildings: [{id, name, category, upkeep}], construction: {building, name, turns_left}?, fortification_level, capacity, buildable: [{building, name, category, cost, turns, available, reason}], resources: [ids], effects: {tax_income, trade_income, health, unrest, wealth, goods_satisfaction, growth, garrison, fortification_level, recruit_cost, supply}}`. Each `effects` entry is `{flat, percent}`. `construction` is present only while a building is under way. |
| `get_faction_economy(id)` (M3) | `{treasury, income, projected_income, army_upkeep, building_upkeep, tax_rate: "low"\|"normal"\|"high", goods: {resource_id: count}, goods_categories: [..]}` |
| `get_army_ids()`, `get_army(id)` | `{faction, general, general_name, location, units[unit], movement_points, supply, stance, path[]}` |
| `get_reachable(army)` | `{province_id: coût}` |
| `find_path(army, target)` | `PackedStringArray` (vide si inatteignable ou déjà sur place) |
| `get_recruitable(province)` | `[{unit_type, name, cost, upkeep, available, reason}]` |
| `submit_order({type, ...})` | `{ok, error}` — types snake_case identiques aux variantes Rust `Order` |
| `end_turn()`, `get_events()` | `[{kind, text_fr, province, army, faction}]` |

`unit` = `{unit_type, name, strength, max_strength, morale}`. Les identifiants inconnus donnent un
dictionnaire vide. Vérification headless : `core/checks/campaign_sim_check.gd`.

### 7.5 `CampaignSim` : personnages et dynasties (M4)

| Méthode | Retour |
|---|---|
| `get_character(id)` | `{id, name, epithet, sex: "male"\|"female", age, alive, faction, house, title, role, skills{command, governance, court}, experience, skill_points, xp_to_next, skills_learned[], traits[{id, name, category, description}], spouse, spouse_name, children[{id, name, age}], father, mother, location, army, governor_of, captive, piety, prestige}`. `role` est l'activité affichée : `"général de l'armée en X"`, `"gouverneur de Y"`, `"à la cour"` ou `"Captif(ve)"`. |
| `get_faction_characters(faction)` | ids vivants : dirigeant, héritier, puis par âge |
| `get_skill_tree()` | `[{id, name, branch, tier, prerequisites[], cost, description, effects[{kind, value, mode}]}]` |
| `get_learnable(character)` | ids apprenables maintenant (prérequis appris, points suffisants) |
| `get_marriage_candidates(character)` | `[{id, name, age, faction}]` |
| `get_province_state(id)` | gagne `governor` et `governor_name` quand un gouverneur est nommé |
| ordres | `learn_skill{character, skill}`, `assign_governor{province, character}`, `propose_marriage{character, spouse}`, `debug_grant_xp{character, amount}` (tests headless) |
| `GameDataStore.get_trait(id)`, `get_skill(id)` | définitions statiques |

Événements M4 : `birth`, `death`, `succession`, `regency`, `no_heir`, `trait_acquired`.

### 7.6 `CampaignSim` : technologies (M6)

Règles : `core/crates/sim-campaign/src/research.rs` (spec `docs/design/m6-technologies.md`).
`FactionState` gagne `research` (tech en cours), `research_progress`, `research_points_last_turn` et
`research_banked` (progression conservée des recherches abandonnées) ; `STATE_VERSION` = 4.

| Méthode | Retour |
|---|---|
| `get_tech_tree(faction)` | `[{id, name, branch: "military"\|"civil", tier, cost, effective_cost, prerequisites[], unlocks{units[], buildings[]}, unlock_names{units[], buildings[]}, effects[{kind, value, mode, unit_category}], description, historical_year, historical_uncertain, state: "known"\|"available"\|"locked"\|"researching", progress}]`, trié par branche, rang, id. `progress` = progression en cours ou mise de côté. |
| `get_research(faction)` | `{technology, name, progress, cost, points_per_turn, turns_left}` ; vide si aucune recherche. `cost` = coût effectif ; `turns_left` = -1 si aucun point. |
| `get_research_points(faction)` | points de recherche par tour (affichés même sans recherche) |
| ordre | `research{technology}` via `submit_order` : refusé si inconnue, déjà acquise ou prérequis manquant ; changer de recherche met la progression de côté |

- Points par tour = 5 + Σ effets `research_points` des bâtiments des provinces **possédées** + Σ ceux des
  technologies acquises + ⌈gouvernance du dirigeant / 2⌉ ; versés en fin de tour juste après
  l'économie. France 1337 ≈ 20 points/tour.
- Coût effectif = `cost` × 1,25 (arrondi au supérieur) si `historical_year` > année courante + 20.
- Achèvement : tech ajoutée, recherche vidée (surplus perdu), événement `technology_researched`
  (toutes factions, texte « X maîtrise une nouvelle technologie : Y. »).
- Effets : `tax_income`, `trade_income` (revenu), `health`, `growth`, `unrest` (population) de toutes
  les techs acquises s'ajoutent à ceux des bâtiments de chaque province **contrôlée** ;
  `army_morale`, `army_melee`, `army_ranged`, `army_armor` (mode `add`, par `unit_category`, toutes
  catégories si absent) s'ajoutent aux statistiques de chaque unité dans la bataille automatique.
  Déblocages : `required_technology` des unités et bâtiments (déjà vérifié par le recrutement et la
  construction) ; `tests/m6.rs` vérifie que chaque `unlocks.*` correspond.
- IA minimale : sans recherche en cours, choisit la tech disponible la moins chère de la branche où
  elle en possède le moins.

### 7.7 `CampaignSim` : diplomatie et religion (M5)

Méthodes dans `core/crates/godot-bridge/src/campaign_sim_diplomacy.rs` (bloc `#[godot_api(secondary)]`).

| Méthode | Retour |
|---|---|
| `get_diplomacy(faction)` | `[{id, name, color, status: "war"\|"truce"\|"peace"\|"alliance"\|"vassal"\|"suzerain", attitude, attitude_reasons[{text, value}], truce_turns_left, embargo_by_us, embargo_on_us, war_score, casus_belli, claims[], religion, religion_name, loyalty (-1 si pas vassal), power, allies[]}]` ; `attitude` = attitude de l'autre faction envers `faction`. |
| `evaluate_proposal(order)` | `{accept, score, reasons[{text, value}]}` pour `propose_peace`, `propose_alliance`, `demand_vassalage`, `propose_faction_marriage`, `request_papal_mediation` ; pour `declare_war`, `reasons` liste motif, parjure et alliés appelés aux armes. |
| `get_offers()` | `[{id, from, from_name, kind, text, expires_in}]` (propositions reçues par le joueur). |
| `answer_offer(id, accept)` | `{ok, error}` (équivaut à l'ordre `answer_offer`). |
| `get_religion_state(faction)` | `{religion, religion_name, papal_favor, excommunicated, turns_left, schism, obedience_choice_pending}` |
| `get_province_religion(id)` | `{religion, religion_name, heresy, heresy_religion, heresy_name}` |
| `get_province_relations(ids)` | relation du joueur avec le contrôleur de chaque province : `"self"`, `"war"`, `"truce"`, `"peace"`, `"alliance"`, `"vassal"`, `"suzerain"`. |

Ordres (`submit_order`) : `declare_war{target}`, `propose_peace{target, provinces[], tribute}` (chaque
province listée passe à l'autre partie ; tribut positif payé par la cible), `propose_alliance{target}`,
`break_alliance{target}`, `set_embargo{target, active}`, `demand_vassalage{target}`,
`release_vassal{target}`, `send_gift{target, amount}`, `propose_faction_marriage{target, character, spouse}`,
`answer_offer{offer, accept}`, `request_papal_mediation{target}`, `donate_to_church{amount}`,
`choose_obedience{religion}`. Une proposition refusée renvoie `ok = false` et `error` = « X refuse : raisons ».

Événements M5 : `war_declared`, `peace_signed`, `alliance_formed`, `alliance_broken`, `vassalage`,
`vassal_rebellion`, `embargo`, `diplomatic_offer`, `diplomacy`, `marriage`, `excommunication`, `schism`,
`heresy`. Les événements produits par des ordres ouvrent le journal du tour suivant (`pending_events`).

Données : `factions/*.json` gagne `claims[{kind: "throne"|"province", faction?, province?, note}]` ;
`religions/*.json` gagne `historical_adherents[]` (obédience au Schisme) et `origin_provinces[]` (hérésies).
### 7.8 Batailles (M7) : `CampaignSim` et `BattleSim`

Spécification : `docs/design/m7-battles.md` (sièges : `m8-sieges.md` § 2, IA : `m9-ai.md` § 2).
Sauvegarde : `STATE_VERSION` = 4 (`interactive_battles`, `pending_battles` conservées d'un tour à
l'autre, `BattleRequest.attacker_origin`, `BattleRequest.siege` — M8, `#[serde(default)]`).

Côté campagne (`core/crates/sim-campaign/src/battle_request.rs`, pont dans
`core/crates/godot-bridge/src/battle_sim.rs`, bloc `#[godot_api(secondary)]`) :

| Méthode `CampaignSim` | Retour |
|---|---|
| `get_pending_battles()` | `[{index, attacker, defender, province, province_name, attacker_name, defender_name, player_side: "attacker"\|"defender", attacker_strength, defender_strength, seed, siege, fortification, breach}]` ; `seed` est la graine à passer à `BattleSim.setup` (même météo que l'aperçu) ; `siege` = assaut d'une place (le défenseur est la garnison, `defender_strength` son effectif) |
| `get_battle_setup(index)` | `BattleSetup` sérialisé : `{province, province_name, terrain, river, season, player_side, attacker, defender}` avec, par camp, `{faction, faction_name, army, units: [{unit_type, name, category, mounted, soldiers, max_soldiers, morale, experience, stats{melee, ranged, range, armor, morale, speed, ammo, charge?}, abilities[]}], general?: {character, name, command, unit_index, morale_bonus, charge_percent, ranged_percent, defense_percent}}` (effets du général = `character_effects` M4) ; assaut : `siege: {fortification, breach}`, défenseur = garnison (général = gouverneur), pas de rivière ; `{}` si inconnue ou périmée |
| `resolve_battle(index, outcome)` | `{ok, error, events}` : pertes par unité, moral ±, général tombé (`kill`) ou capturé, XP/traits (`on_battle_resolved`), retraite du vaincu, journal |
| `auto_resolve_battle(index)` | événements (résolution `battle_auto`) |
| `set_interactive_battles(bool)`, `get_interactive_battles()` | réglage joueur (défaut `true`) ; `false` = tout auto-résolu comme avant M7 |
| `debug_stage_battle(attacker, defender)` | index de la bataille mise en scène (le défenseur est amené chez l'attaquant), -1 sinon ; tests et captures |
| `debug_stage_siege(army, province)` | index de l'assaut mis en scène (l'armée assiège `province`, garnison de 3 milices si vide), -1 sinon |

Assaut (M8 § 2) : avec `interactive_battles`, l'ordre `assault` du joueur, ou d'une IA contre une place
du joueur, crée une bataille en attente `siege` au lieu d'être résolu ; `resolve_battle` applique alors
les pertes à l'armée et à la garnison et, en cas de victoire, la prise de la place.

Classe `BattleSim` (`RefCounted`) :

| Méthode | Retour |
|---|---|
| `setup(setup_dict, seed) -> bool` | construit le champ (relief, forêts, boue, rivière), la météo et déploie les deux armées |
| `tick(dt)` | avance de `dt` secondes par pas fixes de 0,1 s |
| `issue_command(dict) -> {ok, error}` | `{type: "move", units, x, z, run?, facing?}`, `{type: "attack", units, target, run?}`, `{type: "halt", units}`, `{type: "formation", units, kind: "line"\|"column"\|"square"\|"wedge"}`, `{type: "fire_at_will", units, enabled}`, `{type: "withdraw", units}`, `{type: "target_wall", units, piece}` (siège : engins contre un pan) ; seules les unités du camp du joueur sont acceptées ; ids entiers |
| `set_ai(side, bool)` | IA de bataille d'un camp (autoplay, tests) ; IA tactique M9, décisions toutes les 2 s |
| `get_units()` | `[{id, side, type, name, category, render, soldiers, max_soldiers, initial_soldiers, morale, fatigue, ammo, max_ammo, state, state_label, formation, x, z, y, facing, width, depth, is_general, present, left_field, fire_at_will, can_shoot, running, withdrawing, stakes, target, destination?, on_wall, climbing (pan, -1), climb_progress, ladders, synthetic, ram, siege_tower, wall_breaker}]` ; `state` ∈ idle, marching, charging, melee, shooting, routing, rallied, climbing ; `render` ∈ infantry, archer, cavalry, siege, ram, tower ; `y` = hauteur du chemin de ronde pour les régiments sur les murs |
| `get_soldier_transforms(side)` | `PackedFloat32Array` (x, y, z, angle) par soldat vivant |
| `get_soldier_buffer(side, render)` | `PackedFloat32Array` au format `MultiMesh.buffer` (12 flottants par soldat) |
| `get_terrain()` | `{width, depth, resolution, nx, nz, heights: PackedFloat32Array, forests: [{x, z, radius}], mud: [...], river?: {points: PackedVector2Array, width, fords: [{x, z, half_width}]}, siege?: get_siege()}` |
| `get_siege()` | siège seulement (`{}` sinon) : `{fortification, center: Vector2, square_radius, thickness, wall_height, gate, hold_time, hold_to_win, integrity, pieces: [{index, kind: "wall"\|"gate", a: Vector2, b: Vector2, hp, max_hp, intact, docked_tower}], towers: [{x, z, radius, height}]}` ; à relire pour afficher les dégâts |
| `get_weather()` | `{key: "clear"\|"rain"\|"fog"\|"snow", label}` |
| `get_setup()`, `get_strength(side)`, `get_elapsed()`, `get_ticks()`, `get_height(x, z)` | lecture |
| `is_finished()`, `get_outcome()` | `{winner, attacker, defender: {losses[] (par unité de campagne), total_losses, morale_delta, routed, general_killed, general_captured}, duration}` (format accepté par `resolve_battle`) ; `{}` tant que la bataille dure |
| `get_events()` | `[{time, text_fr, side}]` depuis le dernier appel (« Les Chevaliers de France chargent ! », « Les Archers à l'arc long d'Angleterre sont à court de flèches. ») |

Conversion : les soldats d'un régiment sont la force de campagne (`Unit.strength`, déjà un effectif) ;
les pertes reviennent unité par unité ; un régiment retiré ou sorti du champ garde ses survivants.

### 7.9 Événements de chronique (M10)

Spec : `docs/design/m10-events.md`. Données `data/events/evt_*.json` (schéma
`data/schemas/event.schema.json`, dossier facultatif), types `data_model::entities::event`, règles
`core/crates/sim-campaign/src/chronicle.rs`, pont `core/crates/godot-bridge/src/campaign_sim_events.rs`.

**Format.** `id` (`evt_`), `title`, `text` (français), `kind` (`historical` | `random` | `chained`,
F1 : ne part que programmé par `schedule_event`), `trigger`,
`scope`, `options` (1 à 3 : `text`, `effects[]`, `ai_weight` défaut 1), `historical_date?`, `sources[]`.
- `trigger` : historique → `date {year, season?}` et `until_year?` (défaut : année + 2) ; aléatoire →
  `mean_time_to_happen` (tours ; chance par tour = ⌈1000 / mtth⌉ ‰) ou `chance_permille` ; `conditions[]`
  (toutes requises).
- `scope` : `{type: "global"}` (le joueur décide), `{type: "faction", faction?}` (la faction nommée, sinon
  chaque faction qui remplit les conditions), `{type: "province", faction?, province?}` (la province
  nommée, sinon une tirée parmi celles qui remplissent, restreinte aux provinces de `faction` ; son
  contrôleur décide).
- Conditions (`type`) : `faction_exists {faction}`, `faction_is_player {faction}`, `at_war {a?, b?}` (b
  absent : en guerre contre quiconque), `controls {faction?, province}`, `character_alive {id}`,
  `character_captive {id}`, `ruler_is {faction, character}`, `ruler_trait {faction?, trait}`,
  `ruler_house {faction?, house}`, `ruler_age_between {faction?, min, max}`, `year_between {from, to}`,
  `province_unrest_above {province?, amount}`, `province_besieged {province?, by?}`,
  `province_coastal {province?}`, `treasury_above {faction?, amount}`, `provinces_below {faction?, count}`,
  `religion_is {faction?, religion}`, `schism {active}`, `not_fired {event}`, `fired {event}`,
  `season {season}`, `any_of {conditions[]}`. Un `faction`/`province` absent vaut la faction/province visée.
- Effets (`type`) : `treasury {faction?, amount}`, `unrest|health|wealth|devastation {province?, amount}`,
  `population {province?, percent}` (`province` : `"all"` = toutes les provinces contrôlées par la faction
  qui décide, un `prov_`, ou absent = province visée sinon toutes), `prestige|piety {character?, faction?,
  amount}` (`character` : `"ruler"` défaut, `"heir"` ou un `chr_`), `papal_favor {faction?, amount}`,
  `opinion {faction, towards?, amount, reason, duration?}` (attitude de `faction` envers `towards`,
  défaut : la faction qui décide ; 20 tours), `declare_war {a, b}`, `peace {a, b}` (trêve de 5 ans),
  `add_trait {character?, faction?, trait}`, `kill_character {id, faction?}`,
  `spawn_army {faction?, province?, units[]}`, `claim {faction?, kind: throne|province, target}`,
  `loyalty {vassal?, amount}` (défaut : tous les vassaux), `plague_wave {from_year, to_year}` ; F1 :
  `capture_character {id, faction?, captor}`, `release_character {id, faction?, ransom?}` (rançon versée
  au geôlier), `schedule_event {event, delay}` (l'événement part `delay` tours plus tard pour la même
  faction et province, conditions vérifiées alors), `marry {a, b}` (mariage historique).

**Chargement.** Erreur (`DataError::InvalidEvent`) : 0 ou plus de 3 options, historique sans `date`,
aléatoire sans `mean_time_to_happen`/`chance_permille`, `mean_time_to_happen` nul, `schedule_event` de
délai nul ou visant l'événement lui-même ; clé ou `type` inconnu
→ `DataError::Json` (`deny_unknown_fields`). Avertissement (`Warning`, entité `evt_…`) pour tout id
inconnu dans une condition ou un effet : la simulation l'ignore ; événement `chained` jamais programmé. Test pytest
`tools/tests/test_events_schema.py` : chaque fichier valide le schéma JSON.

**Simulation.** `CampaignState.chronicle` (`ChronicleState`, `#[serde(default)]`, `STATE_VERSION`
inchangé = 4) : `fired_events` (historiques déjà déclenchés), `pending_decisions[{id, event, faction,
province?, options[], expires_turn}]`, `next_decision_id`, `plague_wave? {start_turn, duration}`,
`disabled` (aucun nouvel événement).
Phase de fin de tour après la religion, avant la population :
1. décisions expirées (`expires_turn` ≤ tour) : premier choix appliqué, journal « (délai écoulé) » ;
2. historiques non déclenchés dont la date est atteinte et `until_year` non dépassé, si les conditions
   tiennent pour au moins une cible (sinon rien : l'histoire diverge ; passé `until_year`, jamais) ;
3. aléatoires : un tirage par faction vivante et par événement (RNG de campagne, déterministe), au plus
   un aléatoire par faction et par tour ; les aléatoires des IA restent hors journal ;
4. vague de peste : les provinces triées par latitude de leur capitale (sud d'abord) sont frappées une
   fois chacune sur `(to_year − from_year) × 4` tours (Peste noire : 12) : santé −30, population −15 à
   −30 % (tirage), mécontentement +20 pour chaque classe ; un événement `plague` par tour.
Une IA applique aussitôt le choix de plus fort `ai_weight` (égalité : RNG) ; le joueur reçoit une
décision de 2 tours (événement `chronicle` « Chronique : titre. »). Les effets passent tous par
`chronicle::apply_effect`.

| Méthode `CampaignSim` | Retour |
|---|---|
| `get_pending_decisions()` | décisions du joueur : `[{id, event, title, text, historical, options[{index, text, effects_text}], expires_in, province, province_name}]` ; `effects_text` = une ligne française par effet ; `expires_in` = fins de tour restantes (1 = ce tour-ci) |
| `choose_event_option(decision, option)` | `{ok, error}` (équivaut à l'ordre `choose_event_option`) |
| `set_chronicle_enabled(enabled)` | active/suspend les nouveaux événements (`chronicle.disabled` ; mesures isolées comme l'étape économie du smoke) ; les décisions en cours expirent toujours et la peste continue |

Ordre (`submit_order`) : `choose_event_option{decision, option}` ; refus « décision inconnue » /
« choix invalide ». Événement de journal M10 : `chronicle` (le résultat d'un choix par ordre ouvre le
journal du tour suivant, comme les ordres M5).

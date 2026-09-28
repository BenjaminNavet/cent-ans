# FE4b — grappe C4 « Allemagne du Sud » (Souabe, Franconie, Tyrol)

Branche `feat/fe4b-empire`, dans le worktree `game_project-fe4b`. Travail de données uniquement
(pas de commit, pas de `cargo test`, pas de pipeline géo — laissés à l'orchestrateur).

## État : TERMINÉ (données créées/modifiées, validation JSON Schema manuelle OK)

## Ce qui a été fait

8 nouvelles provinces créées ou reconfigurées, 7 nouveaux titres, 7 nouvelles factions jouables,
8 nouveaux personnages, 7 fichiers de colonies.

### 1. Wurtemberg (nouvelle, subdivisée de Souabe)
- `data/provinces/prov_wurttemberg.json`, `data/titles/tit_wurttemberg.json` (county),
  `data/factions/fac_wurttemberg.json`, `data/characters/chr_ulrich_iv_de_wurtemberg.json`
  (Ulrich IV, comte 1325-1344), `data/settlements/prov_wurttemberg.json` (Stuttgart, château de
  Wirtemberg, Tübingen, Waiblingen).

### 2. Bade (nouvelle, subdivisée de Souabe)
- `data/provinces/prov_baden.json`, `data/titles/tit_baden.json`, `data/factions/fac_baden.json`
  (`government: principality`), `data/characters/chr_rodolphe_hesso_de_bade.json`,
  `data/settlements/prov_baden.json` (Baden-Baden, Pforzheim, Hohenbaden, Ettlingen).
- **Simplification assumée et documentée** : le margraviat de Bade est réellement partagé en
  1337 entre plusieurs branches cousines (Baden-Baden, Baden-Pforzheim/Durlach,
  Baden-Eberstein/Hachberg). Représenté ici par une seule faction sous « Rodolphe Hesso de
  Bade » (branche de Baden-Baden). `uncertain: true` posé sur `holder_1337` du titre et sur le
  blason de la faction ; dates de vie du personnage approximatives (`uncertain: true`).

### 3. Souabe résiduelle (fichier existant `prov_swabia.json`)
- `owner` : `fac_empire` → `fac_austria` (pas de nouvelle faction, pas de nouveau titre — la
  province reste directement dans `tit_empire.de_jure_provinces`, comme demandé).
  Population réduite en conséquence des deux détachements ci-dessus (429520/72800/13000/4680 →
  219520/45800/7500/2780, split exact). Description réécrite : villes libres d'empire + terres
  éparses habsbourgeoises (« Outre-Forêt »/Vorderösterreich).
- `fac_austria.json` **non modifié** (pas de liste de provinces dans les factions, confirmé en
  lisant son schéma avant de commencer — seul le champ `owner` de la province suffit).

### 4. Franconie → Évêché de Wurtzbourg (fichier existant `prov_franconia.json`, id conservé)
- `name.display` : « Franconie » → « Évêché de Wurtzbourg » ; `owner` : `fac_empire` →
  `fac_wurzburg`. Nouveau titre `tit_wurzburg.json` (principauté ecclésiastique, `rank: county`),
  faction `fac_wurzburg.json` (`government: theocracy`), personnage
  `chr_otto_von_wolfskeel.json` (prince-évêque 1333-1345, duc de Franconie de droit depuis 1168).
  Population réduite (voir ci-dessous, Nuremberg et Bamberg détachés).
  `data/settlements/prov_franconia.json` : gardé Wurtzbourg (passé en `city`, capitale),
  Rothenburg, Ebrach, Volkach ; ajouté « Forteresse de Marienberg » (résidence fortifiée des
  évêques) ; retiré Nuremberg, Bamberg, Kronach (déplacés vers leurs provinces propres).

### 5. Nuremberg (nouvelle, subdivisée de Franconie)
- `data/provinces/prov_nuremberg.json`, `data/titles/tit_nuremberg.json`,
  `data/factions/fac_nuremberg.json`, `data/characters/chr_jean_ii_de_hohenzollern.json`
  (burgrave 1332-1357, ancêtre des Hohenzollern de Brandebourg/Prusse — mentionné en
  description). `data/settlements/prov_nuremberg.json` : ville libre de Nuremberg (`city`),
  château de Cadolzburg (résidence du burgrave), Kronach (forteresse épiscopale de Bamberg,
  enclave), Fürth. Description de la province documente la dualité ville
  libre/burgraviat (deux pouvoirs distincts sur le même site, la ville échappant en pratique à
  l'autorité du burgrave).

### 6. Bamberg (nouvelle, subdivisée de Franconie)
- `data/provinces/prov_bamberg.json`, `data/titles/tit_bamberg.json` (principauté
  ecclésiastique), `data/factions/fac_bamberg.json` (`government: theocracy`),
  `data/characters/chr_leopold_d_egloffstein.json`, `data/settlements/prov_bamberg.json`
  (Bamberg ville, Forchheim, Altenburg, abbaye de Banz).
- **Incertitude signalée** : je n'ai pas pu vérifier « Léopold d'Egloffstein, prince-évêque de
  Bamberg 1335-1343 » par une source externe (pas d'accès web dans ce worktree). Retenu tel
  quel avec `uncertain: true` sur `holder_1337` et sur le personnage (dates et identité).
  **Point ouvert pour relecture historienne** de l'orchestrateur ou d'un lot ultérieur.

### 7. Tyrol (fichier existant `prov_tirol.json`, id conservé)
- `name.display` : « Tyrol et Trente » → « Tyrol » ; `owner` : `fac_empire` → `fac_tirol`.
  Nouveau titre `tit_tirol.json`, faction `fac_tirol.json` (`succession_law:
  cognatic_primogeniture`, cohérent avec l'héritage par Marguerite Maultasch), personnage
  `chr_jean_henri_de_luxembourg.json` (comte par mariage 1330, frère cadet de Charles IV,
  fils de Jean de Bohême). Ajout de `chr_marguerite_de_tyrol.json` (comtesse suo jure,
  Marguerite Maultasch) pour éviter une référence `family.spouses` orpheline — non demandé
  explicitement par la spec mais nécessaire à la cohérence des données (elle est la détentrice
  historique réelle du comté, Jean-Henri n'étant comte que par ce mariage).
  `relations` de `fac_tirol` inclut une entrée `marriage_tie` vers `fac_bohemia` comme demandé.
  Capitale gardée à Innsbruck (déjà présente) ; description signale que Merano/Meran était la
  résidence historique antérieure.
  `data/settlements/prov_tirol.json` : fichier existant conservé tel quel (6 colonies déjà
  présentes, largement suffisant, aucune ne concernait Trente).

### 8. Trente (nouvelle, subdivisée de Tyrol)
- `data/provinces/prov_trent.json`, `data/titles/tit_trent.json` (principauté ecclésiastique),
  `data/factions/fac_trent.json` (`government: theocracy`), `data/characters/chr_nicolas_de_brno.json`,
  `data/settlements/prov_trent.json` (Trente ville, château du Buonconsiglio, Rovereto, Pergine).
- **Incertitude signalée** : Nicolas de Brno (Mikuláš z Brna) n'est attesté comme évêque de
  Trente qu'à partir de 1338 selon mes connaissances ; faute de pouvoir vérifier son
  prédécesseur exact pour 1337 (pas d'accès web), je l'ai retenu quand même avec
  `uncertain: true` partout (titre, faction, personnage) et une note explicite dans chaque
  description. **Point ouvert pour relecture historienne.**
- `culture: cul_german` par simplification malgré la population mixte germano-italienne réelle
  de Trente — documenté dans la description de la province, comme demandé par la spec.

## Titre racine `tit_empire.json`

Fichier partagé entre toutes les grappes FE4b (édité en parallèle par d'autres agents pendant ce
lot — `git status` montre des modifications concurrentes sur `prov_cologne`, `prov_mainz`, etc.
et sur `tit_empire.json` lui-même). J'ai relu le fichier juste avant chaque édition et retiré
uniquement `prov_franconia` et `prov_tirol` de `de_jure_provinces` (désormais couvertes par
`tit_wurzburg` et `tit_tirol`). `prov_swabia` reste dans `tit_empire.de_jure_provinces` (pas de
titre propre créé, conformément à la spec). Un premier `Edit` a échoué avec « File has been
modified since read » à cause d'une écriture concurrente d'un autre agent ; relu et réédité avec
succès. **Point d'attention pour l'orchestrateur** : ce fichier est un point de collision entre
grappes, à vérifier attentivement à la fusion.

## Validation

Script Python jetable avec `jsonschema` (Draft202012Validator, résolution manuelle de
`common.schema.json` et `faction.schema.json` en local, pas d'accès réseau) : les 8 provinces,
8 titres (dont `tit_empire`), 7 factions, 8 personnages et 7 fichiers de colonies modifiés ou
créés passent tous la validation de schéma. Pas de `cargo test` ni de pipeline géo lancés
(hors mandat).

## Vérification des collisions d'identifiants

`grep -h '"blazon"' data/titles/*.json data/factions/*.json` avant écriture : aucun doublon
avec mes 7 nouveaux blasons. Recherche des ids de colonies (Stuttgart, Tübingen, Baden-Baden,
Pforzheim, Cadolzburg, Marienberg, Forchheim, Trente, Rovereto, etc.) : aucune collision avec les
142 fichiers de colonies existants avant ce lot.

## Points ouverts / incertitudes assumées (résumé pour relecture)

1. Baden : margraviat réellement divisé entre cousins en 1337, simplifié en une seule faction
   (branche de Baden-Baden), `uncertain: true` posé.
2. Bamberg : titulaire et dates de l'évêque (Léopold d'Egloffstein, 1335-1343) non vérifiables
   sans accès web dans ce worktree — `uncertain: true` posé partout.
3. Trente : Nicolas de Brno attesté surtout à partir de 1338, retenu par approximation pour
   1337 faute de mieux — `uncertain: true` posé partout.
4. Tyrol : ajout non demandé explicitement mais nécessaire d'un personnage
   `chr_marguerite_de_tyrol` (comtesse suo jure) pour ne pas laisser de référence
   `family.spouses` orpheline dans `chr_jean_henri_de_luxembourg`.
5. `tit_empire.json` est un fichier partagé entre grappes ; mes seules modifications sont le
   retrait de `prov_franconia` et `prov_tirol` de `de_jure_provinces`. À vérifier à la fusion
   avec les autres grappes (C1/C2/C3/C5) qui l'éditent aussi.
6. Aucun fichier hors de mon périmètre touché : ni `data/heraldry/houses.json`, ni
   `data/ui/front_end.json`, ni `data/portraits/archetypes.json`, ni `fac_austria.json`.

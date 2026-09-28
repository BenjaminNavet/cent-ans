# FE4b — grappe C5 « Pays-Bas impériaux » (Namur, Liège, Utrecht, Lorraine)

## État : TERMINÉ (côté données de cette grappe)

Branche `feat/fe4b-empire`, travail effectué dans `/Users/jean_hubert/dev/game_project-fe4b`.
Aucun commit, aucun test, aucune régénération géo effectués (mandat de l'orchestrateur).

## Ce qui a été fait

Pas de nouvelle province : réaffectation de 4 provinces existantes (`prov_namur`, `prov_liege`,
`prov_utrecht`, `prov_lorraine`) de `fac_empire` à leurs nouvelles factions vassales, toutes
jure vassales directes de `tit_empire`.

- `data/provinces/prov_namur.json`, `prov_liege.json`, `prov_utrecht.json`, `prov_lorraine.json` :
  seul le champ `owner` modifié (`fac_empire` → `fac_namur`/`fac_liege`/`fac_utrecht`/`fac_lorraine`).
  Rien d'autre touché (culture, description, population, colonies déjà correctes et sourcées).
- 4 titres créés (`data/titles/`) : `tit_namur` (county), `tit_liege` (county, principauté
  épiscopale), `tit_utrecht` (county, principauté épiscopale), `tit_lorraine` (duchy). Tous
  `de_jure_liege: "tit_empire"`.
- 4 factions créées (`data/factions/`) : `fac_namur` (government: county), `fac_liege`
  (theocracy), `fac_utrecht` (theocracy), `fac_lorraine` (duchy). Toutes `suzerain: "fac_empire"`.
  `fac_liege`, `fac_utrecht`, `fac_lorraine` marquées `playable: true` ; `fac_namur` `playable:
  false` (petit comté, gouverné par un mineur en 1337).
- 5 personnages créés (`data/characters/`) : `chr_philippe_iii_de_namur` (ruler),
  `chr_guillaume_i_de_namur` (heir, status: minor, 13 ans), `chr_adolphe_de_la_marck` (ruler,
  Liège), `chr_jean_iii_de_diest` (ruler, Utrecht), `chr_raoul_de_lorraine` (ruler, Lorraine).
- Aucune colonie créée : `data/settlements/prov_{namur,liege,utrecht,lorraine}.json` existaient
  déjà avec 7 à 10 colonies réelles sourcées chacune (largement au-dessus du minimum de 3) — non
  modifiés.
- `tit_empire.json` non modifié par moi : `prov_utrecht` y figurait déjà dans
  `de_jure_provinces` (avec `prov_namur`, `prov_liege`, `prov_lorraine`). Le fichier apparaît
  modifié dans `git status` du fait d'un autre agent en parallèle (grappe C-autre), pas de mon
  fait.
- `fac_empire.json` non touché : ce fichier ne porte pas de liste de provinces (le schéma
  `faction.schema.json` n'a pas de champ `provinces`), la « récupération » des 4 provinces se
  fait entièrement via le champ `owner` des fichiers de province.

## Validation

Validation manuelle par script `jsonschema` (schémas `province`, `title`, `faction`,
`character`, avec résolution locale des `$ref` vers `common.schema.json`/`faction.schema.json`
pour contourner l'absence de réseau) : les 17 fichiers créés/modifiés de cette grappe passent
tous. Pas de `cargo test`, pas de `cargo fmt`/`clippy`, pas de pipeline géo (hors mandat).

## Points historiques vérifiés (WebSearch)

- **Namur** : au printemps 1337 (date de `holder_1337`), le comte-marquis régnant est
  **Philippe III de Namur** (marquis depuis 1336, comte depuis 1335), maison de Dampierre,
  cadette de Flandre. Il meurt à Famagouste (Chypre) courant 1337 — la description de province
  préexistante l'indiquait déjà — et son frère cadet **Guillaume Ier le Riche** (13 ans en 1337)
  lui succède la même année ; modélisé comme `ruler` (Philippe III) + `heir` `status: minor`
  (Guillaume). Blason : lion de Flandre (or/sable) brisé d'un bâton de gueules — distinct du
  blason plein de Flandre (`tit_flanders`/`fac_flanders`, déjà utilisé), sourcé (thèse UCLouvain
  sur l'héraldique namuroise).
- **Liège** : Adolphe de la Marck, prince-évêque 1313–1344, grand bâtisseur et fondateur du
  studium liégeois — conforme au mandat. `government: theocracy`, `succession_law: elective`.
  Blason : colonne d'argent sur gueules (armes traditionnelles du perron liégeois) — pas de
  doublon trouvé dans `data/titles`/`data/factions`.
- **Utrecht** : Jean (III) de Diest, évêque 1322–1340 (confirmé par recherche web), choisi sur
  suggestion du comte de Hollande (union personnelle avec Hainaut, `fac_hainaut` — `fac_holland`
  n'existe pas comme faction séparée) et du duc de Gueldre (`fac_guelders`), sacré à Avignon par
  Jean XXII contre le choix initial du chapitre. Blason : croix d'argent sur gueules du Sticht
  (attestée dès 1291, sceau de Jean de Sierck), différenciée du blason identique de Savoie
  (`tit_savoy`) par un écusson en abîme aux armes de la maison de Diest (pratique attestée depuis
  Jean de Diest en 1328).
- **Lorraine** : Raoul de Lorraine, duc depuis 1329, mort à Crécy le 26/08/1346 aux côtés de
  Philippe VI — conforme au mandat. Double allégeance modélisée : `suzerain: fac_empire` +
  relation `alliance` avec `fac_france` dans `relations`, décrite dans la description. `rank:
  duchy` (pas comté). Blason : bande de gueules aux trois alérions d'argent sur or — armes
  historiques attestées des ducs de Lorraine, non dupliquées ailleurs.

## Incertitudes assumées

- Dates de naissance de Philippe III de Namur (~1319), Guillaume Ier de Namur (~1324), Adolphe
  de la Marck (~1288) et Raoul de Lorraine (~1320) : approximatives, marquées `uncertain: true`
  quand le champ le permet (naissance/mort). Pas de contradiction de sources trouvée sur les
  successions elles-mêmes.
- Utrecht : blason composite (croix + écusson Diest) reconstruit à partir de deux faits
  historiques distincts (croix du Sticht + pratique de brisure personnelle des évêques depuis
  1328), pas une attestation directe d'un blason unique pour 1337 — jugé raisonnable plutôt que
  de dupliquer le blason de Savoie sans distinction.
- Cultures `cul_french` conservées pour Namur et Liège (déjà présentes dans les fichiers de
  province avant ce lot) plutôt que `cul_german` suggéré en option par le brief : la Wallonie
  mosane du XIVe siècle est de langue et de culture françaises/romanes (wallon), choix documenté
  ici plutôt qu'appliqué sans le signaler.

## Aucune collision constatée
Vérification par `grep -h '"blazon"' data/titles/*.json data/factions/*.json` avant écriture :
aucun blazon dupliqué introduit (Utrecht différencié de Savoie comme indiqué plus haut).

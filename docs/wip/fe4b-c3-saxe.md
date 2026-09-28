# FE4b — grappe C3 « Saxe / Brandebourg / Thuringe »

Sous-tâche du chantier FE4b (Empire et Pays-Bas), branche `feat/fe4b-empire`. Données uniquement,
pas de commit, pas de `cargo test`, pas de pipeline géo (fait par l'agent orchestrateur).

## État : TERMINÉ (validation schéma manuelle faite, reste géo/tests/commit à l'orchestrateur)

## Fichiers créés

- Provinces : `data/provinces/prov_mecklenburg.json`, `prov_thuringia.json`, `prov_brunswick.json`.
- Titres : `data/titles/tit_brandenburg.json`, `tit_mecklenburg.json`, `tit_meissen.json`,
  `tit_brunswick.json`, `tit_bremen.json`.
- Factions : `data/factions/fac_brandenburg.json`, `fac_mecklenburg.json`, `fac_meissen.json`,
  `fac_brunswick.json`, `fac_bremen.json`.
- Personnages : `data/characters/chr_louis_v_de_baviere.json`,
  `chr_albert_ii_de_mecklembourg.json`, `chr_frederic_ii_de_misnie.json`, `chr_otto_le_doux.json`,
  `chr_burchard_grelle.json`.
- Colonies : `data/settlements/prov_mecklenburg.json`, `prov_thuringia.json`, `prov_brunswick.json`.

## Fichiers modifiés

- `data/provinces/prov_brandenburg.json` : renommé « Brandebourg » (retrait de Mecklembourg),
  `owner` → `fac_brandenburg`, plus côtier (ports transférés au Mecklembourg), population et
  ressources réduites en conséquence.
- `data/provinces/prov_meissen.json` : renommé « Misnie » (retrait de Thuringe), `owner` →
  `fac_meissen`, population réduite.
- `data/provinces/prov_lower_saxony.json` : renommé « Archevêché de Brême » (retrait de
  Brunswick-Lunebourg), `owner` → `fac_bremen`, capitale changée de Brunswick à Brême, population
  réduite.
- `data/settlements/prov_brandenburg.json` : Stralsund, Rostock, Wismar déplacés vers
  `prov_mecklenburg.json` (4 colonies restantes : Berlin, Francfort-sur-l'Oder, Spandau,
  Brandebourg-sur-la-Havel).
- `data/settlements/prov_meissen.json` : Erfurt déplacé vers `prov_thuringia.json` (6 colonies
  restantes : Meissen, Freiberg, Stolpen, Leipzig, Altzella, Dresde).
- `data/settlements/prov_lower_saxony.json` : Brunswick et Lunebourg déplacés vers
  `prov_brunswick.json` ; ajout de Stade (bailliage brêmois sur l'Elbe inférieure) ; 5 colonies
  restantes (Brême, Hildesheim, Goslar, Loccum, Stade, Verden — 6 en fait).
- `data/titles/tit_empire.json` : retrait de `prov_brandenburg`, `prov_meissen`,
  `prov_lower_saxony` de `de_jure_provinces` (la liste avait déjà été modifiée entre-temps par une
  autre grappe parallèle qui a retiré Cologne/Mayence/Palatinat/Trèves/Westphalie ; édition faite
  sur l'état courant, sans toucher aux autres entrées).
- `data/factions/fac_empire.json` : **non modifié** — ce fichier ne liste pas les provinces
  détenues (la propriété est portée par le champ `owner` de chaque province, déjà mis à jour).

## Choix de conception assumés

- **`government` de Brandebourg et de Misnie** : mis à `"county"` (rang réel de margraviat/
  margraviat+landgraviat) alors que le **titre** (`tit_brandenburg`, `tit_meissen`) est en
  `rank: "duchy"`, par convention politique pour les aligner sur les autres grands électorats de
  l'Empire, comme demandé par le mandat. Documenté dans la description de chaque faction/titre.
- **Stralsund** rattachée à `prov_mecklenburg` (et non à une province de Poméranie, hors mandat de
  ce lot) : signalé `uncertain`/note explicite dans la description de la province et de la colonie,
  car Stralsund est historiquement un fief des ducs de Poméranie-Rügen, pas du Mecklembourg.
- **Hildesheim et Goslar** restent dans `prov_lower_saxony` (devenue « Archevêché de Brême ») bien
  qu'ils ne soient pas historiquement des possessions de l'archevêque de Brême (Hildesheim est un
  évêché distinct, Goslar une ville libre d'Empire) : simplification cartographique assumée et
  signalée dans la description de la province et du titre, faute de mandat pour créer une
  province Hildesheim/Goslar séparée dans ce lot.
- **Brunswick-Lunebourg** modélisé sous un seul titulaire (Otton le Doux, branche de Lunebourg)
  alors que le duché welf est en réalité fragmenté entre plusieurs princes cadets en 1337 —
  signalé explicitement `uncertain`/note dans le titre, la faction et la description de la
  province, conformément au mandat.
- **Burchard Grelle** (archevêque de Brême) : nom et dates de règne (1327-1344) non retrouvés avec
  certitude absolue par mes propres moyens de recherche (pas d'accès web dans cette session) ;
  repris tels que fournis dans le mandat, mais marqués `uncertain: true` sur la date de mort et la
  prise de fonction, avec note explicite. **Point à vérifier par l'orchestrateur ou une passe de
  relecture historienne si une source fiable est disponible.**
- **Blasons** : Brandebourg (aigle rouge sur argent, distincte de l'aigle noire impériale),
  Mecklembourg (tête de buffle noire, meuble historique attesté), Misnie (crancelin de Wettin,
  attesté), Brunswick (deux léopards d'or sur gueules, différenciés d'une bordure componée pour
  éviter la collision exacte avec les armes normandes/anglaises déjà présentes dans le jeu — signalé
  `uncertain`), Brême (clef d'argent et de gueules, plausible mais non confirmée — signalé
  `uncertain`). Vérifié par `grep` qu'aucun blason n'est un doublon exact d'un blason existant.
- **Population** : chaque nouvelle province reçoit une estimation indépendante (pas de retrait des
  effectifs de la province mère), conformément à la pratique déjà actée dans FE4a (léger
  surcomptage assumé, documenté dans le champ `population.note` de chaque province).

## Validation faite

- Script Python jetable (`jsonschema` 4.26.0 + `referencing`) validant les 27 fichiers créés/
  modifiés contre `data/schemas/{province,title,faction,character,settlement}.schema.json` : tout
  au vert.
- Vérification manuelle d'absence de collision d'id (`grep` sur les colonies, titres, factions).

## Points ouverts pour l'orchestrateur

- Pipeline géo (`cent-ans geo provinces`/`settlements`/`horizon`/`navgrid`) à relancer pour les 3
  nouvelles provinces (mecklenburg, thuringia, brunswick) et régénérer les cartes/aperçus/reliefs.
- `cargo test -p data-model` et `pytest` non lancés ici (hors mandat de cette sous-tâche) : à
  vérifier par l'orchestrateur après fusion des 5 grappes C.
- Dates de Burchard Grelle à confirmer si une source fiable est disponible.
- Simplification Hildesheim/Goslar dans l'archevêché de Brême et Stralsund dans le Mecklembourg :
  à revoir si une passe ultérieure crée des provinces dédiées (Pomeranie, Hildesheim) ailleurs
  dans le lot FE4b ou un lot suivant.

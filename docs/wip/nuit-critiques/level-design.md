# Critique level design (carte, champs de bataille, positions de départ)

Rôle : level designer. Lecture ciblée de `data/` (provinces, battle_maps, rules, crossings, compose). Aucun client lancé, aucune capture. Constats à vérifier avant correctif : certains reposent sur un échantillon de fichiers.

## Constats

### 1. Seulement 3 champs de bataille historiques, tous en terrain `plains`
- Problème : `data/battle_maps/` ne contient que Crécy, Poitiers et Azincourt, tous `"terrain": "plains"`. Pourtant `data/rules/battle_terrain.json` définit 9 profils (heath, bocage, hills, mountains, forest, marsh, steppe, desert). La variété tactique de la guerre de Cent Ans (Sluys, Auray, Cocherel, Patay, Formigny, Castillon, Verneuil) n'est pas jouable en bataille historique.
- Preuve : `data/battle_maps/*.json` (champ `terrain`) ; `data/rules/battle_terrain.json` (clés `terrains`).
- Gravité : majeur.
- Correctif : ajouter 3 cartes à profil différent : Auray (1364, heath/landes), Castillon (1453, camp retranché + rivière, plains/marais), Formigny (1450, bocage). Chaque carte reprend le gabarit de `crecy.json`.
- Effort : M par carte.

### 2. Les trois cartes partagent le même gabarit de champ
- Problème : `field` identique (2400 × 1600, `zone_depth_m` 450, `max_regiments_per_side` 160 ; Azincourt 2000 × 1600). Les deux armées partent en face à face sur une ligne droite, ce qui donne trois variantes d'un même duel frontal. Il manque des rôles de terrain asymétriques (défenseur sur crête, flanc ouvert, armée en marche surprise, bataille de rencontre).
- Preuve : `data/battle_maps/{crecy,poitiers,azincourt}.json`, clé `field` et `armies`.
- Gravité : mineur.
- Correctif : introduire un champ `deployment` (frontal, embuscade de flanc, ligne en L). Poitiers a déjà la réserve du captal ; généraliser ce principe.
- Effort : M.

### 3. Gwynedd a pour capitale Carmarthen
- Problème : Carmarthen est dans le sud du pays de Galles (Deheubarth/Dyfed). La capitale de Gwynedd est Caernarfon/Aberffraw. La province `mountains` « Gwynedd » est donc ancrée au mauvais endroit, et la couche de rendu (nom au zoom, pose du lieu) place un nom nord-gallois sur le sud.
- Preuve : `data/provinces/prov_gwynedd.json` (`capital_city`), région `galles`.
- Gravité : majeur (erreur de fidélité visible).
- Correctif : soit renommer la province « Galles » ou « Pays de Galles », soit remplacer la capitale par Caernarfon et le seed Voronoi, soit découper en deux provinces (Gwynedd / Deheubarth). Régénérer avec `geo provinces`.
- Effort : S (renommage) ou M (découpe).

### 4. L'Angleterre tient Édimbourg (Lothian) et toute la côte galloise dès 1337
- Problème : `prov_lothian` est à `fac_england` en 1337. Le royaume d'Écosse (`fac_scotland`) n'a que `prov_fife` dans la liste de la région `ecosse`. En 1337 Édouard Balliol tenait le sud-est écossais de façon précaire (1333-1338) et Édimbourg changeait de main. Position défendable comme « seconde guerre d'indépendance » mais non documentée dans le fichier, et elle réduit l'Écosse à une unique province. L'Angleterre pèse 10 provinces dont Dublin, Man, Lothian : trop de fronts pour un joueur anglais, trop peu de profondeur pour l'Écosse.
- Preuve : `data/provinces/prov_lothian.json` (`owner`), `prov_fife.json`.
- Gravité : majeur (équilibre et fidélité).
- Correctif : donner Lothian à Écosse (ou à un fac Balliol rebelle) et documenter le choix dans un ADR ; ajouter 1 à 2 provinces écossaises (Mar, Moray) si la grille de provinces le permet.
- Effort : S (propriétaire), M (nouvelles provinces).

### 5. Île de Man classée en région `irlande` avec un poids Voronoi de 0,35
- Problème : l'île de Man est mal classée. Son poids Voronoi minuscule (0,35) la réduit à un confetti, un territoire illisible à l'échelle de la carte et un piège de pathfinding (province de 2-3 pixels).
- Preuve : `data/provinces/prov_man.json` (`region`, `geo.voronoi_weight`).
- Gravité : mineur.
- Correctif : fixer `region` sur `mer_d_irlande` ou `ecosse`, relever le poids à au moins 0,6 ou fusionner avec une côte voisine (Cumberland).
- Effort : S.

### 6. Aquitaine : une seule grande province anglaise, pas de profondeur défensive
- Problème : toute la Gascogne anglaise est `prov_guyenne` (poids 1,3, capitale Bordeaux). La région `aquitaine` ne compte que 6 provinces. La guerre de Cent Ans se joue sur Bordelais / Périgord / Agenais / Bazadais ; ici la perte de Bordeaux équivaut à la perte de toute la position anglaise. Pas de manœuvre graduelle, pas de bastides, pas d'enjeu à Castillon ou La Réole.
- Preuve : `data/provinces/prov_guyenne.json` ; région `aquitaine` (guyenne, armagnac, agenais, saintonge, bearn, albret).
- Gravité : majeur (jouabilité du front principal).
- Correctif : scinder en Bordelais, Périgord, Bazadais/Landes ; chaque capitale (Bordeaux, Périgueux, Bazas) devient une cible.
- Effort : L (provinces, colonies, rééquilibrage de `starting_fit`).

### 7. Équilibre de départ : trésoreries et armées de départ très inégales
- Problème : `starting_fit.json` donne p. ex. Brabant 18 772, Flandre 19 728, Gênes 25 408, Bohême 16 632, Autriche 13 384, quand la plupart des petites factions ont 700-1 800. Le Brabant et la Flandre, deux provinces, dépassent largement la plupart des grands acteurs de la guerre. De plus `starting_armies.json` donne la même composition aux seules trois factions principales (France, Angleterre, Bourgogne) et `default_army` à toutes les autres : Écosse, Castille, Aragon, Flandre partent avec chevaliers + hommes d'armes + arbalétriers, sans identité (pas de longbow gallois, pas de jinetes).
- Preuve : `data/rules/starting_fit.json` (`treasuries`), `data/rules/starting_armies.json` (`factions`, `default_army`).
- Gravité : majeur (équilibre).
- Correctif : plafonner la trésorerie initiale (p. ex. 8 000) ou la baser sur des provinces et non sur les marchés ; ajouter 8 à 10 compositions régionales (Écosse : schiltrons/piquiers, Castille : genetours, Aragon : almogavars, Flandre : milices à pied).
- Effort : M.

### 8. Cols : aucun côté franco-anglais, pas de chaîne pyrénéenne ni jurassienne complète
- Problème : sur 169 passages, 27 « pass » : alpins, pyrénéens (Perthus, Somport, Roncevaux, Puymorens) et balkaniques. Les défilés du Massif central (Velay, Cévennes), du Jura (Saint-Cergue, Jougne) et les gués de la Manche/Somme sont absents hors Blanchetaque. Les gués ne sont que 20. Le Somme a 7 passages, la Garonne 5, la Dordogne 4, la Loire 13 : cohérent, mais pas de passages à Gascogne autour de Castillon-la-Bataille alors que « Bac de Castillon » existe (nom ambigu avec Castillon-sur-Dordogne), pas de pont de l'Authie (Ponthieu).
- Preuve : `data/map/crossings.json` (`crossings`, types).
- Gravité : mineur.
- Correctif : compléter les passages le long des itinéraires de chevauchée (Chevauchée de 1346, Poitiers 1356) : Authie, Orne, Dives, Vienne, Clain ; marquer Jougne et Saint-Cergue.
- Effort : M.

### 9. Épingles d'auteur vides : la carte composée n'est pas retouchée là où la lisibilité en dépend
- Problème : `data/map/compose/pins.json` contient `"pins": []`. Le cartogramme déforme la carte pour donner de la place aux régions denses ; sans épingle, les villes jumelées (Calais/Boulogne, Bruges/Gand, Paris/Saint-Denis) peuvent se chevaucher au zoom de campagne. L'éditeur existe mais n'a pas servi.
- Preuve : `data/map/compose/pins.json` ; `docs/editeur-carte.md`.
- Gravité : mineur.
- Correctif : poser 10 à 15 épingles sur les paires de lieux serrées, après un export des distances entre lieux composés (< 12 px). Passe de 30 minutes dans l'éditeur.
- Effort : S.

### 10. Seules 8 villes ont des vues de lieux de repère
- Problème : `data/landmarks/` (8 fichiers) et `data/landmarks_v2/` (8 fichiers) couvrent Avignon, Bordeaux, Bruges, Calais, Le Caire, Londres, Paris, Rouen (+ Orléans en v2). Sur 215 colonies, Reims (sacre), Caen, Troyes, La Rochelle, Toulouse, Gand, Calais fortifié sont reconnaissables sur une carte ancienne et manquent. Le Caire est présent mais pas Constantinople ni Gênes, alors que ces dernières sont des pôles commerciaux majeurs du jeu.
- Preuve : `ls data/landmarks data/landmarks_v2`.
- Gravité : mineur.
- Correctif : ajouter les silhouettes Reims, Caen, La Rochelle, Gand, Constantinople, Gênes (même pipeline).
- Effort : M.

### 11. Pas de bataille de passage nommée pour les gués historiques
- Problème : `river_crossings.json` modélise des batailles de passage génériques (facteurs par type de pont/gué), mais aucune bataille historique n'utilise ce mécanisme alors que Blanchetaque (1346) est le cas d'école, et Stirling Bridge / Cocherel / Verneuil suivent. Le générique et l'historique ne se parlent pas.
- Preuve : `data/rules/river_crossings.json`, `data/map/crossings.json` (« Gué de Blanchetaque »), `data/battle_maps/`.
- Gravité : mineur.
- Correctif : ajouter `data/battle_maps/blanchetaque.json` (gué avec marée, rive opposée tenue) comme premier scénario de passage.
- Effort : M.

### 12. `neighbors` vide dans 208 des 213 fichiers de provinces
- Problème : les fichiers sources `data/provinces/*.json` ont `"neighbors": []` pour presque toutes les provinces. L'adjacence réelle est calculée par `geo provinces` (`provinces.py` l. 720) et écrite dans `provinces.geojson`. Un lecteur de données ou un outil externe qui lit le fichier source croit les provinces isolées ; risque de divergence si la géométrie change sans régénération.
- Preuve : `data/provinces/prov_ponthieu.json` (`neighbors: []`) ; `tools/cent_ans_tools/geo/provinces.py` l. 720.
- Gravité : mineur.
- Correctif : retirer la clé des sources (elle est trompeuse) ou la remplir à la génération, avec un test qui compare.
- Effort : S.

## Top 3

1. **Constat 6** : scinder la Guyenne en 3 provinces ; c'est le front principal du jeu et il tient en un seul nœud.
2. **Constat 1** : élargir les champs de bataille historiques à d'autres terrains (Auray en landes, Castillon, Formigny en bocage) ; la variété tactique est un acquis de la bibliothèque de terrains, non exploité.
3. **Constats 3 et 4** : corriger Gwynedd/Carmarthen et le statut de Lothian ; erreurs de fidélité visibles en deux minutes, peu coûteuses à régler.

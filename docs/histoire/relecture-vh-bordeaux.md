# Relecture historique — Bordeaux vers 1340 (ville 1:1, VH8 / RS-G) — 28 septembre 2026

Relecture indépendante de `data/landmarks_v2/bordeaux.json` (ADR 0078, `docs/landmarks-v2.md` ;
suivi d'auteur `docs/archive/chantiers.md`) : faits marqués `probable` ou `hypothetical` d'abord,
puis dates clés (1337-1453) et gabarits. Source principale relue **en texte intégral** : L. Drouyn,
*Bordeaux vers 1450* (1874, domaine public, texte OCR d'archive.org), que l'auteur n'avait utilisé
que par extraits. Les sources sous droits (Wikipédia, Bordeaux Métropole, Boutoulle) servent aux
faits seulement ; rien n'en est extrait (`extracted: false`). Positions vérifiées en projetant en
EPSG:3035 (pyproj, `lonlat_to_local`) les objets OSM correspondants (rues nommées par Drouyn,
portes, églises ; cache Overpass `tools/geo/raw/osm/bordeaux_*.json`).

Verdicts : **confirmé** (conforme aux sources), **corrigé** (une source fiable le contredit : le JSON
a été modifié), **incertain** (sources muettes ou divergentes : donnée gardée, note mise à jour).

## Bilan

| Verdict | Nombre |
|---|---|
| Confirmé (dont 5 confirmés sur la date mais incertains sur un point) | 20 |
| Corrigé (y compris les ajouts de portes situées par Drouyn) | 10 |
| Incertain (laissé `probable` / `hypothetical`, expliqué) | 4 |

**Point signalé : deux murs à ≈ 20-30 m l'un de l'autre à l'ouest.** C'est **historique**. Drouyn
place le mur romain dans « les maisons qui bordent le côté oriental de la rue des Remparts » et le mur
du XIVe s. « dans celles qui séparent la rue des Remparts de la rue Bouffard » ; il note que « le mur
du XIVe siècle […] était très rapproché du mur romain » (porte Dijeaux reportée de 10-12 m vers
l'ouest), qu'une tour du XIVe s. se dressait « presque en face » de la tour romaine du Dragon, et cite
une baillette de 1495 portant sur « des terrains compris entre les deux murs de la ville, près la porte
Basse ». Mesure sur OSM : rue des Remparts de (−102, 354) à (−78, 105), rue Bouffard de (−131, 347)
à (−118, 106) ; les deux murs du fichier sont à 19-32 m l'un de l'autre, de part et d'autre de la rue.
Test ajouté.

**Correction majeure** : au sud-ouest, entre la tour carrée de l'angle Sainte-Eulalie et la porte
Saint-Julien, le mur suivait « l'intérieur des maisons du sud de la rue de la Miséricorde », la place
Henri-IV en diagonale et « le côté méridional de la rue Saincric, autrefois rue du Chemin-de-Ronde »,
puis coupait la rue Henri-IV en biais avant la place intérieure d'Aquitaine (Drouyn, p. 10 et 41).
Le fichier suivait la rue Henri-IV, qui longe le « grand bastion » extérieur : le tracé est remonté
de 25 à 55 m vers le nord (quartier `accrue_sud` recalé).

## Sources

| Abr. | Référence |
|---|---|
| Drouyn | L. Drouyn, *Bordeaux vers 1450. Description topographique*, Archives municipales de Bordeaux, t. complémentaire, 1874, texte intégral : https://archive.org/details/bordeauxvers145000drou (p. 4-5 castrum ; p. 9-12 tracé et liste des 21 portes ; p. 36-43 deuxième et troisième enceintes, tours ; notices des portes Basse, Saint-André, Dijeaux, Saint-Symphorien, Audeyole) |
| WP Remparts | Wikipédia, « Remparts et portes de ville de Bordeaux » (bibliographie : Desforges ; Drouyn) |
| WP monuments | Wikipédia, « Cathédrale Saint-André de Bordeaux », « Tour Pey-Berland », « Basilique Saint-Michel de Bordeaux », « Église Saint-Pierre de Bordeaux », « Église Sainte-Eulalie de Bordeaux », « Palais de l'Ombrière », « Abbaye Sainte-Croix de Bordeaux » |
| Boutoulle | F. Boutoulle, « Enceintes, tours, palais et castrum à Bordeaux… », *Revue archéologique de Bordeaux* XCIV, 2003 (source d'auteur, non relue ici) |
| BM | Bordeaux Métropole, carte des patrimoines, « Ancien rempart de Bordeaux et jardin » (source d'auteur) |
| OSM | OpenStreetMap : rues des Remparts, Bouffard, Monbazon, place Rohan, rue du Hâ, Jean-Burguet, Henri-IV, de la Miséricorde, Saincric, Tombe-l'Oly, Sainte-Catherine, du Mirail, de Candale, Bigot, du Port, du Moulin, Beyssac, des Faures, de la Fusterie, de la Rousselle, des Portanets, Ausone, du Chai-des-Farines, de la Porte-Saint-Jean, du Pas-Saint-Georges, des Trois-Conils, cours du Chapeau-Rouge ; nœuds et emprises : porte Dijeaux, porte d'Aquitaine, porte de Bourgogne, porte Cailhau, Grosse Cloche, églises |

## Tableau des faits

| # | Fait restitué (avant) | Verdict | Source | Changement fait |
|---|---|---|---|---|
| 1 | Deux murs parallèles à 20-30 m à l'ouest (castrum et troisième enceinte) | **Confirmé** | Drouyn (p. 39-40, 81-85 : mur du XIVe s. entre les rues Bouffard et des Remparts, mur romain côté est de la rue des Remparts ; « terrains compris entre les deux murs », 1495) ; OSM | Note de l'enceinte ; test ajouté |
| 2 | Front sud-ouest le long de la rue Henri-IV | **Corrigé** | Drouyn (p. 10 : « bord méridional de la rue de la Miséricorde, à travers la place Henry IV, sur le côté méridional de la rue Saincric, autrefois rue du Chemin-de-Ronde » ; p. 41 : le grand bastion « longeait le bord extérieur de la rue Henry IV ») ; OSM (rue de la Miséricorde (2, −533) → (132, −605), rue Saincric (141, −631) → (155, −640)) | Points (−30, −586) (76, −642) (126, −666) (200, −705) remplacés par (−46, −557) (2, −552) (108, −612) (125, −622) (155, −652) (200, −692) ; polygone `accrue_sud` recalé |
| 3 | Grosse tour carrée à l'angle Jean-Burguet / Henri-IV | **Confirmé** | Drouyn (p. 40 : base du mur du XIVe s. retrouvée « près du coin des rues Henry IV et Jean-Burguet ») ; OSM (carrefour (−86, −551), sommet du fichier (−88, −553)) | — |
| 4 | Porte Sainte-Eulalie (−60, −570), hypothétique | **Corrigé** (position) | Drouyn (liste : « au sud de l'église Sainte-Eulalie » ; déplacée en 1603) | Déplacée sur le nouveau tracé à (−46, −557), au droit de l'église |
| 5 | Porte Saint-Symphorien omise (« position inconnue ») | **Corrigé** (ajout) | Drouyn (liste : « vers le milieu de la rue des Remparts » ; notice : près du ruisseau de La Mothe, « portail de Sainct-Seurian » en 1321 et 1381) | Porte ajoutée à (−108, 230) |
| 6 | Porte du Far au débouché de la rue du Hâ (probable) | **Confirmé** | Drouyn (« au bout occidental de la rue du Ha ») ; OSM (bout de la rue du Hâ (−81, −173), porte (−96, −172)) | — |
| 7 | Porte Dijeaux 10-12 m à l'ouest de la porta Jovia | **Confirmé** | Drouyn ; WP Remparts ; OSM (nœud « Porte Dijeaux » (−113, 355), porte du fichier (−108, 355)) | — |
| 8 | Porte Saint-Julien (place de la Victoire) | **Confirmé** | Drouyn (bout sud de la rue Sainte-Catherine) ; WP Remparts (porte de 1302 remplacée par la porte d'Aquitaine) ; OSM (porte d'Aquitaine (302, −769)) | — |
| 9 | Porte du Mirail (522, −745), hypothétique | **Corrigé** (position) | Drouyn (« au sud de la rue du Mirail et à l'est de la porte d'Aquitaine ») ; OSM (rue du Mirail finit en (460, −639)) | Déplacée à (462, −745) ; reste hypothétique |
| 10 | Grand portail Sainte-Croix (site des abattoirs) | **Incertain** | Drouyn (« sur l'emplacement de l'Abattoir ») ; l'abattoir du XIXe s. n'a pas été localisé ; une rue du Portail (OSM) mène vers l'abbaye, ce qui n'est pas concluant | — |
| 11 | Mur de Garonne : rues Ausone, du Chai-des-Farines, Rousselle | **Confirmé** | Drouyn (p. 10 : « entre le quai et la rue de La Rousselle », côté ouest « des rues Ausone et du Chai-des-Farines ») ; OSM (écart de 5 à 15 m) | Note |
| 12 | Porte du Caillau à l'ouest de la porte Cailhau de 1493-1496 | **Confirmé** | Drouyn (« au bas de la place du Palais, un peu à l'ouest de la porte actuelle ») ; WP Remparts (porte actuelle 1493-1496, plus près de la Garonne ; mur de ≈ 2 m × 8-10 m) ; OSM (porte Cailhau (748, 20) ; porte du fichier (727, 12)) | — |
| 13 | Porte des Salinières (895, −298) | **Corrigé** (léger) | WP Remparts (porte de Bourgogne « à l'emplacement de l'ancienne porte médiévale des Salinières ») ; Drouyn (« à l'ouest de la porte Bourgogne, non loin de l'entrée de la rue de la Rousselle ») ; OSM (porte de Bourgogne (889, −273)) | (882, −280) |
| 14 | Porte de la Grave (1029, −472), hypothétique | **Corrigé** | Drouyn (« au bout oriental de la rue des Faures ») ; OSM | Placée où la rue des Faures coupe le mur : (1001, −442) |
| 15 | Porte Sainte-Croix de la Rivière (1268, −795), hypothétique | **Corrigé** | Drouyn (« au bout inférieur de la rue du Port ») ; OSM | Placée où la rue du Port coupe le mur : (1253, −778) |
| 16 | Portes de Beyssac, du Peugue (Brisson), de l'Orme-de-Casse omises | **Corrigé** (ajouts) | Drouyn (« au bout de la rue de Beysac » ; « à l'embouchure du Peugue dans la rue Ausone » ; « au bas du cours du Chapeau-Rouge ») ; OSM (rue Beyssac, rue Ausone, cours du Chapeau-Rouge) | Portes ajoutées à (1139, −626), (749, −70), (611, 470) ; positions à ± 20 m (rues prolongées jusqu'aux quais du XVIIIe s.). Graphie « Orme » : l'OCR donne « Orne » et « Ome » |
| 17 | Portes d'Audeyole et du Redge omises | **Incertain** | Drouyn (près des colonnes rostrales ; bains des Quinconces) : front nord bouleversé par le château Trompette | Toujours omises (tracé hypothétique) |
| 18 | Portes Saint-Jean, Saint-Pey, des Paux, Pey-Miqueu | **Confirmé** (positions probables) | Drouyn (bout du cours d'Alsace-et-Lorraine ; derrière Saint-Pierre ; au nord de la fontaine des Grâces ; entre la rue des Faures et la porte Bourgogne) ; OSM (rue de la Porte-Saint-Jean, Saint-Pierre, rue Saint-Rémi) | — |
| 19 | Castrum 725 × 450 m, tracé de Drouyn | **Confirmé** | Drouyn (p. 4-5 : « 725 mètres de long … et 450 de large », murs de 2,80 à 4 m) ; WP Remparts (740 × 480 m, 278-290) | — |
| 20 | Portes du castrum (Médoque, Basse, Cadène, Bégueyre) | **Confirmé** | Drouyn ; WP Remparts (Bégueyre au carrefour rue du Pas-Saint-Georges / cours d'Alsace ; porte Basse rue de Cheverus, démolie en 1803) ; OSM (rue du Pas-Saint-Georges (474, −61), rue Porte-Basse (176, −48)) | — |
| 21 | Porte Saint-André (castrum, ouest) absente | **Corrigé** (ajout) | Drouyn (porte Saint-André, dite porte Basse en 1404, au bout occidental de la rue des Trois-Conils ; pieds-droits retrouvés) ; OSM (rue des Trois-Conils finit en (−78, 105)) | Porte ajoutée à (−66, 105) |
| 22 | Deuxième enceinte 1205-1255, doublée, restituée en un mur | **Confirmé** / **incertain** (dates) | Drouyn (deux murs à ≈ 10 m, deux fossés, contre le mur romain à l'ouest de la porte Basse) ; WP Remparts (1227-1245 ; porte des Carmes « ouverte en 1307 ») ; Boutoulle (1205-1255, source d'auteur) | — (dates sans effet : pas de `from_year`) |
| 23 | Troisième enceinte achevée en 1340 (1302-1327) | **Confirmé** / **incertain** (début) | Drouyn (ordonnance de 1302) ; BM (convention de 1302-1304, travaux jusqu'aux années 1320) ; WP Remparts (« en 1327 » pour la décision, mais porte Saint-Julien « datant de 1302 ») | Note : les deux datations |
| 24 | Fossés en eau du côté de la terre seulement | **Confirmé** | Drouyn (p. 43, d'après Vinet : « excepté du côté de la rivière ») | — |
| 25 | Cathédrale : 124 m dans œuvre, nef unique de 18 m, voûtes 23 m, pas de portail ouest | **Confirmé** | WP monuments (déambulatoire vers 1280 raccordé à la nef vers 1330, chœur et chapelles du XIVe s., voûtes effondrées en 1427) ; Drouyn (castrum « à quelques mètres » de la façade ouest ; fichier : 5 m) | — |
| 26 | Tour Pey-Berland à partir de 1440, 47 m | **Confirmé** (date) / **incertain** (hauteur) | WP (première pierre la veille des nones d'octobre 1440, chantier jusqu'en 1500, terrasse à ≈ 50 m) | Description : hauteur atteinte en 1440-1453 inconnue, gabarit de la tour achevée |
| 27 | Saint-Michel : ancienne église jusqu'en 1429, église-halle à partir de 1430 | **Confirmé** (dates) / **incertain** (ancienne église) | WP (chantier « aux environs de 1330 », ralenti ; « l'église-halle sort de terre en 1430 » ; clocher 1472-1492) | — (emplacement et gabarit de l'ancienne église toujours inconnus) |
| 28 | Saint-Pierre reconstruite à partir de 1358 | **Confirmé** | WP (reconstruction du milieu du XIVe à la fin du XVe s. ; port et place Saint-Pierre cités en 1262) ; Drouyn (mur romain coupant l'église) | Description : gabarit de 1340 restitué |
| 29 | Sainte-Eulalie : chevet gothique du XIVe s., angle de rempart | **Confirmé** | WP (consacrée sous Guillaume le Templier, bas-côtés au XIIIe s., chevet au XIVe s. ; « intégrée à la Ville et marque un angle de rempart ») | — |
| 30 | Sainte-Croix : façade à un clocher, emprise | **Confirmé** / gabarit **corrigé** (description) | WP (nef de cinq travées de 39 m, abside polygonale de 15,30 m, second clocher d'Abadie 1861-1865) | Description (abside polygonale, ronde dans le gabarit) |
| 31 | Ombrière : site attesté, disposition restituée | **Incertain** (`probable` gardé) | WP (tour de l'Arbalesteyre, 1080, 18 × 14 m, 40 m en avant du rempart) ; OSM (place du Palais (672, −9) ; château du fichier (660, −18)) | — |
| 32 | Saint-Seurin, Saint-Éloi, Saint-Rémi, Saint-Projet : positions OSM | **Confirmé** (positions) / gabarits **incertains** | OSM (emprises à ≤ 1 m des centres du fichier) | — |
| 33 | Couvent des Jacobins sous les allées de Tourny | **Incertain** (`hypothetical` gardé) | WP (rasé en 1678 pour le glacis du château Trompette) ; position dans les allées non vérifiée | — |
| 34 | Château Trompette et fort du Hâ absents | **Confirmé** | Drouyn ; WP (1453-1456) | — |

## Ce qui reste ouvert

- **Front nord** (allées d'Orléans - Quinconces) et portes d'Audeyole, du Redge : connus par les
  textes seuls ; tracé hypothétique. À reprendre avec l'*Atlas historique des villes de France —
  Bordeaux* (2009) et le plan de Lattre (1733) ou les plans manuscrits cités par Drouyn (contrôle
  humain : plans non réutilisables).
- **Pointe sud-est** (fort Louis - Garonne) et grand portail Sainte-Croix : l'emplacement de l'abattoir
  du XIXe s. reste à localiser.
- **Front sud-ouest corrigé** : le bastion triangulaire avant la place Henri-IV et le grand bastion
  à redans (probablement postérieurs) ne sont pas figurés ; le tracé nouveau est `probable` (rues du
  XIXe s. décrites par Drouyn, maisons « du sud » supposées à 15-20 m de l'axe).
- **Tours** : Drouyn donne les diamètres de plusieurs tours du nord-ouest (4,5 à 12 m) ; le gabarit
  du moteur n'a qu'un rayon et un espacement par enceinte.
- Couvents (Cordeliers, Carmes, Augustins), Puy-Paulin, hôpital Saint-Julien : toujours omis (pas de
  position relue ; Drouyn en donne une partie, à exploiter dans un lot suivant).
- Hauteurs de toutes les églises en 1340, hauteur de Pey-Berland en 1453 : inconnues.

## Vérifications

- `uv run --project tools pytest -q tools/tests/test_landmarks_v2.py
  tools/tests/test_landmarks_v2_bordeaux.py` : réussis (test des enceintes complété : porte
  Saint-Symphorien, écart de 15-40 m entre les deux murs de l'ouest, tracé de la rue de la
  Miséricorde).
- Modifications faites par script (`landmarks_v2.write_city`, format du fichier inchangé).

# Relecture historique — Avignon vers 1340 (ville 1:1, VH8) — 28 septembre 2026

Relecture indépendante de `data/landmarks_v2/avignon.json` (ADR 0078, `docs/landmarks-v2.md` ;
suivi de l'auteur : `docs/archive/chantiers.md`). Méthode : faits marqués `probable` ou
`hypothetical` d'abord, puis dates clés (1337-1453) et gabarits. Sources sérieuses (Inventaire
général PACA, POP, étude universitaire de J.-M. Poisson, notice d'Archeodunum, Centre des
monuments nationaux), Wikipédia en appoint quand elle cite ses sources. Faits seulement : rien
n'est extrait des sources sous droits (`extracted: false`). Positions vérifiées en projetant en
EPSG:3035 (`lonlat_to_local`) les objets OSM correspondants (Overpass, 28 septembre 2026).

## Bilan

| Verdict | Nombre |
|---|---|
| Confirmé | 20 |
| Corrigé (y compris les lignes 3, 14 et 26, confirmées sur la date mais corrigées sur un point) | 8 |
| Incertain (laissé `probable` / `hypothetical`, expliqué) | 10 |

**Correction majeure** : la **tour de la Campane** (1339-1340) n'est pas sur le front est à côté
de la tour des Anges mais à l'**angle nord-ouest du palais Vieux**, contre la cathédrale ; elle
mesure 45 m (non 40) sur 14,94 × 12,80 m. Déplacée de (70, 21) à (−4, 52), certitude relevée à
`probable`. Autres corrections : chapelle des Templiers (1273-1281, 23,80 × 8 m, non « fondée en
1256, 30 × 12 m »), tour Philippe-le-Bel rehaussée à 27 m (non 30), noms du XIVe s. de deux portes
des remparts, date des remparts (1357-1373) et reconstruction du front nord-est vers 1364.

## Sources principales

| Abr. | Référence |
|---|---|
| Poisson 2004 | J.-M. Poisson, « Le palais des papes d'Avignon : structures défensives et références symboliques », dans P. Boucheron et J. Chiffoleau (dir.), *Les Palais dans la ville*, PUL, 2004. <https://books.openedition.org/pul/19345> |
| Archeodunum | Archeodunum, « Avignon, palais des Papes : plancher de la tour de la Campane ». <https://www.archeodunum.com/Notice_site/avignon-387/> |
| Inv. pont | Inventaire général PACA, dossier IA84000918, « Pont Saint-Bénezet… Châtelet et tour Philippe-le-Bel » (S. Delétoille, I. Havard, B. Decrock, 2012-2016). <https://dossiersinventaire.maregionsud.fr/dossier/IA84000918> |
| POP coll. | Inventaire général Occitanie, IA30001059, collégiale Notre-Dame de Villeneuve. <https://pop.culture.gouv.fr/notice/merimee/IA30001059> |
| CMN | Centre des monuments nationaux, fort Saint-André (fiche de visite ; acte de 1362). <https://www.fort-saint-andre.fr/> |
| ACM | Avignon cité millénaire, « Rue Victor Hugo » (couvent des Dominicains). <https://www.avignoncitemillenaire.com/rue-victor-hugo> |
| WP Remparts | Wikipédia, « Remparts d'Avignon » (d'après M. Maynègre 1991, V. Serdon 2021, Viollet-le-Duc 1856). |
| WP | Autres articles de Wikipédia FR cités : Palais des papes, Pont Saint-Bénézet, Tour Philippe-le-Bel, Cathédrale Notre-Dame-des-Doms, Liste des églises d'Avignon, Chapelle des Templiers d'Avignon, Petit Palais, Collégiale Notre-Dame de Villeneuve, Fort Saint-André. |
| Rolland 1989 | F. Rolland, « Un mur oublié : le rempart du XIIIe siècle à Avignon », *Archéologie médiévale* 19, 1989 (Persée, résumé seulement). |
| OSM | OpenStreetMap : contour du palais (relation 1253164, voie 83769181), églises, tours, portes, rues. |

## Tableau des faits

| # | Fait restitué (avant) | Verdict | Source | Changement fait |
|---|---|---|---|---|
| 1 | Tour de la Campane : front est à côté des Anges (70, 21), 12 m, 40 m, `hypothetical` | **Corrigé** | Poisson 2004 (« angle nord-ouest du palais vieux », 1339) ; Archeodunum (« angle nord-ouest du Palais à côté de la cathédrale », 45 m, murs de 3 m, 14,94 × 12,80 m, cinq niveaux) | `at` (−4, 52) dans l'angle nord-ouest du contour OSM ; `size` 14, `height` 45 ; `certainty` → `probable` ; test ajouté |
| 2 | Tour de la Campane achevée en 1340 | **Incertain** | WP (commencée en août 1339) ; Poisson (1339) ; achèvement en juin 1340 non recoupé | `from_year` 1340 gardé, réserve dans la description |
| 3 | Tour des Anges : 1335, 46 m, section 18 m | **Confirmé** (section **corrigée**) | Poisson 2004 (1335, carré de 17,50 m, 46 m, murs de 3 m, « côté oriental, très en avant vers le sud ») | `size` 17,5 ; description |
| 4 | Tour de Trouillas : 52 m, achevée en 1346 | **Confirmé** | Poisson 2004 (1341-1345, 52 m, murs de 4,50 m, extrémité nord) ; WP (commencée en août 1341, incendiée en 1354) | Description |
| 5 | Tour de la Garde-Robe 1342 | **Confirmé** | WP (juillet 1342) | — |
| 6 | Tour Saint-Laurent 1356 | **Confirmé** | WP (1353-1356) ; Poisson (1353, angle sud) | Description (1353-1356) |
| 7 | Palais Vieux 1335-1342, palais Neuf 17 juillet 1342-1351 | **Confirmé** | WP ; Poisson | — |
| 8 | Répartition des ailes et positions des tours Anges, Garde-Robe, Trouillas | **Incertain** | Poisson (positions relatives seulement) ; aucun plan d'érudit consulté | Inchangé ; tours Saint-Jean (1338), Gâche, Cardinal Blanc non figurées |
| 9 | Pont : 1177-1185, ≈ 920 m, 4 m, rebâti 1234-1237 | **Confirmé** | Inv. pont (1177-1185 env., 920 × 4 m, détruit en 1226 puis rebâti en pierre) ; WP (1234-1237) | Note |
| 10 | Pont : 22 arches | **Incertain** | WP (22) contre Inv. pont (« vingt-trois ») | 22 gardées, écart noté (le test porte sur 22) |
| 11 | Chapelle sur la troisième pile | **Confirmé** | Inv. pont (« en 1184, une chapelle est aménagée à l'intérieur d'une pile (la troisième en venant d'Avignon) ») ; OSM : chapelle Saint-Bénézet à (−173, 336), soit 0,225 de la première travée (`chapel_at` 0,22) | — |
| 12 | Pas de châtelet côté Avignon en 1340 | **Confirmé** | Inv. pont (Châtelet « élevé à partir de 1414 ») | Note |
| 13 | Tracé coudé et partage 8 + 14 arches | **Incertain** | Aucune source de tracé | Inchangé (`hypothetical` sur la seconde travée) |
| 14 | Tour Philippe-le-Bel : premier étage 1303 ; rehaussée vers 1360, 30 m | **Confirmé** (dates) / **corrigé** (hauteur) | Inv. pont (1293-1307, 27 m, tour de guet 7 m plus haute) ; WP (premier étage 1303, étage gothique vers 1360) | Phase 1360 : `height` 27 ; descriptions |
| 15 | Enceinte du XIIIe s. 1234-1248, tracé par les rues | **Confirmé** | WP Remparts d'après Maynègre (1234-1237, 30-40 m en avant du mur rasé en 1226, achevée en 1248 ; rues Trois-Colombes, Campane, Philonarde, Lices, Henri-Fabre, Joseph-Vernet, Grande-Fusterie) ; OSM (portails au débouché des rues homonymes ; « Planet du Portail Peint » à 1 m) | Note précisée |
| 16 | Portail Magnanen sur le mur du XIIIe s. | **Confirmé** | webiane ; WP Remparts : la « porte Magnanen » des remparts est une percée de 1902 (à ne pas confondre) | Note |
| 17 | Front nord du mur du XIIIe s., 5 portails sur 11, fossé, hauteur | **Incertain** | Rolland 1989 (tour semi-circulaire, béal de la Sorgue ancien fossé ; article complet non lu) | Inchangé |
| 18 | Remparts `from_year` 1357 | **Confirmé** | WP Remparts d'après Serdon 2021 (« élevés de 1357 à 1373 ») ; travaux engagés en 1355 (Innocent VI) | Note : 1357-1373 |
| 19 | Front nord-est des remparts | **Corrigé** (note) | WP Remparts (Viollet-le-Duc) : porte Saint-Lazare emportée par la crue de la Durance de 1358, rebâtie vers 1364 avec les remparts jusqu'au rocher des Doms | Note (anneau non daté par tronçon : limite du format) |
| 20 | Noms des sept portes | **Corrigé** (deux noms) | WP Remparts : porte de l'Oulle = porte Saint-Jacques (XIVe s.) ; porte de la Ligne = porte Aurose (XIVe s.) ; autres noms anciens non datés (porte Aiguière, portail Saint-Antoine, portail Imbert neuf, portail des Miracles) | « Porte Saint-Jacques (de l'Oulle) », « Porte Aurose (de la Ligne) » ; test mis à jour |
| 21 | Porte de la Ligne à la position OSM | **Incertain** | WP Remparts : déplacée d'une centaine de mètres en 1757 | Réserve dans la note |
| 22 | Nombre de portes et de tours | **Incertain** | 12 portes et 36 tours (Serdon) contre 7 portes, 35 grandes tours et 50 petites (Maynègre) | Note ; espacement de 110 m gardé |
| 23 | Notre-Dame-des-Doms : clocher écroulé le 27 janvier 1405 | **Confirmé** | WP (« le mardi 27 janvier 1405… ») ; chapelle de Jean XXII 1316 | — ; date de reconstruction du clocher toujours manquante |
| 24 | Petit Palais : Frédol 1317, Arnaud de Via, Benoît XII 1336 | **Confirmé** | WP Petit Palais (Frédol 1317 ; achat par Arnaud de Via le 11 juin 1323 ; Benoît XII le 5 juin 1336) | — |
| 25 | Chapelle des Templiers : fondation 1256, vers 1305, 30 × 12 m | **Corrigé** | WP Chapelle des Templiers (présence dès 1174 ; chapelle 1273-1281, Notre-Dame-de-Bethléem en 1281 ; Hospitaliers vers 1330, confirmé en 1342 ; 23,80 × 8 m, quatre travées, chevet plat) | `length_m` 24, `width_m` 8, nom et description |
| 26 | Dominicains : 1312-1336, « quatorze chapelles » | **Confirmé** (dates) / **corrigé** (détail) | ACM (1224 ; 1312-1336 ; neuf travées, dix-huit chapelles ; sacristie 1345, cloître 1348, clocher 1356 ; démolition jusqu'en 1852) ; Liste des églises | Description ; emprise gardée (bordait les rues Victor-Hugo et Saint-André, position exacte non sourcée) |
| 27 | Carmes (1267, église 1319-1520) | **Confirmé** | Liste des églises ; OSM à 1 m | — |
| 28 | Augustins (1261, église rebâtie à partir de 1345) | **Confirmé** | Liste des églises (« bâtie en 1345 par Jean Laugier ») | — |
| 29 | Cordeliers (1226, église achevée en 1350) | **Confirmé** (dates) / **incertain** (emprise) | Liste des églises ; OSM : tour et chapelle à 36 m du centre restitué | — |
| 30 | Clarisses (1239, église 1316) ; Saint-Laurent (918) ; Saint-Agricol (collégiale 1321) ; Saint-Pierre (collégiale 1358) | **Confirmé** | Liste des églises ; positions OSM à ≤ 2 m (Saint-Agricol, Saint-Pierre, Sainte-Claire) | — |
| 31 | Saint-Didier gothique 1356-1359 ; Saint-Martial 1363 ; Célestins 1395 | **Confirmé** | Liste des églises (1356-1359 ; 1363-1388 ; 1393 puis 1395-1455) ; OSM (Saint-Didier, Saint-Martial à ≤ 1 m) | — |
| 32 | Collégiale de Villeneuve : collégiale 1336, cloître 1350, clocher 1362 (attribués à POP) | **Corrigé** | POP coll. (chapelle consacrée le 1er juin 1333 ; porche est autorisé le 1er avril 1334 ; dès 1336 cloître et réorientation ; « deuxième clocher » non daté) ; WP (collégiale fondée le 7 août 1333) | Descriptions et `use` de la source POP corrigés |
| 33 | Clocher de la collégiale en 1362, à l'ouest | **Incertain** | WP d'après Petit Patrimoine (tour-beffroi de 1362 **à l'est**) ; absent de POP | Phase 1362 → `hypothetical` ; la tour reste à l'ouest (le gabarit `church` ne sait pas la placer au chevet) |
| 34 | Fort Saint-André vers 1362 | **Confirmé** | CMN (années 1360, acte de 1362) ; WP (Jean le Bon, achevé sous Charles V, Jean de Loubières) | Description |
| 35 | Chartreuse 1356 ; abbaye Saint-André (Xe s.) | **Confirmé** | WP ; OSM (Chartreuse à 18 m, abbaye à 1 m) | — |
| 36 | Positions OSM des monuments | **Confirmé** | OSM projeté : Notre-Dame-des-Doms (28, 83) contre (28, 82) ; Saint-Didier (−75, −430) ; Templiers (−269, −236) contre (−270, −236) ; tour Philippe-le-Bel (−737, 916) ; fort (−401, 1740) contre (−408, 1745) ; collégiale (−778, 1442) | — |
| 37 | Dominicains (−520, −360) | **Incertain** | ACM (rues Victor-Hugo et Saint-André) : le point est au carrefour de ces rues, l'église de 80 m peut être de part et d'autre | Inchangé |
| 38 | Hauteurs des églises et du palais Neuf | **Incertain** | Aucune source de hauteur | Inchangé (restituées) |

## Ce qui reste ouvert

- **Plan du palais** : répartition des ailes du palais Vieux et du palais Neuf, sections des tours
  Garde-Robe et Saint-Laurent ; tours Saint-Jean (1338), de la Gâche et du Cardinal Blanc non
  figurées. À reprendre avec S. Gagnière (*Le palais des papes d'Avignon*) ou D. Vingtain
  (*Avignon, le palais des papes*, 1998), non consultés.
- **Mur du XIIIe s.** : front nord, six portails non situés, gabarits ; l'article complet de
  Rolland (1989) et le livre de Maynègre (1991) n'ont pas été lus.
- **Remparts** : l'anneau est daté d'un bloc (1357) alors que le front nord-est est rebâti vers
  1364 ; position médiévale des portes de la Ligne (avant 1757) et Saint-Roch (avant 1865).
- **Pont** : 22 ou 23 arches ; tracé coudé et île restitués.
- **Collégiale de Villeneuve** : la tour-beffroi est à l'est (chevet) ; le gabarit `church` ne
  propose que `west` ou `crossing` (point de format).
- **Clocher de Notre-Dame-des-Doms** après 1405 : date de reconstruction non trouvée.
- Livrées cardinalices, Sainte-Madeleine, Notre-Dame-la-Principale, Saint-Geniès, Saint-Étienne,
  Sainte-Catherine (1354) : toujours non figurées faute de position sourcée.

## Vérifications

- `uv run --project tools pytest -q tools/tests/test_landmarks_v2.py tools/tests/test_landmarks_v2_avignon.py`
  : réussis (test des portes mis à jour pour les noms du XIVe s. ; test de la Campane ajouté :
  45 m, au nord-ouest de la tour des Anges).
- Fichier réécrit par `write_city` (format inchangé, aller-retour vérifié octet pour octet avant
  modification).

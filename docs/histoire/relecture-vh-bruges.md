# Relecture historique — Bruges vers 1340 (ville 1:1, VH8 / RS-G) — 28 septembre 2026

Relecture indépendante de `data/landmarks_v2/bruges.json` (ADR 0078, `docs/landmarks-v2.md` ;
suivi d'auteur `docs/archive/chantiers.md`) : faits marqués `probable` ou `hypothetical` d'abord,
puis dates clés (1337-1453) et gabarits. Sources relues en texte intégral : notices de l'Inventaris
Onroerend Erfgoed (CC BY 4.0), fiches du *Beheerplan Vesten* de la ville de Bruges (2024),
Wikipédia NL en appoint. Faits seulement, rien d'extrait (`extracted: false`). Positions vérifiées en
projetant en EPSG:3035 (pyproj, `lonlat_to_local`) les objets OSM (portes, églises, Burg, Markt,
Provinciaal Hof, rues des portes ; cache Overpass `tools/geo/raw/osm/bruges_*.json`).

Verdicts : **confirmé** (conforme aux sources), **corrigé** (une source fiable le contredit : le JSON
a été modifié), **incertain** (sources muettes ou divergentes : donnée gardée, note mise à jour).

## Bilan

| Verdict | Nombre |
|---|---|
| Confirmé (dont 5 confirmés sur la date mais incertains sur un point) | 20 |
| Corrigé | 3 |
| Incertain (laissé `probable` / `hypothetical`, expliqué) | 4 |

Le fichier était déjà très bien sourcé (Inventaire, Beheerplan) ; les dates des portes, du beffroi,
des halles, de l'hôtel de ville, de Saint-Donatien et de la Poertoren sont confirmées.

**Corrections** :
1. **Nef de Saint-Sauveur** : l'Inventaire (objet 29716) date la nef gothique actuelle du « premier
   quart du XVe siècle » ; elle n'est donc plus rendue dès 1359 mais à partir de 1425 (de 1359 à 1424,
   chœur seul).
2. **Ezelpoort** : le bâtiment de 1369, qui se dresse dans le fossé intérieur (Inventaire), est à
   62 m du point de la porte et à 42 m de l'axe du fossé restitué le long de la Koningin
   Elisabethlaan ; porte, levée et fossé recalés sur lui.
3. **Waterhalle** : hauteur restituée 18 m portée à 30 m (estimation publiée, Wikipédia NL).

## Sources

| Abr. | Référence |
|---|---|
| IOE 302193 | Inventaris Onroerend Erfgoed, « Brugse geplantsoeneerde stadsomwalling », https://inventaris.onroerenderfgoed.be/erfgoedobjecten/302193 |
| IOE 14697 | Inventaris Onroerend Erfgoed, thème « Brugge - middeleeuwse stadsuitbreiding zuid » (Gilté, Vanwalleghem, Van Vlaenderen, 2004), https://inventaris.onroerenderfgoed.be/themas/14697 |
| IOE 11630 | Inventaris Onroerend Erfgoed, objet de protection « Poertoren », https://inventaris.onroerenderfgoed.be/aanduidingsobjecten/11630 (résumé du moteur de recherche) |
| IOE 29457 | « Belfort-Hal » (historique et étude dendrochronologique ; Debonne 2015) |
| IOE 29238, 29235, 82359, 29716 | « Stadhuis », « Sint-Donaaskerk », « Onze-Lieve-Vrouwekerk », « Sint-Salvatorskathedraal » |
| Beheerplan | Stad Brugge, *Beheerplan Vesten — Bijlage 1 : Inventarisatie*, 2024 (fiches Ezelpoort, Kruispoort, Gentpoort, Smedenpoort, Poertoren, IJskelder) |
| WP NL | Wikipédia NL : « Belfort van Brugge », « Waterhalle », « Het Steen (Brugge) », « Boeveriepoort », « Brugse stadspoorten », « Dampoort (Brugge) » (bibliographie : Duclos 1910 ; Ryckaert, *Stedenatlas* 1991) |
| OSM | OpenStreetMap : Gentpoort, Kruispoort, Smedenpoort, Ezelpoort, Poertoren, Oud Waterhuis, Belfort, Stadhuis, Heilig-Bloedbasiliek, Provinciaal Hof, Burg, Markt, site archéologique de Saint-Donatien, églises, Begijnhof ; Ezelstraat, Smedenstraat, Katelijnestraat, Boeveriestraat, Langestraat, Koningin Elisabethlaan, Potterierei, Lange Rei |

## Tableau des faits

| # | Fait restitué (avant) | Verdict | Source | Changement fait |
|---|---|---|---|---|
| 1 | Enceinte de 1297-1302 : levée de terre, double fossé, palissade | **Confirmé** | IOE 302193 (1297-1300, « dubbele gracht en een aarden wal met een houten palissade ») ; Beheerplan (1297-1302, bases 1297-1301) | — |
| 2 | Démantèlement vers 1328, passages bas en 1340 | **Confirmé** | IOE 302193 et 14697 (« omstreeks 1328 », après 1305) ; Beheerplan (« ca. 1328 », traité d'Athis-sur-Orge) | — |
| 3 | Neuf portes en 1297 (six portes + trois portes de la Dampoort) | **Confirmé** | IOE 302193 (« negen poorten, waarvan er vijf dienst doen als vooruitgeschoven posten ») ; WP NL (Dampoort : « aanvankelijk 3 poorten ») | — |
| 4 | Gentpoort 1361-1363, détruite en 1382, rebâtie 1401-1407 | **Confirmé** | Beheerplan ; IOE 14697 (« 1401 en 1407 ») ; OSM (bâtiment à 20 m du point, sur la levée) | — |
| 5 | Kruispoort 1366-1367, 1401-1406 | **Confirmé** | Beheerplan (première porte 1302-1304, travaux 1366-1367) ; IOE 14697 (1401-1406) ; OSM (à 1 m) | Note (première porte 1302-1304) |
| 6 | Smedenpoort 1367-1368 | **Confirmé** | IOE 302193 (1367-1368) ; Beheerplan et IOE 14697 (1368, Jan Slabbaerd et Matthieu Saghen) ; OSM (bâtiment dans le fossé, 30 m à l'extérieur du point de levée) | — |
| 7 | Ezelpoort 1369-1370 | **Confirmé** (date) / **corrigé** (position) | Beheerplan (1369, sur le modèle des portes Smeden et Boeverie ; pont de 1385) ; IOE 14697 (« het bakstenen poortgebouw ligt in de binnengracht ») ; OSM (bâtiment (−561, 865)) | Porte et sommet de levée de (−509, 832) / (−503, 842) à (−550, 848) ; point du fossé (−518, 855) → (−561, 865) ; test ajouté |
| 8 | Boeveriepoort 1366-1367 | **Incertain** | WP NL (première porte en service en 1312, rebâtie en 1366-1367 par Jan Slabbaert) contre IOE 14697 (bâtie « quatre ans » avant la Smedenpoort de 1368, soit vers 1364) | Date gardée, divergence écrite dans la note de l'enceinte |
| 9 | Katelijnepoort rebâtie en 1401 | **Confirmé** | IOE 14697 (1401-1402, disparue en 1782) ; OSM (bout de la Katelijnestraat (258, −991), porte (254, −980)) | — |
| 10 | Dampoort : trois portes, positions et dates | **Incertain** (`hypothetical`) | WP NL (Sint-Nikolaaspoort, Sint-Leonarduspoort, Speiepoort ; porte unique de 1660) ; OSM (ancienne écluse de la Lange Rei vers (660, 1340-1410), nœud « Dampoort » (750, 1385), à 68 m de la levée) | — |
| 11 | Poertoren 1398-1401, 18 m | **Confirmé** | Beheerplan (commande de 1398) ; IOE 11630 et 14697 (1398-1401) ; IOE 302193 dit « 1367-1368 » (erreur probable, isolée) ; OSM (à 1 m) | Description (sources et divergence) |
| 12 | Tour est du Minnewater 1400-1401, arasée vers 1621 | **Confirmé** (dates) / position **incertaine** | Beheerplan (fiche IJskelder) ; IOE 14697 (« reeds in 1621 afgebroken ») | — |
| 13 | Murs de pierre de la Begijnenvest (1398) et Minnewater - Katelijnepoort (1399) | **Confirmé** (dates) / tracés **incertains** | Beheerplan | — |
| 14 | Beffroi et halles d'un seul tenant après 1280 | **Confirmé** | IOE 29457 (dendrochronologie : charpentes 1268-1300, 1281-1291 ; tour 1270-1300 ; « geen herkenbare delen … van rond 1240 ») ; OSM (à 1 m) | — |
| 15 | Beffroi : une assise en 1340, seconde vers 1345-1346 | **Incertain** | IOE 29457 (« toevoeging of restauratie » d'une seconde assise vers 1345-1346) contre WP NL (deux assises carrées dès la réparation de 1291-1296) | Divergence écrite dans la description |
| 16 | Octogone 1482-1486, 83 m | **Confirmé** | IOE 29457 ; WP NL (1483-1487) | — |
| 17 | Halles ≈ 84 × 44 m | **Confirmé** (probable) | WP NL ; IOE 29457 (emprise Markt - Wollestraat - Oude Burg - Hallestraat) | — |
| 18 | Waterhalle 1284-1294, 95 × 24 m, démolie en 1787, hauteur 18 m | **Confirmé** (dates, emprise) / **corrigé** (hauteur) | WP NL (achevée en 1294, 95 × 24 m, quinze travées, « geschatte hoogte van 30 meter ») ; OSM (Provinciaal Hof à 5 m) | `height_m` 18 → 30 ; test ajouté |
| 19 | Ghyselhuus jusqu'en 1375, hôtel de ville à partir de 1376 | **Confirmé** | IOE 29457 (échevins au Ghiselhuus après l'incendie de 1280) ; IOE 29238 (« sloper van het Ghyselhuus », première pierre 1376, arrêt 1380-1385, achèvement 1421) ; OSM (hôtel de ville à 1 m) | — |
| 20 | Saint-Basile (1139-1149) | **Confirmé** | IOE 29239 (source d'auteur) ; OSM (à 3 m) | — |
| 21 | Saint-Donatien : nef gothique 1331-1361, démolie en 1799 | **Confirmé** (dates) / gabarit **incertain** | IOE 29235 (1184, chœur à déambulatoire, transept, tour de croisée, nef de quatre travées ; nef reprise 1331-1361) ; OSM (site archéologique (187, 65) au bout est du gabarit) | — |
| 22 | Le Steen, côté ouest du Burg | **Confirmé** (site) / gabarit **incertain** | WP NL (« op de westzijde van de Burg », cité en 1088, prison, démoli en 1784-1785 ; parcelle entre Ten Steegere et la Breidelstraat) | — |
| 23 | Notre-Dame : tour de brique 1270-1340, 70 m sans flèche | **Confirmé** (date) / hauteur **incertaine** | IOE 82359 (« romp tussen 1270-1340 » ; ailleurs « 1270-tweede helft van de 14de eeuw » ; flèche vers 1440) | — (hauteur de 1340 inconnue) |
| 24 | Saint-Sauveur : nef romane jusqu'en 1358, nef rebâtie dès 1359 | **Confirmé** (1358) / **corrigé** (nef) | IOE 29716 (« 1358 : toren en Romaans schip … verwoest door brand. Eerste kwart van de 15de eeuw : bouw van het huidige gotische schip ») | `saint_sauveur_nef.from_year` 1359 → 1425 ; test mis à jour |
| 25 | Couvents (Dominicains, Franciscains, Augustins, Eekhout) : emprises | **Incertain** (`hypothetical` gardé) | IOE 83312 (Dominicains, source d'auteur) ; WP NL ; OSM (rue Eekhoutpoort (150-200, −230/−260), près du point restitué (205, −255)) | — |
| 26 | Béguinage 1244-1245, dans les murs en 1297 | **Confirmé** | IOE 122155 (source d'auteur) ; OSM (enclos à 4 m, église à 8 m) | — |
| 27 | Positions OSM des églises Saint-Jacques, Saint-Gilles, hôpital Saint-Jean, Potterie, Waterhuis | **Confirmé** | OSM (écarts de 1 à 15 m) | — |

## Ce qui reste ouvert

- **Hauteurs** : beffroi en 1340 (une ou deux assises), tour de Notre-Dame en 1340 (en chantier
  jusqu'à la seconde moitié du XIVe s.), levée avant 1702 ; aucune source chiffrée trouvée.
- **Tracé du fossé** : l'axe restitué d'après l'eau actuelle s'écarte des portes de 20 à 40 m par
  endroits (Gentpoort : fossé élargi au XVIIIe s. vers la Coupure ; nord-ouest comblé au XIXe s.) ;
  seul l'abord de l'Ezelpoort a été recalé. À reprendre avec Ryckaert, *Brugge* (Stedenatlas van
  België, 1991) ou le plan de Marcus Gerards (1562, contrôle humain).
- **Dampoort** : positions et dates des trois portes.
- **Saint-Sauveur entre 1359 et 1424** : état de la nef (ruine, chantier, couverture provisoire) et
  de la tour inconnu ; le moteur rend le chœur seul.
- **Boeveriepoort** : 1364 ou 1366-1367 ; première porte en 1297 ou 1312.
- Portes de la première enceinte (Vlamingpoort, Mariapoort, Noord- et Zuidzandpoort…) : présence en
  1340 non établie, toujours omises.

## Vérifications

- `uv run --project tools pytest -q tools/tests/test_landmarks_v2.py
  tools/tests/test_landmarks_v2_bordeaux.py tools/tests/test_landmarks_v2_bruges.py` : 63 réussis
  (test des monuments mis à jour : nef de Saint-Sauveur 1425, Waterhalle 30 m ; test des portes :
  Ezelpoort à moins de 40 m du bâtiment OSM).
- Modifications faites par script (`landmarks_v2.write_city`, format du fichier inchangé).

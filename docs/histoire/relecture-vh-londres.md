# Relecture historique — Londres vers 1340 (VH6)

Relecture du 2026-09-26 de `data/landmarks_v2/london.json` (restitution 1:1, ADR 0078), sur la liste
« Faits à faire relire » de `docs/wip/vh6-londres.md` puis sur tous les éléments `probable` /
`hypothetical`. Règle suivie : faits seulement ; rien n'est extrait des sources non commerciales
(MoEML, Historic Towns Atlas) ; les positions viennent des monuments classés de Historic England
(NHLE, Open Government Licence v3, service ArcGIS ouvert) et d'OpenStreetMap (ODbL), converties
dans le repère local EPSG:3035 de la ville (origine = Temple).

Repère : `at` = [est, nord] en mètres depuis l'origine ; angle de grille : l'est vrai vaut ≈ −8°.

## Bilan

- **Confirmés : 23** · **Corrigés : 20** · **Incertains : 6** sur 49 faits (verdict principal de chaque ligne ; les verdicts partiels sont précisés dans le tableau).
- Corrections majeures :
  1. **London Bridge passait à l'ouest de St Magnus, pas à l'est** : l'église bordait l'accès du
     pont côté est (en 1762 on a dû percer le passage piéton sous sa tour). Pont décalé de ≈ 48 m
     vers l'ouest, dans l'axe de New Fish Street (Fish Street Hill) ; accès nord et sud retracés.
  2. **Mur Newgate-Aldersgate** : le tracé bombait de 30 à 45 m trop au nord ; recalé sur les
     tronçons classés (Postman's Park, King Edward Street, 121-124 Newgate Street). Mêmes recalages
     (15 à 20 m) entre Ludgate et Newgate (Amen Court, Old Bailey) et entre Moorgate et Bishopsgate.
  3. **Couvent des Franciscains** : église déplacée de ≈ 85 m et remise d'est en ouest le long de
     Newgate Street, sur l'emprise du site classé (elle était tournée de −30° vers le nord-ouest).
  4. **Prieuré de la Sainte-Trinité d'Aldgate** : église orientée comme Mitre Street (sa nef),
     cloître au nord (Mitre Square) au lieu du sud.
  5. **Hôtel d'Exeter** : il était à l'emplacement de l'hôtel de l'évêque de Bath (Arundel) ;
     déplacé de ≈ 130 m vers l'est, entre Milford Lane et le Temple.
  6. Dates : Franciscains achevés vers 1327 (et non 1348) ; Carmes fondés en 1241 (et non 1253) ;
     crypte du Guildhall datée vers 1300 (et non XIIe s.) ; chapelle du pont sur la 9e pile depuis
     la Cité (0,45 au lieu de 0,48).
- Rues d'après la Dissolution percées à travers les enclos corrigés, retirées et ajoutées à
  `osm_streets.exclude` : Mitre Street, Mitre Passage, St James's Passage, Sugar Bakers Court
  (Sainte-Trinité, dissoute en 1532), Austin Friars (Augustins, 1538), Greyfriars Passage,
  Essex Street (vers 1675, sur Exeter Inn).

## Tableau

| Fait | Verdict | Source | Changement |
|---|---|---|---|
| Départ du mur à la Tour : poterne de Tower Hill, tronçon classé jusqu'au métro | Confirmé | Historic England, NHLE 1002063 « London Wall: section from underground railway to Tower Hill » et 1002062 ; OSM « Tower Hill Postern » | Aucun (écarts ≤ 12 m) |
| Aldgate, tronçons Crutched Friars, America Square, Goring Street, Camomile Street, All Hallows | Confirmé | NHLE 1002048, 1002069, 1432676, 1002049, 1005547, 1002050, 1002067 | Aucun (écarts 1-11 m) |
| Tracé Moorgate-Bishopsgate | Corrigé | NHLE 1002051 « Bloomfield House to site of Moor Gate » | Sommets (1640, 305) → (1640, 290), (1750, 257) → (1750, 242) : 15-18 m au sud |
| Tracé Moorgate-Cripplegate | Corrigé | NHLE 1018885 (parking de London Wall), 1018886 (St Alphage) | (1548, 347) → (1540, 325), (1440, 382) → (1440, 365), (1360, 414) → (1360, 400) |
| Position de Moorgate | Corrigé | NHLE 1002051 (extrémité ouest du tronçon classé, « site of Moor Gate ») | Porte (1548, 347) → (1520, 333), sur le mur recalé |
| Moorgate ouverte en 1415 | Incertain | Medieval London (Fordham), « Moorgate » ; Wikipédia (en) « Moorgate » ; Stow via Fordham | `from_year` 1415 gardé ; les sources divergent (poterne percée en 1415 par le maire Thomas Falconer selon Stow, ou poterne plus ancienne agrandie en 1415) ; note du mur précisée |
| Cripplegate, fort de Cripplegate, Noble Street | Confirmé | NHLE 1018887, 1018888, 1018889, 1018890 | Aucun |
| Position d'Aldersgate | Corrigé | NHLE 1018882 « Roman, medieval and post-medieval gateway at Aldersgate » ; plaque OSM « Site of Aldersgate » | (970, 345) → (972, 332) (13 m au sud, sous Aldersgate Street) |
| Tracé du mur Aldersgate-Newgate | Corrigé | NHLE 1018883 « Postman's Park and King Edward Street », NHLE 1003773 « site of Newgate and 121-124 Newgate Street » | Le mur passait à y ≈ 345-356 (et non 375-392) : sommets (890, 347), (800, 352), (700, 357) + bastion (660, 354) |
| Position de Newgate | Corrigé | NHLE 1003773 ; plaque OSM « Site of Newgate » ; axe de Newgate Street | (628, 336) → (626, 325), en travers de Newgate Street |
| Tracé Ludgate-Newgate (Old Bailey) | Corrigé | NHLE 1018884 (Central Criminal Court), 1002068 (Amen Court), 1002052 (Ludgate) ; tronçons OSM | (598, 250) → (617, 240), (572, 160) → (588, 160) : ≈ 20 m à l'est |
| Position de Ludgate (contre St Martin within Ludgate) | Confirmé | NHLE 1002052 ; OSM « Ludgate » | Aucun (≈ 10 m) |
| Prolongement Ludgate → Fleet → Tamise pour les Dominicains, vers 1280-1320 | Confirmé (dates précisées) | W. Page (éd.), *VCH London I* (1909), « Friaries: The black friars », https://www.british-history.ac.uk/vch/london/vol1/pp498-502 ; MoEML « The Wall » (faits) ; Medieval London (Fordham, S. Burns), « Blackfriars » | Note : site concédé en 1276, mur abattu et reconstruit par étapes jusque vers 1320 (et non « autorisé en 1276 ») |
| Front de Tamise ouvert (mur de rive ruiné au XIIe s., FitzStephen) | Confirmé | Wikipédia (en) « London Wall » ; MoEML « The Wall » | Aucun |
| Old St Paul's : longueur 178 m, largeur 31 m | Confirmé | Wikipédia (en) « Old St Paul's Cathedral » (fouilles de Penrose, 1878 : 585 × 100 pieds) | Aucun |
| Old St Paul's : transept 90 m | Corrigé | Idem : 290 pieds ≈ 88 m | `transept_m` 90 → 88 |
| Old St Paul's : tour 87 m + flèche 62 m ≈ 149 m | Confirmé (fourchette) | Wikipédia (en) ; *Encyclopaedia Britannica* 1911, « St Paul's Cathedral » (489 pieds) ; Wren : 460 pieds ≈ 140 m | Aucun ; description : fourchette 140-149 m |
| Old St Paul's : flèche « achevée en 1315 » | Incertain | Wikipédia (en) : clocher élevé en 1221, « New Work » consacré en 1300 et achevé en 1314 ; d'autres sources donnent 1315 | Description : « clocher vers 1221, flèche achevée ou refaite vers 1314-1315 selon les sources » |
| Old St Paul's : position et axe | Corrigé (position) / incertain (angle) | *Britannica* 1911 : « its northern, eastern and southern extremities approximately correspond with those of old St Paul's », orientation de Wren « more northerly » ; R. Crayford, « The Setting-Out of St Paul's Cathedral », *Architectural History* 2001 (axe changé pour viser Ludgate Hill) | Centre (819, 30) → (804, 34) : chevet au chevet de Wren, nef dépassant à l'ouest. Angle −4° (≈ 4° au nord de l'est vrai, Wren ≈ 7°) gardé : sens confirmé, valeur non publiée en ligne |
| Cloître et salle capitulaire de Ramsey (1332) | Corrigé (léger) | *Britannica* 1911 : « in the angle west of the south transept » ; plaque « St Paul's medieval footprint » (London Remembers) | (786, −8) → (776, −6), hors du croisillon sud |
| London Bridge : 19 arches, ≈ 282 m | Confirmé | V. Harding et L. Wright (éd.), *London Bridge: Selected Accounts and Rentals 1381-1538*, London Record Society 31, introduction, https://www.british-history.ac.uk/london-record-soc/vol31/vii-xxix ; Wikipédia (en) « Old London Bridge » | Aucun |
| London Bridge : extrémités « à l'est de St Magnus » | **Corrigé** | Site de St Magnus the Martyr, « Last years of old London Bridge » (en 1762, St Magnus bloquait le nouveau trottoir : passage percé sous la tour) ; plaque OSM « This churchyard formed part of the roadway approach to Old London Bridge » ; pont aligné sur Fish Street Hill (Building London, « Old London Bridge ») | `from` (1606, −598) → (1558, −592), `to` (1450, −880) → (1410, −860) (même orientation, rive sud recalée : ≈ 306 m) ; rues « New Fish Street » et « Grand-rue de Southwark » retracées ; St Magnus (1578, −574) → (1586, −577) |
| Chapelle Saint-Thomas à 0,48, côté aval | Corrigé (fraction) / confirmé (côté) | D. Gerhold, *London Bridge and its Houses, c. 1209-1761* (2019), résumé par C. Catling, *The Past*, https://the-past.com/feature/life-across-the-water-exploring-london-bridge-and-its-houses/ : 9e pile depuis la rive nord ; Wikipédia (en) « Chapel of St Thomas on the Bridge » : 11e pile depuis Southwark, côté aval (est) | `chapel_at` 0,48 → 0,45 (9/20) ; `chapel_side` « left » = est confirmé |
| Pont-levis à 0,66 et sa porte au nord | Corrigé (léger) | Gerhold (*The Past*) : tour du pont-levis sur la 13e pile depuis le nord ; Wikipédia (en) « Fortifications of London » : pont-levis entre les 7e et 8e piles depuis Southwark, mentionné dès 1257 | `drawbridge_at` 0,66 → 0,675, porte 0,62 → 0,65 |
| Grande porte de pierre à 0,9 | Confirmé | Gerhold : 18e pile depuis le nord (2e depuis le sud) ; « Fortifications of London » : 3e pile depuis la rive | Aucun |
| Maisons dès 1201 | Confirmé | Wikipédia (en) « Chapel of St Thomas on the Bridge » ; Gerhold : ≈ 140 unités en 1358 | Note complétée |
| Tablier ≈ 5,5 m au-dessus de l'eau | Incertain | Aucune mesure trouvée ; dénivelé amont/aval jusqu'à 5-6 pieds (Gerhold, Wikipédia) | Aucun (restitution signalée dans la note) |
| Avant-becs de 26 m (`starling_m`) | Incertain | Blog London Historians (Gerhold) : « starlings … up to fifty feet long » | Aucun ; à revoir avec l'ouvrage de Gerhold |
| Westminster : nef romane jusqu'en 1375, nef gothique dès 1376 | Confirmé | Westminster Abbey, « The Nave » (reconstruction commencée en 1376, H. Yevele, achevée en 1517) | Aucun |
| Westminster Hall 73 × 20,7 m (1097-1099) | Confirmé | Wikipédia (en) ; emprise OSM | Aucun |
| Chapelle Saint-Étienne 1292-1348 | Confirmé (précisé) | UK Parliament, « St Stephen's Chapel » ; Virtual St Stephen's (Univ. of York) : largement achevée en 1348, décor jusqu'aux années 1360 | Description : encore en chantier en 1340 |
| Tour du Joyau 1365-1366 | Confirmé | English Heritage, « History of the Jewel Tower » ; NHLE 1003580 (emprise : écart 5 m) | Aucun |
| Horloge de New Palace Yard 1365-1367 | Confirmé | UK Parliament, « A brief history of Big Ben » (première tour d'horloge 1365-1367) | Aucun |
| Salle capitulaire de Westminster | Confirmé | NHLE 1003579 (écart 3 m) | Aucun |
| Palais de Savoie jusqu'en 1381 | Confirmé | London Museum, « The Savoy Palace & Hospital » ; Wikipédia (en) « Savoy Palace » (Pierre de Savoie 1246, Edmond de Lancastre, détruit le 13 juin 1381) | Aucun ; position entre Savoy Street et Carting Lane vérifiée |
| Guildhall d'avant 1411 : position, cryptes | Corrigé | T. Dyson, N. Holder, I. Howell, D. Bowsher, *The London Guildhall: an Archaeological History of a Neighbourhood* (MOLAS, 2007), recension Reviews in History n° 766 : fondation vers 1120-1130, salle reconstruite vers 1300 sur une crypte de 5 travées (crypte ouest), chapelle des années 1290, porterie sur Cat Street | Centre (1275, 190) → (1278, 197), angle −12° → −6° (moitié ouest de la grande salle actuelle, OSM) ; description refaite ; reste `hypothetical` |
| Prieuré de la Sainte-Trinité d'Aldgate (1108) | Corrigé | J. Schofield et R. Lea, *Holy Trinity Priory, Aldgate* (MOLA Monograph 24, 2005), présentation de l'éditeur (fondé en 1107-1108, au nord de Leadenhall Street, juste à l'intérieur d'Aldgate) ; Wikipédia (en) « Holy Trinity Priory, Aldgate » (Mitre Street suit la nef, Mitre Square le cloître) ; OSM (arc au 71-77 Leadenhall Street) | `at` (2225, −140) → (2210, −164), angle −8° → −40°, cloître sud → nord ; reste `hypothetical` (élévation) |
| Austin Friars (1253) | Confirmé (date) / corrigé (position) | *VCH London I*, « Friaries: The Austin friars », https://www.british-history.ac.uk/vch/london/vol1/pp510-513 (1253, Humphrey de Bohun ; église rebâtie en 1354) ; Wikipédia (en) « Dutch Church, Austin Friars » (nef = église hollandaise) | `at` (1710, 72) → (1736, 80) : nef sur l'église hollandaise (OSM), chœur à l'est ; reste `hypothetical` (église d'avant 1354 inconnue) |
| Greyfriars : église 1306-1348, ≈ 91 m | Corrigé | *VCH London I*, « Friaries: The grey friars », https://www.british-history.ac.uk/vch/london/vol1/pp502-507 (1306, achevée en 1327, non dédicacée en 1357, 300 × 89 pieds) ; NHLE 1002002 « The London Greyfriars, site of » | `at` (715, 329) → (782, 291) (église centrée sur (782, 272)), angle −30° → 0° ; description |
| Blackfriars (1276), église ≈ 67 m | Confirmé (date) / incertain (longueur) | *VCH London I*, « The black friars » (1276, église commencée en 1279) ; Fordham (largeur 19,7 m) | Aucun |
| Whitefriars (1253) | Corrigé | *VCH London I*, « Friaries: The white friars », https://www.british-history.ac.uk/vch/london/vol1/pp507-510 (fondé en 1241 par Sir Richard Gray ; rebâti par Hugh Courtenay en 1350) | Nom « 1253 » → « 1241 », description ajoutée ; position (entre Whitefriars Street et Carmelite Street) confirmée |
| Hôtel de l'évêque d'Exeter (Outer Temple) | Corrigé | C. L. Kingsford, « Essex House, formerly Leicester House and Exeter Inn », *Archaeologia* 73 (1923), résumé Cambridge Core (entre le Temple et l'hôtel de l'évêque de Bath, acquis par Walter de Stapledon en 1323) | `at` (−330, −40) → (−205, 10) ; description ; Essex Street exclue |
| Hôtel de l'évêque de Durham | Confirmé (existence en 1340) | *Survey of London* 18, « Durham Place », https://www.british-history.ac.uk/survey-london/vol18/pt2/pp84-98 (bâti par Antony Bek, 1285-1310, refait par Hatfield après 1345 ; à l'ouest d'Ivy Bridge Lane) | Aucun (écart ≈ 30 m, gabarit `hypothetical`) |
| Hôtel de l'évêque de Norwich | Confirmé | *Survey of London* 18, « York House », https://www.british-history.ac.uk/survey-london/vol18/pt2/pp51-60 (mention dès 1237, voisin ouest de Durham) | Aucun (sur Buckingham et Villiers Street) |
| York Place (vers 1240) | Incertain (position) | Wikipédia (en) « York House, Strand » / « Palace of Whitehall » | Aucun ; ≈ 70 m au sud-ouest de la cave de Wolsey, gabarit `hypothetical` |
| Croix d'Éléonore de Charing (1291-1294) | Corrigé | OSM : statue de Charles Ier, qui marque le site de la croix | (−1270, −462) → (−1287, −402) (60 m au nord) |
| Palais de Westminster (Chambre peinte ≈ 24 × 8 m) | Incertain | UK Parliament, « Living Heritage » ; aucun plan ouvert de 1340 | Aucun (reste `hypothetical`) |
| Tour de Londres (Édouard Ier 1275-1285, Middle Tower 1280, Tour Blanche 36 × 32 × 27 m, Saint-Pierre 1286-1287) | Confirmé | NHLE 1002061 ; Wikipédia (en) « Tower of London », « White Tower » | Aucun |
| Palais des évêques de Winchester | Confirmé | NHLE 1002054 (écart 16 m) | Aucun |

## Points ouverts

- Hauteur du tablier et longueur des avant-becs de London Bridge : chercher les chiffres dans
  Gerhold (2019) ou dans les comptes du Bridge House (London Record Society 31).
- Angle d'Old St Paul's : le plan de Penrose ou l'ouvrage de J. Schofield, *St Paul's Cathedral
  before Wren* (English Heritage, 2011), donneraient la valeur exacte.
- Moorgate avant 1415 : une poterne plus ancienne est possible ; non représentée.
- Rues d'après 1666 qui traversent encore l'emprise d'Old St Paul's (St Paul's Churchyard,
  St Paul's Alley) : non retirées, car la voie « St Paul's Churchyard » suit aussi en partie le
  pourtour médiéval ; à traiter segment par segment si le rendu gêne.
- Historic England bloque la lecture des fiches en ligne (HTTP 403) ; les emprises viennent du
  service ArcGIS ouvert (NHLE, OGL), sans le texte des fiches.

# VH8 — Bruges vers 1340 à l'échelle 1:1 (format v2, ADR 0078)

Branche `feat/rs-g-cities` (worktree partagé avec trois autres villes). Référence :
`docs/landmarks-v2.md`, exemple Rouen. Fichiers du lot : `data/landmarks_v2/bruges.json`,
`tools/tests/test_landmarks_v2_bruges.py`, ce suivi. Script d'auteur non versionné (scratchpad).

## État
- [x] Squelette validant le schéma (origine sur le beffroi, EPSG:3035 [3848306, 3143840])
- [x] Recherche des faits (voir ci-dessous)
- [ ] Enceinte de 1297 (vesten) et portes datées, murs de pierre de 1398-1401
- [ ] Monuments, quartiers, espaces libres, eaux (reien à la main d'après OSM), rues OSM
- [ ] Sources, test des faits datés

## Faits retenus (sources)
- Vesten : 1297-1302, double fossé, levée de terre et palissade de bois (Inventaris 302193) ;
  ≈ 6,8 km, 430 ha ; levée ≈ 6 m (coupes de 1702, Beheerplan Vesten, Stad Brugge 2024).
  Démantèlement des fortifications vers 1328 (lettres de Philippe VI du 20 déc. 1328 après
  Cassel ; l'Inventaire dit « en exécution du traité d'Athis-sur-Orge, 1305 »).
- Portes : Gentpoort rebâtie 1361-1363, détruite 1382, rebâtie vers 1401-1407 ; Kruispoort
  travaux 1366-1367, détruite 1382, rebâtie 1401-1406 ; Boeveriepoort 1366-1367 ; Smedenpoort
  1367-1368 ; Ezelpoort 1369-1370 ; Katelijnepoort début XVe s. (Jan van Oudenaerde et Maarten
  van Leuven) ; Dampoort = trois portes (Sint-Nikolaas/Koolkerke, Sint-Leonardus/Dudzele,
  Speiepoort, écluse), dates inconnues.
- Poertoren 1398-1401 (Beheerplan) + mur de pierre à cinq tours le long de la Begijnenvest ;
  1399 mur de pierre Minnewater-Katelijnepoort ; tour est du Minnewater 1400-1401.
- Beffroi : tour de brique et halles d'un seul tenant après l'incendie de 1280 (dendro 1270-1300) ;
  vers 1345-1346 seconde assise carrée ; 1482-1486 octogone ; 83 m aujourd'hui (Inventaris 29457).
- Waterhalle 1284/1294-1787, 95 × 24 m, côté est du Markt sur la Kraanrei (WP NL).
- Stadhuis 1376-1421 (Inventaris 29238) ; avant, Ghyselhuus des échevins (vers 1280).
- Saint-Donatien : romane après 1184 (long chœur à déambulatoire, transept, tour de croisée, nef de
  4 travées), nef gothique 1331-1361, démolie 1799 (Inventaris 29235).
- Notre-Dame : tour de brique 1270-1340 (ou 2e moitié XIVe s.), flèche 1440 (Inventaris 82359).
- Saint-Sauveur : chœur gothique vers 1275, nef romane jusqu'à l'incendie de 1358 (Inventaris 29716).
- Saint-Jean : salles vers 1150, 1234, 1268, 1285-1290 (Inventaris 82410) ; Béguinage 1244-1245
  (Inventaris 122155) ; Potterie 1276, église 1359 (83197) ; Dominicains 1234, église 1311-1320
  (83312) ; Ter Beurze : auberge 1285-1302, bâtiment 1453 (29897).

## Notes
- Carte fine : aucun cours d'eau nommé utile (seul « Zuidervaartje » le long des vesten est) :
  `fine_rivers: []`, reien tracées à la main (`origin: "hand"`, `draw: true`) d'après OSM.
- Cache OSM hors dépôt : `tools/geo/raw/osm/bruges_features.json`, `bruges_features2.json`,
  `bruges_highways.json`.

## Prochaine étape
Écrire enceinte, eaux, monuments, quartiers avec le script d'auteur, lancer `geo landmarks`.

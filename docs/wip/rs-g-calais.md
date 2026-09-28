# RS-G / VH8 — Calais vers 1340 à l'échelle 1:1 (format v2)

Branche `feat/rs-g-cities` (worktree partagé avec d'autres villes). Fichiers : `data/landmarks_v2/calais.json`,
`tools/tests/test_landmarks_v2_calais.py`, ce suivi. Caches OSM (non versionnés) :
`tools/geo/raw/osm/calais_highways.json`, `calais_features.json`, `calais_features2.json`.

## État : terminé (à relire et fusionner par l'orchestrateur)
- Origine à 150 m au nord de la tour du Guet (front de havre, axe de la rue du Havre), à ≈ 283 m de
  l'ancre v1 (qui tombe dans le havre) ; `extent_m` 1 500.
- Enceinte de Philippe Hurepel (`from_year` 1228) : 3,1 km, 52 ha, ≈ 1 160 m est-ouest, 44 tours,
  mur de 2,5 m, fossé ; quatre portes (du Havre, de Boulogne, de Gravelines, vers Saint-Pierre) ;
  seconde enceinte et second fossé (« double enceinte et fossé doublé » de 1346), non datée.
- 13 monuments : tour du Guet, hôtel de ville (1231), Notre-Dame en trois phases (église du XIIIe s.
  jusqu'en 1369, reconstruction anglaise 1370-1409, tour de croisée de 56 m à partir de 1410),
  Saint-Nicolas (jusqu'en 1563), château de Hurepel (1228-1559), hôtel de l'Étape (1363), fortin du
  Rysbank (1347-1399), tour de pierre du Rysbank (1400), trois avant-portes sur terrée circulaire.
- Quartiers : ville close + faubourgs sud, est, ouest (Greaves) ; espaces : place du Marché,
  cimetière Notre-Dame, Rysbank et dunes (grève) ; quai du havre ; havre en polygone `hand`
  (`fine_rivers: []`).
- Rues : 52 tronçons OSM (boulevards, quais, places et rues du quartier de l'Europe exclus) + trois
  chemins à la main (Nieulay, Saint-Pierre, Gravelines ; vue de 1558).
- Tests : `test_landmarks_v2.py` + `test_landmarks_v2_calais.py` (3 tests de faits) : 59 réussis.

## Restitutions hypothétiques (sources manquantes)
- **Tracé de l'enceinte** : seuls la tour Carrée (front sud, dans la citadelle), la longueur de
  1 100 m, le château au nord-ouest et le havre au nord sont sourcés ; fronts nord, est et ouest
  restitués. La profondeur de 400 m citée (Réseau Vauban) est contredite par Notre-Dame et la tour
  Carrée : ≈ 500 m retenus. Tour Pavée non localisée. À reprendre avec L. Lenoir, *À la découverte
  des anciennes fortifications de Calais* (2001), et les rapports de diagnostic (Guéquière 2019,
  Poisson 2018-2019) ; History of the King's Works (plan de Calais) non consulté.
- **Noms et positions des portes** : aucun nom français de 1340 trouvé ; noms anglais (Lantern,
  Milk, Boulogne, Water Gate) du XVIe s. ; seule la Lantern Gate est située (rue de la Mer).
- **Seconde enceinte** : date de construction inconnue, tracé à 26 m restitué.
- **Château** : emprise, donjon et « tour détachée » restitués.
- **Notre-Dame** : orientation nord-sud de l'église du XIIIe s. déduite de la façade à deux tours
  du bras nord (Wikipédia, non recoupé) ; dates 1370 et 1410 conventionnelles ; article de P. Héliot
  (Bulletin monumental 1947, Persée) non lu (accès protégé).
- **Saint-Nicolas, hôtel de ville, hôtel de l'Étape** : positions restituées (Étape : Staple Inn
  d'époque Tudor).
- **Rysbank** : forme du fortin, date de la tour de pierre (avant 1400, inconnue).
- **Havre et trait de côte** : restitués d'après Greaves (langue de terre au nord), la vue de 1558 et
  le site du fort Risban ; bassin du Paradis = ancien havre (désenvasé en 1397). Aucune source
  cartographique fiable du trait de côte de 1340.
- **Rues** : trame OSM de la reconstruction (voies redressées et élargies, Inventaire IA62005683) ;
  aucun plan d'avant-guerre consulté pour vérifier rue par rue.
- Omis faute de localisation : Saint-Jean / Maison-Dieu, Carmes, Augustins (1351), ancienne église
  Saint-Pierre (Pétresse), camp anglais de Villeneuve-la-Hardie (1346-1347), maison forte de la rue
  Chanzy, pont et fort de Nieulay.

## Points de format manquants (contournés)
- `waters` sans `note` ni dates : la restitution du havre est décrite dans `period` et la source OSM.
- Polygones d'eau non dessinés par le moteur (couloir interdit seulement) : l'eau affichée reste
  celle de la carte actuelle.
- `walls`, `gates`, `open_spaces`, `quays` sans `certainty` : hypothèses écrites dans les `note`.

## Prochaine étape
Relecture historique (comme `docs/histoire/relecture-vh-rouen.md`) ; captures et contrôle en jeu
par la session principale (le fond de carte montre le port moderne).

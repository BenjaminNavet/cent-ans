# RS-G lot 1b — Avignon vers 1340 à l'échelle 1:1 (VH8)

Branche `feat/rs-g-cities` (worktree partagé avec les autres villes VH8). Fichiers du lot :
`data/landmarks_v2/avignon.json`, `tools/tests/test_landmarks_v2_avignon.py`, ce suivi.
Format : `docs/landmarks-v2.md` ; exemple Rouen. Cache OSM hors dépôt :
`tools/geo/raw/osm/avignon_*.json`.

## État
- [x] Squelette valide (origine sur l'ancre de la maquette v1, fleuve fin « Rhône »)
- [ ] Sources, enceintes datées et portes, pont Saint-Bénézet, monuments, quartiers, places
- [ ] Recette OSM et `cent-ans geo landmarks --city avignon`
- [ ] Test `test_landmarks_v2_avignon.py`

## Fleuve fin
La carte fine ne connaît qu'un « le Rhône » (normalisé « Rhône ») dans le rayon : une ligne de 300 m
de large le long du nord de la ville (bras d'Avignon). Pas de bras de Villeneuve fin dans le rayon.

## Faits relevés (sources en cours)
- Enceinte du XIIIe s. : 1234-1237, achevée 1248 (Wikipédia « Remparts d'Avignon ») ; tracé : rues
  des Trois-Colombes, Campane, Philonarde, Lices, Henri-Fabre, Joseph-Vernet, Grande-Fusterie ;
  portails Matheron, Peint (Imbert vieux, carrefour Philonarde-Bonneterie-Lices-Teinturiers),
  Boquier, Évêque, Bienson (webiane), Magnanen (nom de rue). Fossé = béal de la Sorgue (Rolland 1989).
- Remparts : 1355/1357-1370/1377, 4 330 m, 8 m, fossé 4 m (Sorgue, Durançole) ; sept portes de la
  fin du XIVe s. : Rhône, Oulle, Saint-Roch, Saint-Michel, Limbert, Saint-Lazare, Ligne (avignon.fr).
- Pont : 1177-1185, 22 arches, ≈ 920 m, tablier 4 m, chapelle sur la 3e pile ; tour Philippe-le-Bel
  (1er étage 1303, rehaussée vers 1360).
- Palais : Vieux 1335-1342, Neuf 1342-1351 ; Campane 1339-1340 ; Trouillas achevée 1346 (52 m) ;
  Anges 46 m ; Garde-Robe 1342-1343 ; Saint-Laurent sous Innocent VI ; chapelle Clémentine 52 × 15 × 20 m.
- Canal de Vaucluse (Sorgue) dès le Xe s. ; Durançole 1229.

## Prochaine étape
Script d'auteur (scratchpad) : enceintes, pont, monuments, quartiers ; puis outil et tests.

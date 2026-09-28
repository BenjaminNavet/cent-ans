# RS-G lot 1b — Avignon vers 1340 à l'échelle 1:1 (VH8)

Branche `feat/rs-g-cities` (worktree partagé avec les autres villes VH8). Fichiers du lot :
`data/landmarks_v2/avignon.json`, `tools/tests/test_landmarks_v2_avignon.py`, ce suivi.
Format : `docs/landmarks-v2.md` ; exemple Rouen. Caches OSM hors dépôt :
`tools/geo/raw/osm/avignon_highways.json`, `avignon_features.json`, `avignon_palace.json`.
Écrit par un script d'auteur temporaire (non versionné) puis `cent-ans geo landmarks --city avignon`.

## État : terminé (à relire par l'historien, lot 1e)
- Origine : ancre de la maquette v1 (palais), `extent_m` 2 000 (fort Saint-André, Chartreuse).
- Enceinte du XIIIe s. (1234-1248), polyligne ouverte le long des rues qui la conservent,
  6 portails nommés ; remparts d'Innocent VI et d'Urbain V `from_year` 1357, anneau de la relation
  OSM (35 sommets, ≈ 4,7 km pour 4,33 km cités), 7 portes de la fin du XIVe s. aux positions OSM.
- Pont Saint-Bénézet en deux travées droites (8 + 14 arches, ≈ 900 m, tablier 4 m, chapelle sur la
  3e pile) jusqu'à la tour Philippe-le-Bel.
- 30 monuments datés (2 attestés, 17 probables, 11 hypothétiques) : palais Vieux (1335), palais Neuf
  (1342), tours des Anges, Campane (1340), Trouillas (1346), Garde-Robe (1342), Saint-Laurent (1356) ;
  Notre-Dame-des-Doms (clocher jusqu'en 1404) ; Petit Palais ; Saint-Agricol, Saint-Pierre,
  Saint-Didier (ancienne ≤ 1355, gothique ≥ 1356), Carmes, Cordeliers, Augustins, Clarisses,
  Dominicains, Templiers, Saint-Laurent, Saint-Martial (1363), Célestins (1395) ; tour
  Philippe-le-Bel (rehaussée 1360), fort Saint-André (1362), abbaye Saint-André, collégiale de
  Villeneuve (clocher 1362), Chartreuse (1356).
- 4 quartiers (ville close, bourgs entre les deux enceintes, Villeneuve, bourg Saint-André),
  4 espaces libres (rocher des Doms, cloître des Carmes, cloître de Benoît XII, grande cour).
- 486 rues OSM (exclusions : rue de la République, cours Jean-Jaurès, rue Thiers, rue Bonaparte,
  place Pie, rues « du Rempart », boulevards et avenues hors les murs).
- Eaux : Rhône fin (« le Rhône », 300 m, bras d'Avignon, couloir seulement), bras de Villeneuve
  (OSM, largeur 220 m restituée, couloir), Sorgue le long de la rue des Teinturiers (OSM, dessinée).
- Tests : `test_landmarks_v2.py` + `test_landmarks_v2_avignon.py` : 61 réussis.

## Manques et restitutions (à relire)
- Mur du XIIIe s. : front nord (rocher, rive du Rhône entre Grande-Fusterie et rocher) non tracé ;
  raccord Philonarde-Campane par la rue Paul-Saïn non cité ; hauteur, tours, fossé restitués ;
  5 des 11 portails ni nommés ni placés ; article de Rolland (1989) à lire en entier.
- Remparts : tronçon du rocher et de la porte du Rhône joints en ligne droite ; date retenue 1357
  (sources : 1355, 1357, 1358) ; porte Saint-Roch médiévale supposée au même endroit que celle de 1865.
- Palais : répartition des ailes, positions et sections des tours (Campane, Garde-Robe,
  Saint-Laurent, Anges) restituées ; aile de la Grande Audience approchée. Plan d'érudit à consulter
  (Gagnière, Vingtain).
- Pont : point du coude et partage des arches restitués ; châtelet côté Avignon non figuré.
- Hauteurs des églises, clocher de Notre-Dame-des-Doms (reconstruction au XVe s. non datée,
  non figurée après 1405), position des Dominicains, Cordeliers, Augustins, Célestins, Saint-Laurent.
- Livrées cardinalices : seules la livrée d'Arnaud de Via (Petit Palais) et celle de Villeneuve
  (collégiale) sont figurées ; les autres faute de positions sourcées.
- Non figurés faute de source de position : Sainte-Madeleine, Notre-Dame-la-Principale, Saint-Geniès,
  Saint-Étienne, Sainte-Catherine (1354), Durançole (tracé distinct), port de la Fusterie, quais.
- Rhône fin décalé de 60-80 m vers le fleuve par rapport au bras OSM ; bras de Villeneuve absent
  de la carte fine (point à traiter dans `hydro_fine`).
- « Rue de la République » de Villeneuve exclue avec celle d'Avignon (même nom).

## Points de format
- Pas de `certainty` ni de `note` sur les portes : les réserves sont dans la `note` du mur.
- Pas de pont coudé : deux entrées `bridges` consécutives.
- Tours carrées du palais en `belfry` (`top: turrets`) : `tower`/`keep` sont des cylindres.

## Prochaine étape
Relecture historienne (lot 1e : `docs/histoire/relecture-vh-avignon.md`), puis contrôle visuel
par l'orchestrateur (`vh4_shots.gd --city=avignon`).

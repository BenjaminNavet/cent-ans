# VH7 — Orléans vers 1340-1429 à l'échelle 1:1 (format v2, ADR 0078)

Branche `feat/vh7-orleans` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`, lot VH7).
Référence : `docs/landmarks-v2.md` (section « Orléans vers 1340-1429 »), exemple Rouen. Liens
symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` ; dylib copiée (aucun changement
Rust). Cache OSM : `tools/geo/raw/osm/orleans_highways.json` (Overpass, bbox 1,87-1,945 E ×
47,88-47,92 N ; le serveur principal était saturé : miroir kumi.systems, mêmes données ODbL).

## Décision : vue stratégique
Orléans n'a pas de maquette L1/L2 (ni dans `data/landmarks/`, ni de bloc de siège) : c'est une
colonie ordinaire ZG6 (`towns_1340.json`, `set_orleans`). Choix : **colonie ordinaire en vue
lointaine** (pas de cuisson Blender) ; la ville 1:1 v2 remplace la ville ZG6 au palier vallée.
Addendum à l'ADR 0078 ; `landmark` devient facultatif dans le schéma v2.

## Changements du moteur commun (petits, rétrocompatibles) — à relire par VH5/VH6
- `landmark_v2.schema.json` : `landmark` facultatif ; nouveau gabarit `earthwork`.
- `LandmarkMonuments` : gabarit `earthwork` (levée trapézoïdale + palissade, fer à cheval ou anneau).
- `LandmarkCityLayer` : `has_maquette(id)` ; ville sans maquette affichée seulement quand le poids
  vallée atteint `TownRenderProfile.min_valley_weight` (comme les villes ZG6), `is_shown` en tient
  compte. Rouen inchangé.
- `TownLayer.setup` : ne construit plus une colonie qui a une ville v2 (sauf `--no-landmarks-1to1`).
- `LandmarkPlan` : index des segments d'eau par cases de 100 m (`_index_waters`, `_in_water` avec
  index facultatif) pour le sol des quartiers (la Loire fine est en 5 tronçons de 350 m de large).
- `TownBuilder` (pont) : nombre de piles = `arches` − 1 quand le plan le donne (Rouen : 12 piles au
  lieu de 18) ; tablier posé sur la base des piles (eau) et relevé en mètres : sous l'exagération
  locale ZG8 il flottait au-dessus des piles (Rouen aussi).
- Tests : `vh4_landmarks_test` (section Orléans ajoutée à la fin), `tools/tests/test_landmarks_v2.py`
  (`test_links_and_origin` accepte une ville sans maquette ; `test_orleans_facts`).

## État
- [x] squelette, décision vue stratégique
- [x] `data/landmarks_v2/orleans.json` : 2 enceintes datées (castrum ≤ 1344, castrum + accrue
      ≥ 1345), 7 portes et poternes, pont de 21 arches, 28 monuments datés (chevet de Sainte-Croix,
      Châtelet, Tourelles, boulevard, bastille Saint-Antoine, Tour Neuve, 11 églises, boulevards de
      1404…), 8 quartiers (faubourgs `until_year` 1428), 6 espaces libres, 438 rues OSM + 3 à la main,
      Loire fine (5 tronçons)
- [x] Moteur (voir ci-dessus), docs `docs/landmarks-v2.md`, addendum ADR 0078
- [x] Captures `docs/img/vh7/` (1337 et `_1429`) : stratégique, transition, vallée, site, pont,
      Tourelles, Sainte-Croix, enceinte, Loire
- [x] Tests : vh4_landmarks_test OK (Orléans compris), zg4_camera OK, zg6_towns OK, smoke OK ;
      pytest complet 730 OK (dont `test_landmarks_v2` 18)
- [x] i/s (machine très chargée, charge ≈ 190) : Orléans 31,5 / 31,2 i/s (d = 1,6 / 0,6), Rouen
      25,5 / 29,4 i/s dans la même session : pas de régression. Plan ≈ 3,5-6 s (fil de travail).

## Limites / points ouverts
- **Butte** : le relief réel monte de 20-25 m entre la Loire (87 m) et la cathédrale (115 m, RGE
  ALTI), avec un talus de 12 m sur 80 m sous la rue de Bourgogne ; le gain de relief local ZG8
  l'exagère en colline à falaises grises : la ville semble perchée, la cathédrale est en partie
  enterrée côté amont. Relève de SZ1 (exagération modulée), pas des données de la ville.
- **Loire sans nappe d'eau** au palier site (SZ2b en cours) : le pont franchit un pré ; les quais et
  le pont tombent bien sur le lit de la carte fine (eau à 86,5-87,6 m entre 47,8951 et 47,8975 N).
- Entre le mur de Loire (47,8983 N) et l'eau fine, une bande de 50-80 m (quais du XVIIIe s. gagnés
  sur le fleuve) : grève figurée par un quai `strand`.
- Rues des faubourgs gardées après 1428 (les rues ne sont pas datées par quartier).
- Pas de faubourgs relevés après 1429 (reconstruction progressive non modélisée).
- Bastilles anglaises de 1428-1429 (Saint-Laurent, Croix-Boissée, Londres, Rouen, Paris,
  Saint-Pouair) non figurées ; arche coupée du pont non figurée.
- Pas de bloc `siege.battle` (ADR 0026) : le siège d'Orléans en bataille reste la ville générique.
- Routes de la carte (rubans) traversent la ville au palier site (comme à Rouen).

## Faits à faire relire (restitutions)
Relus le 2026-09-26 : voir `docs/histoire/relecture-vh-orleans.md` (porte Renart place De Gaulle,
boulevards 1417, bastille Saint-Antoine 1417, nef romane de Sainte-Croix, pont recalé).
- Date de l'accrue du bourg Dunois (1345 retenu ; sources : 1300-1330 ou vers 1356).
- Tracé du mur occidental de l'accrue (≈ 1,9008 E) et place de la porte Renart (axe de la rue des
  Carmes) ; mur nord au sud du Martroi (porte Bannier d'après le nœud OSM « Ancienne porte Bannier »).
- Tracé du castrum (rues Sainte-Catherine, Tour-Neuve, Bourdon-Blanc ; vestiges valentiniens) et
  nom de « porte Dunoise » pour sa porte ouest.
- Pont : 21 arches (Collin) contre 19 dans la demande ; bastille Saint-Antoine et chapelle sur la
  motte : emprise hypothétique.
- Saint-Aignan : rasée en 1358-1359, « reconstruite vers 1420 », rasée en 1428 (une seule source).
- Boulevards à partir de 1404 (une seule source, Inrap) et date du boulevard des Tourelles.
- Gabarit du chevet (58 × 46 m, voûte 30 m) et du Châtelet (48 × 36 m).

## Prochaine étape
Relecture et fusion par l'orchestrateur (après VH5/VH6 : vérifier les conflits dans
`landmark_city_layer.gd`, `town_builder.gd`, `landmark_plan.gd`, `vh4_landmarks_test.gd`). Suites :
bloc de siège ADR 0026, bastilles anglaises, faubourgs relevés.

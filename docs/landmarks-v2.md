# Villes emblématiques à l'échelle 1:1 — format `landmark` v2 (ADR 0078)

Lots VH0 et VH4 (chantier VH, `docs/archive/chantiers.md`). Ce document décrit le format
et le moteur pour les lots qui ajoutent des villes : **VH5 Paris, VH6 Londres, VH7 Orléans**, puis
VH8 (Bordeaux, Avignon, Calais, Bruges). Rouen vers 1340 (`data/landmarks_v2/rouen.json`) sert
d'exemple complet.

## Principe

- En **vue stratégique**, la ville reste la maquette L1/L2 sous loupe radiale
  (`data/landmarks/<id>.json`, glTF cuit par Blender, ADR 0015). Rien ne change pour elle, ni pour
  le décor de siège L3.
- Au **zoom rapproché** (palier vallée, `ZoomTiers.valley_weight`), la ville v2 est planifiée puis
  construite à l'échelle réelle sur le relief fin ; la maquette se dissout (tramage) entre les
  poids vallée 0,35 et 0,65 (`render.fade_valley`), soit vers 8 unités de caméra.
- Le **plancher de caméra provisoire** de ZG4b (`landmark_min_distance`) ne s'applique plus à une
  ville qui a un fichier v2 (`SettlementLayer.landmark_floor_zones`) : la caméra descend jusqu'au
  plancher général du relief (≈ 200 m).
- **Ville sans maquette** (VH7, Orléans) : le champ `landmark` est absent. En vue stratégique, la
  colonie garde sa maquette de colonie ordinaire ; au zoom rapproché, la ville v2 remplace la ville
  ordinaire ZG6 (`TownLayer` ne la construit plus) et ne s'affiche qu'à
  partir du même poids vallée que les villes ZG6 (`TownRenderProfile.min_valley_weight`), quand les
  maquettes des colonies sont masquées (`LandmarkCityLayer.has_maquette`). Pas de plancher ZG4b.
- Tout est **rendu** : aucune règle de jeu, rien dans `core/`.

## Fichier `data/landmarks_v2/<id>.json`

Schéma : `data/schemas/landmark_v2.schema.json` (validé par `tools/tests/test_landmarks_v2.py`).

### Géoréférencement

- `crs` : toujours `EPSG:3035` (la projection de la carte et de toute la carte fine).
- `origin_3035` : [E, N] en mètres, arrondis au mètre. Choisir un point central et stable (Rouen :
  la cathédrale). Le test vérifie qu'il est à moins de 300 m de l'ancre de la maquette v1.
- Toute la géométrie est en **décalages [dE, dN] en mètres, dans les axes de la grille EPSG:3035**
  (x vers l'est de la grille, y vers le nord de la grille). Pas de rotation, pas d'échelle, pas de
  loupe. Attention : le nord de la grille diffère du nord vrai (convergence ≈ 7° à Rouen) ; les
  coordonnées doivent venir d'une vraie projection (pyproj), jamais d'un plan « nord en haut ».
- Angles (`angle_deg`) : antihoraires depuis l'est de la grille (0 = axe vers l'est, 90 = vers le
  nord). Églises : axe vers le chœur.
- `extent_m` : rayon couvert (ville, faubourgs, monuments isolés) autour de l'origine ; sert au
  streaming, à la zone de la caméra et à l'occupation. Toute la géométrie doit y tenir (test).

Conversion en jeu (`LandmarkV2Library`) : unités carte = ((E − minx) / m, (maxy − N) / m) avec
`bounds_projected` de `data/map/map.json` et m ≈ 718,98 m par unité ; repère local du plan = [dE, −dN]
(x vers +X monde, y vers +Z monde = sud), le même que `TownPlan`.

### Sections

| Section | Contenu | Remarques |
|---|---|---|
| `format`, `id`, `name`, `settlement`, `landmark`, `period`, `year`, `seed` | identité ; `landmark` = maquette v1 liée (facultatif : absent pour une ville sans maquette, Orléans) | `seed` : graine du tissu généré |
| `sources` | titre, auteur, date, URL, licence, `extracted`, usage | `extracted: true` seulement si la licence permet l'usage commercial (OSM, IGN…) ; plans Gallica, Agas/MoEML, Cassini, Open Domesday : `false` (contrôle humain) |
| `plan` | façades et profondeurs (ville close, faubourgs), largeurs de rue par rang, mélange de maisons, `detail_cell_m` | défauts : 5-8 m × 20-40 m, faubourgs 8-16 × 25-50, cellules de 250 m |
| `osm_streets` | recette d'extraction OSM : `bbox_lonlat`, `highways`, `exclude`, `main`, `lanes`, `alleys`, `unnamed`, `simplify_m` | voir « Outil » |
| `alpage` | recette ALPAGE (Paris seulement) : `streets` (rues de 1380) et `parcels` (parcelles Vasserot) | voir « Paris et ALPAGE » |
| `fine_rivers` | noms des fleuves de la carte fine recopiés dans `waters` | défaut : Seine ; `[]` : pas de fleuve fin (Paris) |
| `waters` | cours d'eau `[dE, dN, largeur]` ou lit en `polygon` (+ `holes` : îles), `origin` (`rivers_fine`, `osm`, `alpage`, `hand`), `draw` | le fleuve principal vient de la carte fine (`draw: false`, couloir interdit seulement) ; un ruisseau absent de la carte (Robec) est tracé à la main et dessiné (`draw: true`) |
| `walls` | enceintes : `points`, `closed`, hauteur, épaisseur, tours (espacement, rayon, hauteur), `ditch_m`, `gates` (`name`, `at`, `kind`, `height_m`, dates : Moorgate à partir de 1415), dates, `note` | une enceinte ouverte (mur de rive) est une polyligne `closed: false` ; plusieurs enceintes datées possibles (Paris : Philippe Auguste, Charles V à partir de 1356) |
| `streets` | rues : `rank` (`main`, `secondary`, `lane`), `width_m`, `origin` (`osm` ou `alpage` régénérés par l'outil, `hand` gardé), dates | tracer à la main les rues disparues (percées ultérieures, reconstruction) |
| `quays` | quais : polyligne, largeur, `kind` | bande pavée sans maisons |
| `bridges` | `from`, `to`, largeur, hauteur du tablier au-dessus de l'eau, arches, `houses` + `house_span`, `gatehouses_at`, `mills_at`, `chapel_at` + `chapel_side`, `drawbridge_at`, `pier_m`, `starling_m`, dates | pont habité : maisons des deux côtés du tablier, base = tablier ; travées libres au pont-levis et à la chapelle ; chapelle sur la pile (base fixe au-dessus de l'eau) ; rampes d'accès jusqu'aux rives basses ; niveaux recalculés avec les pages de relief plus fines (VH6) ; `arches` donne le nombre de piles (arches − 1, VH7) ; tablier posé sur la base des piles puis relevé en mètres (VH7) |
| `monuments` | gabarit (`model`), `params` en mètres, `at`, `angle_deg`, `clear_m`, dates, `certainty` (`attested`, `probable`, `hypothetical`), `description` | voir « Gabarits » |
| `districts` | quartiers : `zone` (`intra`, `faubourg`), `polygon`, `density` (part des façades occupées), `houses` (mélange du kit), `roofs` (réservé) | hors de tout quartier, aucune parcelle ; le premier quartier qui contient un point l'emporte |
| `open_spaces` | places, marchés, parvis, cimetières (aîtres), jardins, prés, cloîtres, grèves | sans maisons ; places pavées ; arbres dans jardins, cimetières, prés |
| `parcels` | parcellaire importé : `[dE, dN, angle°, façade, profondeur]` (milieu de la façade sur rue, normale vers l'intérieur) | Paris : parcelles Vasserot d'ALPAGE (ODbL) ; placées avant les lanières générées, qui comblent les façades restées libres |
| `render` | `fade_valley` | fenêtre du fondu de la maquette |

Éléments datés : `from_year` / `until_year` inclus, filtrés par l'année de la partie
(`LandmarkCityLayer.set_year` replanifie si un élément apparaît ou disparaît). Rouen : beffroi
communal jusqu'en 1382, beffroi du Gros-Horloge à partir de 1389, aître Saint-Maclou à partir de 1348.

### Gabarits de monuments (`LandmarkMonuments`)

Repère du gabarit : +X le long de l'axe, +Z à droite de l'axe, y = 0 à la base (point le plus bas
de l'emprise), murs prolongés de 8 m sous terre pour les pentes. Longueurs en mètres réels.

| `model` | Paramètres |
|---|---|
| `gothic_cathedral` | `length_m`, `width_m` (nef et bas-côtés), `nave_width_m`, `vault_m` (égout de la nef), `ridge_m`, `aisle_m`, `transept_m`, `transept_width_m`, `transept_at` (fraction depuis l'ouest), `apse` (`round`, `flat`), `chapels`, `bays`, `west_towers` [{`side` (`north`, `south`), `size`, `height`, `top` (`flat`, `pyramid`, `spire`), `spire_m`}], `crossing` {`size`, `height`, `spire_m`}, `open_west` (chœur seul, nef ancienne à part) |
| `church` | `length_m`, `width_m`, `height_m`, `apse`, `tower` {`at` (`west`, `crossing`), `size`, `height`, `spire_m`} |
| `abbey` | ceux de `church` + `cloister` {`side` (`north`, `south`), `size`} |
| `castle` | `ring` [[u, v]…] (u le long de l'axe, v à gauche), `height_m`, `thickness_m`, `tower_radius_m`, `tower_height_m`, `keep` {`at`, `radius_m`, `height_m`}, `halls` [[u, v, longueur, largeur, hauteur, angle°]…] |
| `belfry` | `size`, `height`, `top` (`pyramid`, `lantern`, `turrets` : toit plat et tourelles d'angle, VH6) |
| `hall` | `length_m`, `width_m`, `height_m` (halle : murs bas, grand toit) |
| `royal_palace` | `length_m`, `width_m`, `height_m`, `ranges` [[u, v, longueur, largeur, hauteur, angle°]…] |
| `enclosure` | `length_m`, `width_m`, `height_m`, `ranges` (bâtiments le long des grands côtés) |
| `keep`, `tower`, `gate_tower` | `radius_m` ou `size`, `height` |
| `earthwork` (VH7) | `ring` [[u, v]…] (comme `castle`), `bank_m` (hauteur de la levée), `base_m` (largeur à la base), `palisade_m`, `closed` (faux : fer à cheval ouvert côté porte ou fort) : boulevard de terre et de bois |

Un monument phasé se découpe en plusieurs entrées (Saint-Ouen en 1340 : chœur gothique de
1318-1339 en `gothic_cathedral` avec `open_west`, nef romane en `church`, cloître en `enclosure`).

## Outil `cent-ans geo landmarks`

```
uv run --project tools cent-ans geo landmarks [--city rouen] [--refresh-osm]
```

(`tools/cent_ans_tools/geo/landmarks_v2.py`)

1. **Rues OSM** : extrait Overpass des voies de `osm_streets.bbox_lonlat`, mis en cache dans
   `tools/geo/raw/osm/<id>_highways.json` (non versionné) ; garde les `highways` listés, retire
   tunnels, ponts, aires et les noms de `exclude` (percées du XIXᵉ s., boulevards sur les remparts
   arasés, quais modernes, rues de la reconstruction), coupe aux quartiers (+25 m), simplifie,
   projette en [dE, dN]. Rang : `main` si le nom est dans `main`, `lane` si dans `lanes` ou sans
   nom. Remplace les rues `origin: "osm"`, garde les rues `hand`.
2. **Fleuve fin** : lit les tuiles `hydro_fine` (E2) de la pyramide, garde les fleuves de
   `fine_rivers`, recoud les morceaux, rééchantillonne tous les 40 m avec la largeur ; remplace
   les eaux `origin: "rivers_fine"`. Les quais et l'eau affichée coïncident ainsi par construction.
3. Réécrit le fichier (JSON indenté, un point par ligne).

Pour une nouvelle ville : écrire à la main (en s'aidant de pyproj pour projeter des lon/lat)
l'origine, les enceintes, portes, quais, ponts, monuments, quartiers et places, la recette
`osm_streets`, puis lancer l'outil ; relancer les tests.

## Moteur (Godot, `game/scripts/map/`)

| Fichier | Rôle |
|---|---|
| `landmark_v2_library.gd` | lecture de `data/landmarks_v2/`, conversions EPSG:3035 → unités carte → repère local, filtre des dates |
| `landmark_plan.gd` | plan pur (fil de travail), sortie au format `TownPlan` : couloirs d'eau, monuments réservés, enceintes polygonales (normales vers l'extérieur, fossé, tours, portes), pont (tablier au-dessus de l'eau du relief, maisons du pont), quais, places, rues réelles, **parcelles en lanières** le long de chaque rue dans son quartier (façade et profondeur tirées, maison sur rue, cour ou jardin, annexe au fond), sol des quartiers (faubourgs : seulement près des maisons) |
| `landmark_monuments.gd` | gabarits réels (tableaux de maillage préparés dans le fil du plan, couches de l'atlas BR1) |
| `landmark_city_layer.gd` | streaming (même profil que ZG6, `town_render.tres`), construction par étapes, recalage des hauteurs quand des pages plus fines arrivent, fondu de la maquette, `set_year` |
| `town_builder.gd` (ZG6, étendu) | construit le plan : maisons du kit bas détail BR1 de près et blocs au-delà (HLOD par maison dans le shader), nœuds par cellule d'îlots de `detail_cell_m`, `wall_rings` (enceintes quelconques avec parapet), monuments v2 fusionnés par cellule de 600 m (`_extra_meshes`, `merge_monuments` : un maillage par cellule, base, ancrage et teinte de chaque monument dans `CUSTOM0`, `base_source` 2 de `town_building.gdshader` ; RS-G, Paris ≈ 110 monuments), rues pavées et ruisseaux |

Hauteurs : mètres non exagérés, posés par `town_building.gdshader` à la hauteur affichée ZG8
(`campaign_display_height`), la même que le terrain. Sans pyramide, la couche reste inactive
(maquette seule).

Captures et mesure :
`godot --path game --script res://tests/vh4_shots.gd -- --out=<dossier> --map-weather=clear`. Test headless :
`res://tests/vh4_landmarks_test.gd`.

## Ce que chaque ville (VH5-VH8) doit fournir

1. **Origine** EPSG:3035 et `extent_m` (Paris : ≥ 3 km pour les faubourgs ; Londres : Westminster
   est à 3 km de la Cité, prévoir une enceinte ouverte et un quartier séparé le long du Strand).
2. **Enceintes datées** avec portes (noms, positions) ; murs de rive en polylignes ouvertes.
   Paris : Philippe Auguste (deux rives, jusqu'à la Seine), Charles V (rive droite, 1356-1383).
3. **Fleuve(s)** : nom(s) dans `fine_rivers` ; vérifier que la Seine, la Tamise ou la Loire fines
   passent bien là où sont les quais (voir limites) ; îles comme quartiers ou espaces libres.
4. **Ponts** : Paris (Grand-Pont et Petit-Pont habités, moulins), Londres (London Bridge :
   maisons, chapelle Saint-Thomas, pont-levis), Orléans (pont des Tourelles, boulevard).
5. **Monuments** à gabarit réel avec dates et degré de certitude (Notre-Dame, Sainte-Chapelle,
   Palais, Louvre de Philippe Auguste puis de Charles V ; Tour de Londres, Old St Paul's ≈ 150 m ;
   Sainte-Croix d'Orléans : chevet seul).
6. **Quartiers** (densité, mélange de maisons) et **places** ; faubourgs datés (Orléans : faubourgs
   rasés en 1428 → `until_year: 1428`).
7. **Recette OSM** (`exclude` : percées haussmanniennes à Paris, rues de la reconstruction ; à
   Londres, la voirie d'après 1666 suit en gros le tracé médiéval mais pas partout) ; rues
   disparues tracées à la main (`origin: "hand"`), à partir de faits (plans en domaine public,
   ALPAGE pour Paris), jamais en copiant un plan non commercial.
8. Paris peut remplir `parcels` avec le parcellaire ALPAGE (ODbL) : à implémenter dans
   `LandmarkPlan` (VH5) à la place de `_line_parcels`.

## Londres vers 1340 (VH6)

`data/landmarks_v2/london.json` : origine = ancre de la maquette (Temple), `extent_m` 2 800 (Tour à
2,4 km à l'est, Westminster et Lambeth à 2 km au sud-ouest). Mur ouvert sur la Tamise (le mur de
rive romain était ruiné au XIIe s.), de la poterne de Tower Hill au débouché de la Fleet
(prolongement vers 1280-1320 pour les Dominicains) ; 6 portes + Moorgate datée 1415. Tour :
deux gabarits `castle` concentriques (tours aux sommets, positions OSM), Tour Blanche en `belfry`
à tourelles, douves tracées à la main. London Bridge : 19 arches, piles de 7 m et avant-becs,
maisons, chapelle Saint-Thomas côté aval, pont-levis, deux portes. Westminster : chevet gothique
d'Henri III + nef romane (jusqu'en 1375, nef gothique ensuite), Westminster Hall, Saint-Étienne,
tour du Joyau (1366), horloge (1367). Rues OSM avec exclusions (percées du XIXe s., reconstruction
après 1666 : King William Street, Queen Victoria Street, Cannon Street, etc.) et rues disparues à
la main (Candlewick Street, King Street de Westminster, Old Change, accès du pont, Snow Hill).
Suivi : `docs/archive/chantiers.md`.

## Orléans vers 1340-1429 (VH7)

`data/landmarks_v2/orleans.json`, origine sur Sainte-Croix, `extent_m` 1 850 (jusqu'à Saint-Loup).
Suivi : `docs/archive/chantiers.md` ; captures `docs/img/vh7/` (`res://tests/vh7_shots.gd`,
`--year=1429` pour l'état du siège).

- **Vue stratégique** : Orléans n'a pas de maquette L1/L2 ; la colonie ordinaire (maquette de ville
  sous loupe) reste l'image lointaine, la ville 1:1 remplace la ville ZG6 au palier vallée (voir
  « Principe »). Une petite maquette L1/L2 (et un bloc `siege.battle`, ADR 0026) reste possible
  plus tard ; elle ferait basculer Orléans dans le cas des autres villes (fondu).
- **Enceintes datées** : castrum (2 032 m, 25 ha, portes Bourgogne, Parisis, Dunoise et du Pont)
  jusqu'en 1344 ; à partir de 1345, castrum et accrue du bourg Dunois (portes Bannier et Renart) :
  c'est l'enceinte du siège de 1428-1429 (la grande accrue de 1467-1480 n'existe pas encore).
  Deux anneaux fermés datés plutôt qu'un mur ouvert : les normales et les fossés restent justes.
  Porte Renart sous la place De Gaulle, mur ouest le long du vallon de la rue
  Notre-Dame-de-Recouvrance (relecture historique : `docs/histoire/relecture-vh-orleans.md`).
- **Pont des Tourelles** (21 arches jusqu'en 1434-1435, axe de la rue des Hôtelleries), Châtelet,
  chapelle Saint-Antoine sur la motte, bastille Saint-Antoine en bois à partir de 1417, fort des
  Tourelles, boulevard de terre et de bois (`earthwork`) ; boulevards des portes à partir de 1417
  (porte Renart 1418) (emprises hypothétiques).
- **Sainte-Croix** : chevet gothique (`gothic_cathedral`, `open_west`) raccordé à la cathédrale
  romane (nef, façade à deux tours, en `church`), remplacée seulement aux XVe-XVIe s.
- **Faubourgs et églises hors les murs** (Saint-Aignan, Saint-Euverte, Saint-Laurent, Saint-Paterne,
  Saint-Vincent, Saint-Marceau, couvent des Augustins) : `until_year` 1428 ; Saint-Loup,
  Saint-Jean-le-Blanc et la bastille anglaise des Augustins (prises en mai 1429) jusqu'en 1429.
- **Rues** : 438 rues OSM ; exclusions (rue Jeanne-d'Arc, rue Royale, rue de la République,
  boulevards des XVIIIe-XIXe s., quais, rues de la reconstruction d'après 1940) ; trois chemins
  tracés à la main (Portereau, Saint-Marceau, chemin de Blois).
- **Loire** : la carte fine donne cinq tronçons larges (≈ 350 m) qui se recouvrent ; ils ne servent
  qu'au couloir interdit (`draw: false`) ; `LandmarkPlan` indexe désormais les segments d'eau par
  cases de 40 m (polygones de Paris compris) (sol des quartiers).

## Paris et ALPAGE (VH5)

Le consortium ALPAGE (LAMOP-Paris 1, dir. H. Noizet) publie sous ODbL 1.0, en téléchargement libre
(https://alpage.huma-num.fr/gis-data/), un SIG de Paris : **« Paris en 1380 »** (P. Rouet : voies,
îlots, usages du sol, hydrographie) et les **données Vasserot** (A.-L. Bethe : parcelles
1810-1836). `tools/cent_ans_tools/geo/alpage.py` lit ces GeoPackage (EPSG:2154, cache
`tools/geo/raw/alpage/`, téléchargés au besoin) :

- **Rues** : le réseau de 1380 remplace la recette OSM (plus besoin d'exclure les percées
  haussmanniennes, la rue de Rivoli ou les boulevards : ils n'y sont pas). Rang `main` pour les
  axes majeurs ALPAGE (`AXE_MAJEUR`), `lane` pour les ruelles et voies sans nom ; recette
  `alpage.streets` : `skip_regions` (`PONT` : ponts dans `bridges`), `exclude` (quais, tracés dans
  `quays`), `main`, `simplify_m`. Coupées aux quartiers (+ 25 m) comme les rues OSM.
- **Parcelles** : le cadastre Vasserot est postérieur de cinq siècles ; il ne sert que de gabarit
  des lanières. Une parcelle est gardée si elle est dans un quartier de `alpage.parcels.districts`
  (Cité, Ville, Université), si sa surface est plausible (15-2 500 m²), si aucune rue de 1380 ne la
  traverse, si elle ne recouvre pas (> 30 %) un usage du sol non résidentiel de 1380 (églises,
  couvents, palais, marchés, cimetières, eau, champs, enceintes) et si l'un de ses côtés longe
  une rue de 1380 (à ≤ `reach_m` du bord, parallèle à 30° près). ≈ 4 800 parcelles sur ≈ 6 800
  dans la ville close.
- **Le reste** du fichier (enceintes, portes et poternes, emprises et orientations des églises,
  abbayes, palais, Louvre, Châtelets, Temple, halles ; quartiers tirés des îlots de 1380 et des
  zones bâties hors les murs ; îles, cimetières, prés ; lit de la Seine de 1380) a été écrit une
  fois à partir des géométries ALPAGE puis est maintenu à la main.

La Seine de la carte fine (axe à largeur) est trop large (≈ 130 m) et passe sur le nord de la
Cité : Paris met `fine_rivers: []` et décrit le lit de 1380 en polygones (`waters.polygon`, îles en
`holes`), interdits en entier aux maisons. Le rendu de l'eau reste celui de la carte fine.

Moteur (rétrocompatible, Rouen inchangé) : index en grille des quartiers et des eaux
(`LandmarkPlan.Districts.build_index`, `_water_index`), eaux en polygone, parcellaire importé
(`_imported_parcels` : façade partagée en maisons de ≤ 11 m, profondeur réduite si la parcelle
touche un monument ou une muraille), minutage par étape (`stats.marks_usec`). Plan de Paris :
≈ 2-5 s dans le fil de travail selon la charge (Rouen : ≈ 2-3 s), ≈ 8 800 maisons, 110 monuments.
Captures : `godot --path game --script res://tests/vh4_shots.gd -- --city=paris --out=<dossier>`.

## Bordeaux, Avignon, Calais, Bruges (VH8, lot RS-G)

Mêmes règles que Rouen ; chaque fichier liste ses sources, et les manques sont écrits dans les
`note` et `description` (éléments `hypothetical`). Suivis : `docs/wip/rs-g-<ville>.md` ; relectures
historiennes : `docs/histoire/relecture-vh-<ville>.md`. Tests : `tools/tests/test_landmarks_v2_<ville>.py`
et `_test_vh8` dans `res://tests/vh4_landmarks_test.gd` (plan, portes, eau, monuments fusionnés).

- **Bordeaux** : troisième enceinte (1302-1327) en deux polylignes ouvertes (front de terre avec
  fossé, mur de Garonne sans fossé), castrum et deuxième enceinte intérieurs ; Garonne fine,
  Peugue et Devèze à la main ; Pey-Berland à partir de 1440, Saint-Michel ancienne jusqu'en 1429.
- **Avignon** : enceinte du XIIIe s. ouverte, remparts d'Innocent VI et d'Urbain V à partir de 1357,
  pont Saint-Bénézet en deux entrées `bridges` (pont coudé), palais Vieux 1335 / Neuf 1342, fort
  Saint-André 1362 ; bras de Villeneuve (OSM) et Sorgue dessinée.
- **Calais** : enceinte de Hurepel (1228), havre en polygone `hand` (trait de côte de 1340 non
  sourcé : restitution minimale), Notre-Dame en trois phases, Rysbank ; trame OSM de la
  reconstruction, peu sûre.
- **Bruges** : levée de 1297 démantelée en 1328 (portes basses en 1340, portes de pierre datées
  1361-1401), beffroi en trois états, Waterhalle, reien d'après OSM dessinées (`fine_rivers: []`).

Manques de format relevés (contournés) : pas de `certainty` ni de `note` sur les portes, pas de
levée de terre linéaire, pas de pont coudé, eaux non datées, polygones d'eau non dessinés par le
moteur (l'eau affichée reste celle de la carte fine).

## Limites connues (VH4)

- Parcellaire généré le long des rues OSM nommées (pas de cadastre réel) ; ≈ 4 800 parcelles et
  5 800 bâtiments pour Rouen ; les îlots très petits restent en partie vides (cours).
- Couvertures (`roofs`) non rendues : un toit par modèle du kit bas détail.
- Arbres des jardins calculés (`plan.trees`) mais non dessinés (comme ZG6).
- Le fleuve de la carte fine peut s'écarter du vrai lit (Rouen : la Seine fine passe par le bras
  sud de l'île Lacroix à l'est du pont : une bande de terre sépare le mur de rive de l'eau à l'est
  de la cathédrale). Correction dans `hydro_fine`, pas dans la ville.
- L'exagération du relief au palier vallée (défaut S1, lot SZ1) fait des côtes de Rouen des murs de
  plusieurs centaines de mètres ; la ville elle-même est posée au bon endroit, au pied des côtes.

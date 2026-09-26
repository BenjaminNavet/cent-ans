# Villes emblématiques à l'échelle 1:1 — format `landmark` v2 (ADR 0078)

Lots VH0 et VH4 (chantier VH, `docs/wip/vh-villes-historiques.md`). Ce document décrit le format
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
| `format`, `id`, `name`, `settlement`, `landmark`, `period`, `year`, `seed` | identité ; `landmark` = maquette v1 liée | `seed` : graine du tissu généré |
| `sources` | titre, auteur, date, URL, licence, `extracted`, usage | `extracted: true` seulement si la licence permet l'usage commercial (OSM, IGN…) ; plans Gallica, Agas/MoEML, Cassini, Open Domesday : `false` (contrôle humain) |
| `plan` | façades et profondeurs (ville close, faubourgs), largeurs de rue par rang, mélange de maisons, `detail_cell_m` | défauts : 5-8 m × 20-40 m, faubourgs 8-16 × 25-50, cellules de 250 m |
| `osm_streets` | recette d'extraction OSM : `bbox_lonlat`, `highways`, `exclude`, `main`, `lanes`, `alleys`, `unnamed`, `simplify_m` | voir « Outil » ; `alleys` (VH6) garde en venelles les allées et cours nommées cartographiées en chemins piétons (… Alley, Court, Passage, Yard, Row, Churchyard) |
| `fine_rivers` | noms des fleuves de la carte fine recopiés dans `waters` | défaut : Seine |
| `waters` | cours d'eau `[dE, dN, largeur]`, `origin` (`rivers_fine`, `osm`, `hand`), `draw` | le fleuve principal vient de la carte fine (`draw: false`, couloir interdit seulement) ; un ruisseau absent de la carte (Robec) est tracé à la main et dessiné (`draw: true`) |
| `walls` | enceintes : `points`, `closed`, hauteur, épaisseur, tours (espacement, rayon, hauteur), `ditch_m`, `gates` (`name`, `at`, `kind`, `height_m`, dates : Moorgate à partir de 1415), dates, `note` | une enceinte ouverte (mur de rive) est une polyligne `closed: false` ; plusieurs enceintes datées possibles (Paris : Philippe Auguste, Charles V à partir de 1356) |
| `streets` | rues : `rank` (`main`, `secondary`, `lane`), `width_m`, `origin` (`osm` régénéré par l'outil, `hand` gardé), dates | tracer à la main les rues disparues (percées ultérieures, reconstruction) |
| `quays` | quais : polyligne, largeur, `kind` | bande pavée sans maisons |
| `bridges` | `from`, `to`, largeur, hauteur du tablier au-dessus de l'eau, arches, `houses` + `house_span`, `gatehouses_at`, `mills_at`, `chapel_at` + `chapel_side`, `drawbridge_at`, `pier_m`, `starling_m`, dates | pont habité : maisons des deux côtés du tablier, base = tablier ; travées libres au pont-levis et à la chapelle ; chapelle sur la pile (base fixe au-dessus de l'eau) ; rampes d'accès jusqu'aux rives basses ; niveaux recalculés avec les pages de relief plus fines (VH6) |
| `monuments` | gabarit (`model`), `params` en mètres, `at`, `angle_deg`, `clear_m`, dates, `certainty` (`attested`, `probable`, `hypothetical`), `description` | voir « Gabarits » |
| `districts` | quartiers : `zone` (`intra`, `faubourg`), `polygon`, `density` (part des façades occupées), `houses` (mélange du kit), `roofs` (réservé) | hors de tout quartier, aucune parcelle ; le premier quartier qui contient un point l'emporte |
| `open_spaces` | places, marchés, parvis, cimetières (aîtres), jardins, prés, cloîtres, grèves | sans maisons ; places pavées ; arbres dans jardins, cimetières, prés |
| `parcels` | réservé à VH5 | parcellaire importé (ALPAGE, ODbL) à la place du parcellaire généré |
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
| `town_builder.gd` (ZG6, étendu) | construit le plan : maisons du kit bas détail BR1 de près et blocs au-delà (HLOD par maison dans le shader), nœuds par cellule d'îlots de `detail_cell_m`, `wall_rings` (enceintes quelconques avec parapet), monuments v2 (`_build_extra`), rues pavées et ruisseaux |

Hauteurs : mètres non exagérés, posés par `town_building.gdshader` à la hauteur affichée ZG8
(`campaign_display_height`), la même que le terrain. Sans pyramide, la couche reste inactive
(maquette seule).

Options : `--no-landmarks-1to1` (rendu d'avant VH4). Captures et mesure :
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
Suivi : `docs/wip/vh6-londres.md`.

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

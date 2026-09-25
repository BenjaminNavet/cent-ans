# ADR 0036 — Carte de campagne zoomable : pyramide de relief streamée jusqu'à 1-5 m

- Statut : accepté (chantier ZG, « zoom géographique »)
- Date : 2026-09-25

## Contexte

Le joueur veut « voir des montagnes, des vallées, des villes » en zoomant sur la carte de campagne.
La carte est déjà à l'échelle géographique réelle (EPSG:3035, 2 945 × 2 945 km, 1 unité monde =
1 pixel 4096 = 719 m ; relief ×4,3 à la verticale via `MapData.HEIGHT_SCALE = 0.006`), mais sa
**résolution** plafonne : surface de rendu à 719 m/px, tuiles fines 8192² à 360 m/px (ADR 0019)
alors que la source Copernicus GLO-90 (90 m) est déjà en cache ; caméra arrêtée à 22 unités
(≈ 16 km, ~40 km à l'écran ; 7 unités au-dessus des villes emblématiques). Une vallée de 1 km,
un éperon de château, le site d'une ville n'existent pas à cette résolution.

ADR 0019 avait écarté 16384² pour deux raisons encore valables : poids dans le dépôt (plafond
~150 Mo de textures versionnées) et maillages de 1025² sommets construits en GDScript trop lents.
Aller plus fin impose donc de changer d'architecture, pas seulement de source.

Le joueur a validé trois paliers et demandé d'aller jusqu'au troisième :

| Palier | Résolution | Emprise | Source |
|---|---|---|---|
| 1 | 180 → 90 m | toutes les terres de l'emprise Copernicus (lon −11 → 12, lat 41 → 60) | Copernicus GLO-90 (en cache) |
| 2 | 45 → 22,5 m | « cœur » : France, Angleterre et pays de Galles, Bénélux, Rhénanie (lon −6 → 9, lat 42 → 56), terres seules | Copernicus GLO-30 |
| 3 | 11 → 2,8 m | zones de détail (villes, sièges et batailles célèbres, forteresses) | MNT nationaux : IGN RGE ALTI (France), LiDAR Environment Agency (Angleterre), AHN (Pays-Bas), DHM Vlaanderen, Wallonie |

## Décision

### Pyramide de relief en étages de tuiles

- Un seul mécanisme pour tous les paliers : une **pyramide quadtree** de tuiles 512² pixels
  16 bits (même format que `data/map/height/h_{col}_{row}.png` : PNG `I;16`, hauteur normalisée
  sur `[height_min_m, height_min_m + 5000]`, pixel centré, grille EPSG:3035 de `map.json`).
- **Étage k** : une tuile couvre `256 / 2^k` unités monde, soit `360 / 2^k` m/px. E0 = les tuiles
  actuelles (360 m, 16 × 16, versionnées) ; E1 180 m ; E2 90 m ; E3 45 m ; E4 22,5 m ; E5 11,2 m ;
  E6 5,6 m ; E7 2,8 m. Tuile (k, col, row) : enfants (k+1, 2col+dx, 2row+dy).
- **Pyramide creuse** : une tuile n'existe que là où un étage a une vraie source (pas de tuile en
  mer au-delà de E0, pas d'E3-E4 hors du cœur, pas d'E5-E7 hors des zones de détail). Le moteur
  retombe sur l'ancêtre le plus fin disponible.
- **Continuité** : chaque étage est filtré par moyenne de zone depuis la meilleure source, avec le
  même rehaussement de rendu que `heightmap_render.png` (ADR 0019, σ 5 km, ±120 m, effacé en
  altitude), calculé une fois sur la source la plus fine puis moyenné : un enfant moyenné 2 × 2
  redonne son parent à la quantification près. Le trait de côte reste celui d'E0 (terre > 0,5 m)
  aux étages ≤ E2 ; au-delà, côte de la source, raccordée à l'eau par un fondu de 2 pixels.
- **Manifeste versionné** `data/map/relief_pyramid.json` (schéma `relief_pyramid.schema.json`) :
  étages, m/px, sources, emprises, liste compacte des tuiles présentes par étage (bitmap RLE),
  empreinte (taille, somme de contrôle) du cache attendu.
- **Tuiles non versionnées** dans `data/map/pyramid/E{k}/{col}_{row}.png` (gitignoré), produites
  par `uv run --project tools cent-ans geo pyramid` (paliers 1-2) et `geo detail-dem` (palier 3).
  Les worktrees d'agents lient `data/map/pyramid` et `tools/geo/raw` au dépôt principal par lien
  symbolique pour ne télécharger et calculer qu'une fois. Sans cache, le jeu tourne comme
  aujourd'hui (E0 seul) : le test de fumée ne dépend pas du cache.
- **Plafonds** : téléchargements bruts ≤ 20 Go (hors dépôt), cache de pyramide ≤ 4 Go, mesurés
  et consignés dans `docs/wip/zg-zoom-geographique.md`. Si le palier 2 dépasse, on réduit
  l'emprise du cœur avant de dégrader la résolution.

### Sources et anachronismes

- **GLO-30 est un modèle de surface** (canopée, bâtiments, retenues d'eau modernes). FABDEM
  (qui les retire) est sous licence non commerciale, incompatible avec un jeu sans licence
  arrêtée : on garde GLO-30 et on le **corrige** : hauteur de canopée retirée sous la couverture
  arborée d'ESA WorldCover 10 m (CC BY 4.0), bâti moderne aplani, plans d'eau des barrages
  postérieurs à 1340 (liste sourcée) remplacés par une interpolation du fond de vallée depuis
  les berges.
- **Les MNT de 1-5 m montrent le XXIᵉ siècle** : remblais et déblais d'autoroutes et de voies
  ferrées, carrières, canaux, digues modernes, retenues. Le palier 3 les **efface** avant cuisson
  à partir d'OpenStreetMap (autoroutes, voies ferrées, carrières, `landuse=quarry|landfill`,
  réservoirs), par un masque dilaté et une interpolation lissée (Laplace) depuis les bords.
  Ce qui reste (terrasses, talus, fossés anciens, mottes) est gardé.
- Les zones de détail sont dans `data/map/detail_zones.json` (schéma `detail_zones.schema.json`) :
  identifiant, nom, centre lon/lat, demi-côté en km, étage maximal, source, justification
  historique. Première liste : les 7 villes emblématiques (Paris, Londres, Avignon, Calais, Rouen,
  Bordeaux, Bruges), les batailles et sièges célèbres (Crécy, Poitiers, Azincourt, L'Écluse,
  Auray, Formigny, Castillon, Orléans, Harfleur, Cocherel...), quelques forteresses (Château-
  Gaillard, Vincennes, Carcassonne, Mont-Saint-Michel, Douvres...). Ce sont aussi les sites de
  EP2/EP7 (batailles épiques), qui liront la pyramide au lieu d'un téléchargement propre.
- Licences : Copernicus DEM (attribution), ESA WorldCover (CC BY 4.0), IGN (Licence Ouverte 2.0),
  Environment Agency (Open Government Licence v3), AHN (CC0), Vlaanderen (licence ouverte
  modèle Vlaanderen), Wallonie (CC BY 4.0 selon le jeu de données), OpenStreetMap (ODbL, masques
  seulement, pas de données redistribuées), EU-Hydro (Copernicus). Chaque source vérifiée par
  l'agent qui l'intègre et consignée dans `CREDITS.md` et l'écran des crédits. Tout coût
  (normalement nul) dans `docs/budget.md`.

### Moteur : quadtree streamé, déplacement au GPU

- `TerrainBuilder` garde ses 16 × 16 morceaux E0 lointains et proches. Au-dessus, un
  **quadtree** (`game/scripts/map/relief_quadtree.gd` + `relief_pyramid.gd`) choisit les nœuds
  par erreur à l'écran (taille projetée d'un pixel de l'étage ≤ ~2 px, avec hystérésis) et
  remplace l'actuel « relief fin » (`FineTerrainJob`), dont il reprend les services
  (`surface_height_at`, `chunk_surface_changed`, cache LRU, jupes contre les fissures).
- **Géométrie** : un maillage de patch fixe partagé, déplacé dans le vertex shader à partir
  d'une page de hauteurs (texture 32 bits flottants ou 16 bits reconstruits, pas de demi-flottant :
  sa précision de 2-4 m à 4 000 m ruinerait les étages fins), au lieu d'un maillage construit
  par tuile en GDScript (le goulot mesuré par ADR 0019). Pages dans un `Texture2DArray` à
  taille fixe (budget VRAM ≤ 256 Mo), table de pages en uniforme.
- **Décodage** hors fil principal ; en Rust (`GameDataStore` décode déjà les PNG 16 bits) si le
  GDScript ne tient pas 60 i/s en panoramique au palier 2. Le processeur garde les octets des
  tuiles chargées pour `surface_height_at` (pose des objets, sélection, lignes de vue de rendu).
- **Précision** : à 2,8 m/px une unité monde vaut 256 pixels ; les flottants 32 bits autour de
  4 096 unités donnent ~0,35 m : suffisant jusqu'à E7, pas au-delà (pas d'étage E8).

### Caméra et échelle verticale

- Zoom minimal : 22 unités aujourd'hui → **≈ 5 unités** partout au palier 1, **≈ 1,5 unité**
  (~1 km) sur le cœur au palier 2, **≈ 0,3 unité** (~200 m) dans les zones de détail au
  palier 3 (généralise `close_zones`, lot L1). La distance minimale suit l'étage le plus fin
  disponible sous la caméra ; plan proche ajusté.
- **Exagération verticale dynamique** : ×4,3 en vue stratégique (lisible de loin), ramenée
  progressivement vers ×1,5 au zoom maximal (sinon un talus de 10 m devient une falaise de 43 m).
  Un seul propriétaire (`MapData.vertical_scale()` + signal), relu par le shader et par
  `surface_height_at` ; les calques qui posent des objets réagissent au signal comme ils
  réagissent déjà à `chunk_surface_changed`.
- Paliers de zoom (lot C6, `zoom_tiers.tres`) étendus : au-delà du « comté », vues « vallée »
  (~5 km) et « site » (~1 km).

### Ce qui doit suivre le relief fin

- **Hydrographie** : les fleuves Natural Earth 10m ne tombent pas dans des fonds de vallée de
  22 m. Réseau EU-Hydro (ou tracé par écoulement sur le relief, conditionné par EU-Hydro), rubans
  de fleuves en maillage au palier 2-3, lit creusé cohérent. Côtes et lacs raccordés.
- **Routes, colonies, hameaux, ponts** recollés au sol le plus fin chargé.
- **Couverture du sol de près** : `splat.png` (719 m/px) reste la tendance régionale ; le
  shader ajoute au palier 2-3 un parcellaire procédural (champs ouverts, bocage, vignes en
  terrasses, lisières) guidé par la pente, l'exposition et la distance aux villages.
- **Villes à l'échelle** : empreinte réelle vers 1340 (surface intra-muros sourcée, enceinte,
  faubourgs, finage), avec le kit de bâtiments BR1 ; les villes emblématiques gardent leurs
  maquettes L1/L2.

### Le cœur n'est pas touché

Aucune règle ne lit la pyramide : `heightmap.png`, `navgrid.png` et `core/` restent inchangés
(ADR 0019). Tout le chantier est du rendu, des données hors ligne et de l'outillage.

## Lots

| Lot | Contenu | Dépend de |
|---|---|---|
| ZG0 | Squelette : ADR, manifeste et schémas, stubs Python et GDScript, wip | — |
| ZG1 | Données paliers 1-2 : `geo pyramid` (E1-E2 depuis GLO-90 ; téléchargement GLO-30, correction de surface, E3-E4 sur le cœur), manifeste, tests | ZG0 |
| ZG2 | Moteur : quadtree streamé, patchs déplacés au GPU, pages, `surface_height_at`, remplacement de `FineTerrainJob`, banc de performance | ZG0 |
| ZG3 | Données palier 3 : `detail_zones.json`, récupérateurs IGN/EA/AHN/Flandre/Wallonie, effacement des anachronismes (OSM), E5-E7 | ZG0 |
| ZG4 | Caméra rapprochée, zoom minimal par étage, exagération verticale dynamique, paliers de zoom étendus | ZG2 |
| ZG5 | Hydrographie, côtes, routes et colonies recollées, parcellaire de près | ZG1, ZG2 |
| ZG6 | Villes à l'échelle réelle vers 1340, faubourgs, finage | ZG4 |
| ZG7 | Performance, recette visuelle aux trois paliers, export de la pyramide dans la version publiée, documentation (`docs/geo.md`, `docs/godot-map.md`), crédits | tous |

Vagues : ZG1 + ZG2 + ZG3 ; puis ZG4 + ZG5 ; puis ZG6 ; puis ZG7.

## Conséquences

- Le dépôt ne grossit que du manifeste et du code ; le cache (≤ 4 Go) et les sources brutes
  (≤ 20 Go) vivent hors git et se régénèrent. La version publiée embarque la pyramide
  (plusieurs Go) : taille de téléchargement nettement plus grande, à arbitrer dans ZG7
  (paliers 2-3 éventuellement en contenu optionnel).
- Mémoire vidéo : + ≤ 256 Mo de pages de relief.
- Les paysages proches reflètent le relief réel débarrassé des principales traces modernes, pas
  un relevé de 1340 : haies, chemins creux, forêts et villages restent des reconstructions.
- `FineTerrainJob` et les tuiles E0 restent le repli sans cache.

## Mise en œuvre des paliers 1-2 (lot ZG1)

- **Emprise du cœur réduite** pour tenir le cache E1-E4 sous 2,5 Go : lon −6 → 9, lat 42 → 56
  moins l'Espagne, l'Italie, la Suisse, l'Allemagne de la rive droite du Rhin, l'Écosse et la
  frange irlandaise (`pyramid.CORE_EXCLUDE`) ; restent la France, l'Angleterre et le pays de
  Galles, le Bénélux et la rive gauche du Rhin. Enveloppe du manifeste : [−6 ; 42,35 ; 8 ; 56].
- **Bâti** : une interpolation depuis les bords du masque effacerait le relief des grandes villes
  (Paris couvre 40 km, la vallée de la Seine disparaîtrait) et dépendrait du découpage en blocs.
  Le bâti est remplacé par une estimation morphologique locale du sol (ouverture de 340 m sur la
  surface brute, fermeture de 180 m, lissage), sans couture entre blocs.
- **Canopée** : décalage de 10 m × fraction arborée WorldCover, mesuré sur les lisières en
  terrain plat (forêt d'Orléans 11,4 m, Sologne 10,0, Weald 12,1, Ardenne 11,7 ; Landes 0,8 m,
  coupes rases postérieures aux acquisitions radar).
- **Continuité** : la base floue du rehaussement est celle d'E0 (reconstruit à 0,04 m près),
  échantillonnée à chaque étage ; E1 = moyenne 2 × 2 d'E2, E3 = moyenne 2 × 2 d'E4. E0 porte des
  coutures le long des méridiens et parallèles entiers (lecture tuile par tuile de GLO-90,
  jusqu'à ~300 m dans les Alpes) que les étages fins n'ont pas.

## Addendum (lot ZG2, 2026-09-25) : moteur tel que réalisé

Détail dans `docs/godot-map.md` (« Relief streamé : pyramide et quadtree »). Écarts et précisions
par rapport à la décision :

- **Le quadtree dessine tout le terrain** quand le cache existe (les morceaux E0 restent construits
  mais masqués : repli, bornes, vue parchemin), au lieu de ne couvrir que le proche. `chunk_level`
  et `chunk_surface_changed` gardent leur sens pour les couches (routes, colonies, maquettes).
- **Sélection CDLOD** (distance ↔ espacement des sommets projeté ≤ 4 px, soit ≤ 0,5 px par pixel de
  page) avec **morphing géomorphe** au vertex shader au lieu d'une hystérésis : pas de saut de
  géométrie au changement de niveau. Jupes en plus contre les écarts transitoires.
- **Pages en `FORMAT_R16` avec mipmaps** (entier 16 bits normalisé, même codage que les PNG) :
  ni flottants 32 bits (deux fois plus de VRAM) ni reconstruction ; 256 couches 512² = 171 Mo.
  Table de pages en `instance uniform` par nœud (page fine, page du parent, 8 voisines).
- **Décodage en Rust dans des fils natifs** (`ReliefDecoder`, `request`/`poll`) : godot-rust est
  mono-fil, `GameDataStore` ne peut pas être appelé depuis `WorkerThreadPool` (panique). Replis :
  décodage Rust sur le fil principal (≤ 2 tuiles, ≤ 4 ms par image), puis `Png16` GDScript
  (0,1-1 s par tuile : repli seulement).
- **Recalages des couches** : les pages qui arrivent ne signalent un morceau proche que lorsque son
  étage le plus fin chargé change, **après 700 ms sans nouvelle page** (la chaîne E1 → E4 donne un
  seul recalage), au plus 2 morceaux toutes les 250 ms. Sans ce délai, les recalages des maquettes
  (50-300 ms chacun) doublaient le coût du panoramique.
- **Mesures** (banc `--bench-map`, 1 440 × 900, Apple Silicon, machine partagée à une charge de
  200+ : chiffres relatifs seulement, pyramide réelle E1-E7 de 11 943 tuiles) : deux essais appariés
  quadtree / repli E0 : 31,3 / 31,2 i/s puis 31,5 / 34,1 i/s ; médiane 24,7-26,5 ms contre
  24,3-26,3 ms ; images > 50 ms 54 contre 57-63 ; `update_view` 4,7 ms en moyenne ; ≈ 1 100 pages
  décodées en 28 s (≈ 6 ms par tuile, fils natifs) ; 9,5 M primitives contre 11 M. La cible
  « 60 i/s au palier 2 » reste à vérifier sur machine au repos ; à parité avec le repli E0, le
  quadtree n'est pas le goulot (les recalages synchrones des maquettes, ≈ 2,5 s cumulées, le sont).

## Addendum (orchestrateur, 2026-09-25) : villes emblématiques

Les villes emblématiques (maquettes L1/L2 sous loupe radiale ×3,5, ADR 0015) gardent la loupe en
vue stratégique mais passent en **1:1 géoréférencé au zoom rapproché** : sur un relief de 3 m, la
Seine d'une maquette agrandie ne tomberait plus dans la vraie vallée. Ce passage est le lot VH4
du chantier « villes historiques » (ADR 0037, `docs/wip/vh-villes-historiques.md`), qui démarre
après la fusion de ZG2 et ZG4. ZG6 ne traite que les villes ordinaires.

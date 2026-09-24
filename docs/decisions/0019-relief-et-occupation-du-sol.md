# ADR 0019 — Relief fin et occupation du sol historique de la carte de campagne

- Statut : accepté (lot R1)
- Date : 2026-09-25

## Contexte

Le joueur trouve la carte « assez réaliste mais assez plate » et sans forêts ni lacs lisibles.
Le relief venait d'ETOPO 2022 15″ (≈ 460 m) : la heightmap 4096² (718 m/px) et les tuiles fines
8192² (360 m/px, rééchantillonnées bilinéairement, donc sans détail réel sous ~460 m). Les forêts
de `splat.png` étaient un mélange fixe par terrain de province plus du bruit fractal (une
« bouillie » sans les grands massifs connus) ; seuls les lacs Natural Earth assez grands pour un
pixel existaient.

## Décision

### Relief

- **Source** : Copernicus DEM GLO-90 (3″, ≈ 90 m, altitudes EGM2008 comme ETOPO) sur les terres
  de l'emprise jouable (lon −11 → 12, lat 41 → 60 : France, îles Britanniques, Bénélux, nord de
  l'Espagne, ouest de l'Allemagne, Suisse, Italie du Nord ; 312 tuiles, ≈ 1 Go en cache non
  versionné). Bucket public AWS Open Data `copernicus-dem-90m`, HTTPS anonyme : aucune dépense.
  ETOPO reste la source en mer (bathymétrie) et hors emprise.
- **Résolution** : on garde la grille virtuelle **8192²** (360 m/px), mais **filtrée
  correctement** (moyenne de zone depuis 90 m au lieu d'une interpolation bilinéaire depuis
  460 m). Le passage à 16384² (180 m/px) a été mesuré et écarté : 1,36 Mo par tuile de terre au
  lieu de 0,36 Mo (≈ + 140 Mo de tuiles pour l'Europe de l'Ouest, au-delà du plafond de ~150 Mo
  une fois ajoutées les autres textures), et des maillages de 1025² sommets par tuile que le
  constructeur GDScript (`FineTerrainJob`) ne bâtit pas en temps utile. Le détail sous le pixel
  4096 passe plutôt par l'ombrage (ci-dessous), visible à tous les zooms.
- **Surface de rendu unique** : `heightmap_render.png` (4096², moyenne 2 × 2 des tuiles fines)
  est lue par `MapData` à la place de `heightmap.png` quand `map.json.render_heightmap` existe.
  Terrain, armées, villes, fleuves, sélection et relief fin reposent donc sur la même surface.
  Elle porte un **rehaussement de rendu** du relief local (masque flou `h + 0,8 · clamp(h − flou
  σ 5 km, ±120 m)`, effacé au-dessus de 600-1 600 m d'altitude moyenne) : les plaines vallonnées
  (cuestas d'Île-de-France, vallées de la Seine et de la Loire, bocage normand) se lisent sans
  rendre les Alpes absurdes. Le trait de côte suit exactement `heightmap.png` (terre > 0,5 m,
  mer ≤ 0).
- **Le cœur n'est pas touché** : `heightmap.png` (source de `navgrid.png`, lot M1) et
  `navgrid.png` sont inchangés ; `core/crates/data-model` ne lit que des métadonnées de
  `map.json` (les nouvelles clés sont ignorées par serde). Aucune règle de jeu ne voit le relief
  de rendu.
- **Forêts des règles figées** : `tools/cent_ans_tools/geo/navgrid.py` lisait le canal forêt de
  `splat.png` pour le coût de déplacement. La splat V2 est conservée telle quelle sous
  `data/map/navgrid_splat.png` et lue en priorité : `navgrid.png` reste identique (test
  `test_committed_navgrid_is_up_to_date`). Aligner les coûts sur les forêts historiques
  (régénérer la grille depuis la nouvelle `splat.png`) est une décision de règle laissée à
  l'orchestrateur.
- **Lecture du relief** : `relief_shade.png` (LA8, 8192²) cuit hors ligne depuis le relief fin :
  L = détail d'altitude par rapport à la heightmap de rendu filtrée bilinéairement (normales
  d'ombrage sous le pixel 4096), A = courbure multi-échelle (0,5 / 1,5 / 4,5 km) : creux et fonds
  de vallée sombres, crêtes claires. Le shader l'échantillonne avec mipmaps.

### Occupation du sol vers 1340

- **Forêts** : KK10 (Kaplan et al. 2011 ; données PANGAEA, CC BY 3.0), part de chaque cellule
  de 5′ sous usage anthropique, moyenne 1330-1349 (avant la Peste noire), lue par requêtes HTTP
  partielles dans le fichier de 18,5 Go (seuls ~16 blocs HDF5 utiles). Forêt visée = potentiel
  forestier (limite des arbres décroissant avec la latitude, landes de province, hautes terres
  océaniques, dunes, zones humides, garrigue) × (1 − défriché), relevée dans les **grands massifs
  nommés** de `data/map/historical_forests.json` (55 entrées sourcées : Orléans, Bière, Cuise,
  Retz, Yveline, Brocéliande, Ardenne, Argonne, Othe, Chaux, Morvan, Vosges, Sherwood, New
  Forest, Dean, Weald, Arden, Waltham, Savernake, Inglewood, Ettrick...) et abaissée dans ses
  **landes** (Landes de Gascogne, monts d'Arrée, Pennines, Dartmoor, Campine...). Allocation
  **binaire** : forêt là où un score (bruit fractal à marge uniforme + préférence de terrain :
  pentes, crêtes, sols pauvres d'altitude ; moins autour des villes, villages et hameaux et dans
  les plaines inondables) est dans la part visée. Massifs à lisières nettes, clairières et
  essarts ; lisières ondulées et plus nettes de près dans le shader.
- **Contrat `splat.png` inchangé** (RGBA : R prairie, G cultures, B forêt, A roche/lande, somme
  255 sur terre), taille portée à **4096²** (lire la taille dans le fichier). Nouveau
  `forest_kind.png` (L8 2048², part de résineux : étage montagnard, pinèdes calédoniennes,
  pins méditerranéens, massifs nommés) destiné au rendu des forêts (lot V4).
- **Zones humides** : `data/map/wetlands.json` (32 entrées sourcées : Dombes, Brenne, Sologne,
  Woëvre, étangs lorrains, Bresse, Fens, Somerset Levels, Romney Marsh, Hatfield Chase, marais
  poitevin, Brière, Camargue, marais de la Somme, Moëres, polders flamands et du Zwin...) →
  `wetlands.png` (RGB 4096² : marais, étangs, prés humides, plus prés humides des fonds de
  vallée). Rendu procédural : mares en taches dans les roselières, étangs médiévaux à chaussée
  droite, couverture moyenne au dézoom. Les lacs Natural Earth plus grands qu'un pixel restent de
  l'eau réelle (masque de terre).
- **Rendu** : `game/shaders/relief_landcover.gdshaderinc` et `game/scripts/map/relief_landcover.gd`,
  quatre crochets d'une ligne dans `terrain.gdshader` (inclusion, gradient, poids, surface) et un
  dans `terrain_builder.gd`, pour faciliter la fusion avec les lots V4 et CV1.

## Conséquences

- Dépôt : + ≈ 66 Mo (relief_shade 43 Mo, heightmap_render 15 Mo, splat 8,4 Mo au lieu de 3,8,
  tuiles fines 53 Mo au lieu de 50,5, wetlands et forest_kind < 1 Mo).
- Chargement : `relief_shade.png` (≈ 0,9 s avec mipmaps) décodé en tâche de fond, splat 4096² + ≈ 150 ms ; mémoire vidéo
  + ≈ 190 Mo. Performance mesurée dans `docs/wip/r1-relief-campagne.md`.
- Licences : Copernicus DEM (attribution obligatoire), KK10 CC BY 3.0, Natural Earth domaine
  public ; crédits dans `CREDITS.md` et l'écran des crédits.
- Régénération : `docs/geo.md` (`geo relief-shade`, `geo kk10`, `geo landcover`). `geo relief`
  (ETOPO seul) reste un repli qui écrase les tuiles Copernicus.
- Limites : KK10 répartit l'usage du sol à l'échelle de la cellule (tendance régionale, lissée) ;
  les emprises des massifs et zones humides nommés sont des ellipses approchées aux bords
  bruités, pas des tracés d'archives.

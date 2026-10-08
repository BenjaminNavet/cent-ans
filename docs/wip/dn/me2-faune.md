# DN ME2 : troupeaux et faune (branche dn/me2-faune)

Etat (08/10) : couche, shader, donnees, schema et tests faits ; a fusionner par l'orchestrateur.

## Ce qui existe
- `data/map/map_fauna.json` (schema `data/schemas/map_fauna.schema.json`, test `tools/tests/test_map_fauna_schema.py`) : 32 especes = ids `animal_*` de `dn_catalog_map_extra.json`, 52 zones (ellipses lon/lat, EPSG:3035 par `FaunaLayer.laea_3035`), densite en troupeaux / 1000 km2, filtres de biome (splat : prairie/cultures/foret), pente, altitude, marais (`marsh_max`, wetlands.png), lacs, fleuves, colonies, proximite mer (phoques, morses) ou fleuve (castors), poids par saison (transhumance : profils alpage ete / plaine hiver, glandee en automne). Genere par un script jetable, edition a la main possible.
- `game/scripts/map/fauna_layer.gd` : semis par cellule de 96 px a la demande (deterministe), un MultiMesh par espece et par cellule (<= 25 appels a d = 8, 20 a d = 60 en Camargue), amincissement par rang (plafond 3000), niveau de detail par distance, eviction des cellules. Branche dans `CampaignLife._setup_fauna` (`--no-fauna`, `--life-off=fauna`), graine d'errance par tour (`set_turn`).
- `game/shaders/fauna.gdshader` : errance (Lissajous par troupeau, arrets pour brouter par temps deforme), pas (pattes en diagonale), rebond, tete basse a l'arret, cap suivant la marche, altitude par `campaign_display_height`, saison par `campaign_season` x profil. Pas d'armature.
- Modele : `game/assets/models/dn/fauna/<id>_lod{0,1,2}.glb` (ingestion `animal`) prime ; sinon `placeholder` (folk/horse|cow|sheep.glb, tous les placeholders marchent vers +X) ; sinon boites. Brancher un glb = le deposer (ou une ligne `species`).
- Tests : `game/tests/me2_fauna_test.gd` (donnees, projection, semis sur toute la carte ~0,7 s, regions, rendu, parchemin), `me2_fauna_shot.gd` (capture + sonde numerique de contraste).

## Ecarts / points ouverts
- Taille tenue a l'ecran : longueur = 0,02 x d^0,85 (unites monde), jamais sous la taille reelle ; a regler a l'oeil (`render.length_k`). Le plancher de camera du jeu (20) empeche de voir la taille reelle sauf captures (`floor_distance = 0`).
- Orientation des glb DN : on suppose l'axe long = avant, tete vers +axe ; si un modele ingere marche a reculons, corriger son `yaw_fix` a l'ingestion.
- Altitude : heightmap 719 m/px (pas le relief fin) ; les betes peuvent flotter ou s'enfoncer de quelques metres au tout pres sur le relief fin. Une relecture par `quadtree.surface_snapshot` comme `GroundClutter` regle cela si besoin.
- Pas de mesure de temps de frame fiable (machine a charge 50-100 pendant la nuit) ; estimation : <= 25 appels, <= 3000 instances (LOD <= 1200 tri), vertex shader seul.
- Zone Camargue modeste (une dizaine de troupeaux : masque de terre a 719 m/px et marge d'eau d'un pixel) ; densites des grandes zones (chevres, dromadaires) a rabaisser apres revue visuelle.
- Erreurs RID intermittentes "Initializing already initialized RID / m is null" vues 2 fois sur 7 en headless sous forte charge (non reproduites sur 3 executions consecutives ni sur hb5) : a surveiller.
- Les moutons/vaches des scenes FK (`folk/`) restent separes (scenes seulement).

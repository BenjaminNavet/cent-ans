# DN — assets manquants de la carte de campagne (recensement et plan)

Demande du joueur (08/10) : marais, eau de l'océan, chevaux de Camargue, torrents, nuages, « etc. ».
Catalogue glb : `data/art/dn_catalog_map_extra.json` (145 entrées, format `dn_batch`, 3 graines,
`backend3d: both`). Nouvelle classe d'ingestion `animal` ajoutée à `data/art/dn_ingest_classes.json`
(axe longueur, LOD 4000/1200/300, texture 512). Aucune génération lancée.

## 1. Déjà présent (c) — ne pas refaire
- Saisons : `SeasonVisuals` (poids printemps/été/automne/hiver dans le shader terrain, feuillage, neige des maquettes), `season_banner`.
- Nuages/brume/pluie/neige/orage : `campaign_weather_view`, `precipitation_profile`, bloc `clouds` de `data/ui/campaign_map.json`, ombres de nuages (`campaign_cloud_shadow.gdshaderinc`, RV-C), vue moyenne TB2.
- Ciel/atmosphère : brume (BR/SS), occlusion de vallée, soleil (RV-A/B), panaches régionaux (RV-F), fumées/flammes (`life_effects`).
- Vie : vols d'oiseaux (`life_ambient`, 3 vols x 9 corbeaux-type, un seul modèle), bateaux de fleuve et navires de mer, moulins, gens, caravanes, scènes (FK), troupeaux **dans les scènes seulement** (`folk/` : cow, ox, sheep, horse, procéduraux).
- Eau : textures de détail mer/océan/fleuve (RC5, `water_detail`), lacs/étangs (HC2), rivières fines, ponts/gués (`river_crossings`), baies/côtes (`coast_look`).
- Zones humides : masque `wetlands.png` (77 zones, HC5) cuit dans le sol ; roselières/saules via HB (`hb_ground`, espèces).
- Bâti hors murs (TB3) : ferme, moulin, vignoble, mine, saline, abbaye, marché, port, chantier (`outbuildings/`) ; ruines = icône seule (`ruin_markers`) ; monstres marins = parchemin seulement.
- Végétation/rochers : catalogues `dn_catalog_nature*.json` (49 entrées), HB/HC/GA3.

## 2. (a) glb générés : `dn_catalog_map_extra.json` (145)
- Faune d'élevage et sauvage, **51 animaux** (`animal_*`, `bird_*`) : chevaux de Camargue, taureaux, vaches (2 robes), bœuf de trait, moutons (laine, mérinos), chèvre, porc de glandée, cheval de trait, poney des steppes, âne, mulet, dromadaire, chameau de Bactriane, cerf/biche/chevreuil, sanglier, loup, ours, bouquetin, chamois, renne, élan, aurochs, bison, renard, lièvre, castor, phoque, morse ; oiseaux (oie posée/en vol, grue, cigogne + nid de toit, flamant, mouettes, cormoran, héron, cygne, canard, étourneau, corbeau, vautour, aigle, pélican) ; mer : dauphin bondissant, dos de baleine, saumon bondissant.
- Humain hors villes (`env_*`, ~90) : croix de chemin, calvaire, gibets (bois et pierre), péage, borne, oratoire, hospice de col, mine, carrière, charbonnière, verrerie, forge hydraulique, moulin à marée, bordigue, pêcherie à pieux, séchoirs à poisson, phare, tour à signaux, balise, bac, barge de fleuve (+ halage), barque de pêche, barque nordique, voile latine, boutre, chantier naval, épave, ermitage, monastère de falaise, hôtellerie de pèlerins, chapelle, dolmen, alignement, cromlech, aqueduc, arènes, temple et arc romains, village abandonné, hameau brûlé, fosse de peste, lazaret, champ de bataille (2), pèlerins, cabane de berger, bergerie, cabane d'alpage, yourte, chariot de steppe, charrette, mulet bâté, caravane de chameaux, foire, moulins (tour méditerranéen, polder), noria, colombier, grange dîmière, pressoirs, houblon, verger fleuri, haie de bocage, tourbière, roselière, hutte de marais, massette, bois flotté.
- Relief et eau (rock) : seuil de cascade, lit de torrent, gué, langue de glacier, banquise, falaise de craie, arche marine, récif, grotte, karst, cône volcanique, source chaude, oasis, agave.
- Saisons : sapin enneigé, chêne doré, hêtre cuivré, mélèze doré, prairie fleurie, gerbes.
- Mise à l'échelle : valeurs réalistes en mètres (`target_width_m` = longueur pour les animaux).

## 3. (b) textures 2D, particules, shaders (pas de glb)
| Élément | Technique |
|---|---|
| Houle et écume d'océan | normal map animée en 2 couches + masque d'écume sur `coast_dist.png` (étendre RC5), crêtes de vagues au large par bruit (shader mer) |
| Ressac sur falaises/plages, écume de rivage | bande animée le long de `coastline` (ruban `coast_renderer`) UV défilant, alpha par type de côte (`coast_types.json`) |
| Marées/estrans | deuxième courbe de rivage sombre (vase) animée lentement dans le shader mer ; zones : Mont-Saint-Michel, Wadden, Wash |
| Marais, tourbières, étangs, rizières | décalcomanie de sol : eau sombre + reflets (SSR bon marché), touffes de roseaux en MultiMesh ; rizières = damier d'eau (masque culture) |
| Salines | motif de bassins rectangulaires en décalque sur la côte (Guérande, Camargue, Aigues-Mortes) |
| Torrents/cascades | ruban de rivière à pente forte : écume (flow map) + particules de brume en pied ; seuil par pente du lit (`river_bed.png`) |
| Gués | décalque de pierres plates sur la traversée (`crossings.json`) |
| Glaciers, banquise, neige permanente | masque d'altitude > limite des neiges, texture blanc-bleu + crevasses ; banquise : plaques de glace dérivantes (sprites alpha) au nord, saison hiver |
| Aurores boréales | quad de ciel additif animé (bruit), nuit/hiver, latitude > 60 |
| Nuages | déjà là ; ajouter cartes de nuages hautes (cirrus) et strates basses sur les massifs |
| Brouillard de vallée/matinal | déjà RV (occlusion) ; ajouter nappes sur fleuves et marais le matin |
| Pluie/neige/orage lointains | rideaux de pluie sur la carte (bandes verticales alpha) liés à `precipitation_profile`, éclairs (flash de lumière + sprite) |
| Fumée de charbonnières, forges, verreries | réutiliser `life_smoke` aux positions des `env_*` (données) |
| Cultures et paysages agricoles | textures de parcellaire : vignes en terrasses (lignes + murets), oliveraies (points réguliers), vergers, openfields (bandes), bocage (haies = ruban de buissons), brûlis (cendre noire), lavande/lin/pastel/safran (teintes par saison) via `ground_biome_mix.json` + splat |
| Dunes, éboulis, lave | textures de sol + normal ; dunes : ondulations par bruit à pente |
| Voies romaines, routes dallées | texture de route alternative (pavés) sur arêtes historiques |
| Ombres/reflets des troupeaux, nuées d'oiseaux | blob d'ombre au sol + flipbook d'étourneaux (murmuration) en particules |
| Pestiférés | cartouche de fumée jaune + sprite de croix ; réutiliser les scènes FK |
| Champs de bataille passés | décalque d'herbe foulée, tertres (MultiMesh) depuis l'historique (`war_scars`) |
| Poissons sautants/bancs, sillages | sprites alpha animés sur la mer ; sillage de navire (ruban) |
| Pèlerins/marchands/foires | données `map_scenes.json` (FK) + figurines folk ; foires = mode « saisonnier » |

## 4. Plan d'intégration en lots (par impact visuel décroissant)
Règle commune : un `MultiMesh` par famille et par LOD, rendu seul (aucune règle), densité par biome/région dans `data/map/map_fauna.json` (à créer, schéma `data/schemas/`), `--no-map-extra` pour l'A/B. Charge : tout ce qui est sous le palier près (d < ~45) ; troupeaux animés par shader (déplacement vertex), pas par nœud.

**ME1 — Mer vivante (impact maximal, ~0 $)** : houle 2 couches, écume de rivage et ressac, estrans, fonds clairs près des côtes, sillages, poissons sautants, dauphins/baleines (glb `sea_*` + sprites). Fichiers : shader mer, `coast_renderer.gd`, `sea.gd`, `water_detail.json`. Mécanisme : textures + ruban animé ; glb en MultiMesh (≤ 200). Coût : +0,2 à 0,6 ms GPU, aucun appel de rendu notable.

**ME2 — Troupeaux et faune** : nouveau `fauna_layer.gd` (modèle sur `ground_clutter.gd`) : essaims de 5-30 têtes par cellule de 2 unités, vague d'errance (déplacement lent par paramètre shader : cercle + marche aléatoire déterministe par graine/tour, pas de physique). Semis par biome : Camargue (chevaux blancs, taureaux), prairies atlantiques (vaches), causses/Pyrénées/Highlands (moutons, chèvres), forêts (cerfs, sangliers, porcs de glandée l'automne), steppe (poneys, chameaux de Bactriane), Maghreb/Andalousie (dromadaires), Alpes (bouquetins, chamois), nord (rennes, élans, loups, ours), Pologne (aurochs, bison), côtes (phoques, morses). Saison : transhumance (moutons montent en été) via `campaign_season`. Fondu de distance, ≤ 3 000 instances visibles, 1 appel par espèce. Coût : ~25 appels de rendu, 0,3-0,8 ms.

**ME3 — Oiseaux élargis** : étendre `life_ambient.gd` à plusieurs espèces (oies en V à l'automne, étourneaux, mouettes sur les côtes, flamants en Camargue, cigognes sur les toits des villes alsaciennes/polonaises, grues sur les marais, rapaces en montagne, corbeaux sur les champs de bataille). Shader `life_birds` déjà animé : ajouter une table espèce/biome/saison. Coût : < 0,2 ms.

**ME4 — Eaux douces et zones humides** : torrents (écume/flow map sur pentes), cascades (glb seuil + brume), gués, marais (reflets, roseaux MultiMesh, hutte de marais, tourbières, barques), étangs, salines (décalques), glaciers/banquise. Données : `wetlands.json`, `river_bed.png`, `historical_lakes.json`. Coût : 0,2-0,5 ms.

**ME5 — Atmosphère** : cirrus et strates, rideaux de pluie lointains, éclairs, nappes de brume matinale, aurores (hiver, lat > 60), bancs de brouillard marin. Données : `precipitation_profile`, `data/ui/campaign_map.json`. Coût : 0,1-0,3 ms.

**ME6 — Humain hors villes (semi-statique)** : `roadside_layer.gd` : croix, calvaires, gibets, péages, bornes, oratoires, hospices de col, phares, tours à signaux, balises, bacs, barges, bateaux de pêche, bordigues, chantiers navals, épaves ; sites fixes de `data/map/roadside_sites.json` (positions historiques : Montfaucon, Cordouan, Carnac, Pont du Gard…) + semis le long des routes par densité. Portées de visibilité par type (comme `town_maquette_data`), taille tenue à l'écran comme TB3 (`outbuilding_layer.gd`). Coût : MultiMesh ≤ 40 appels, 0,3 ms.

**ME7 — Industrie et ruines** : mines, carrières, charbonnières, verreries, forges, moulins à marée/polder/noria, séchoirs ; ruines romaines et villages abandonnés, hameaux brûlés, fosses de peste, champs de bataille passés (depuis `war_scars`/`ruined_places`), mégalithes. Fumées via `life_smoke`. Liés aux bâtiments du moteur quand il existe (`building_models.json`).

**ME8 — Paysages agricoles et saisons** : parcellaire par texture (vignes en terrasses, oliveraies, rizières, vergers, bocage, openfields, brûlis, lavande/pastel/safran), arbres colorés d'automne, sapins enneigés, prairie fleurie, gerbes à la moisson. Mécanisme : `ground_biome_mix.json` + variantes `foliage` saisonnières (le shader a déjà les 4 poids). Coût : texture seulement, 0,1 ms.

**ME9 — Pèlerins, marchands, foires, steppe** : yourtes et chariots, caravanes de chameaux (Orient/Maghreb), pèlerins sur Compostelle (cheminement sur routes), foires (Champagne) en saison : scènes FK étendues (`map_scenes.json`), figurines folk existantes + glb ci-dessus.

Ordre de production glb (file GPU sérielle, 0,02 $ x 3 graines/entrée si fal, sinon local) : animaux d'abord (ME2/ME3), puis `env_*` marqueurs de route (ME6), puis relief/eau (ME4), industrie (ME7), saisons (ME8). Budget fal estimé : 145 x 3 x 0,02 = 8,7 $ ; en local (mflux/SF3D) : 0 $, ~3 h GPU. À arbitrer avec l'enveloppe DN (≤ 10 $, déjà entamée par les autres catalogues) : produire d'abord ME2/ME3/ME6 (≈ 80 entrées).

## 5. Points ouverts
- La classe `animal` doit être connue de `dn_ingest.py` (lecture du JSON des classes : normalement automatique).
- Les animaux « en pose de profil ¾ » ne sont pas animés (pas d'armature) : marche simulée par rebond vertex + glissement ; le style TRELLIS ne garantit pas les pattes séparées, à vérifier sur 2-3 espèces avant le lot complet.
- Nuage/mer/atmosphère : déjà présents, le manque ressenti vient surtout de la pauvreté d'animation et de variété ; mesurer avec captures avant d'ajouter des couches.

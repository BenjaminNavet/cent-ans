# 0105 — Textures du sol de bataille en 2k et macro-variation (lot GA2)

Date : 2026-09-28. Statut : accepté. Suite de l'ADR 0086 (convention demi-pixel des rasters) et
de la spec `docs/superpowers/specs/2026-09-28-ga-assets-generes-design.md` (§ GA2).

## Contexte

Le sol de bataille (lot V4) mélangeait 9 couches Poly Haven (CC0) en 1k dans un
`Texture2DArray`. La spec GA2 demande : passage en 2k, 3 à 4 couches ajoutées (prairie fleurie,
herbe piétinée, chaume/éteules, labour), une macro-variation procédurale (50–200 m, teinte et
luminance, faible amplitude) et un `--no-ga2` pour comparer.

Mesure mémoire préalable (avant tout téléchargement, arithmétique BC7/mipmaps supposé par la
spec, 1 octet/texel, facteur mipmaps 4/3) :
- 13 couches, albédo **et** normale en 2k (2048²) : 13 × 2048² × 4/3 ≈ 69,3 Mo chacun,
  soit **≈ 138,6 Mo** — au-dessus du plafond de 120 Mo fixé par la spec.
- 13 couches, albédo 2k / normale **1k** (1024²) : 69,3 + 17,3 ≈ **86,6 Mo** — sous le plafond.

Mesure réelle en jeu (`game/tests/ga2_ground_test.gd`, `TextureLayered.get_format()`) : le format
VRAM effectif de ces couches (sans alpha) est **DXT1/BC1** (format Godot 17), pas BC7/BPTC comme
la spec le supposait — réglage `compress/channel_pack=0` des fichiers `.import`, déjà en place
avant GA2 (`ground_albedo_array.jpg.import` du lot V4, inchangé ici). DXT1 tient 0,5 octet/texel
(la moitié de BC7) : mémoire mesurée **≈ 43,3 Mo** (albédo 2k ≈ 34,7 Mo, normale 1k ≈ 8,7 Mo),
très en dessous des 86,6 Mo anticipés. Décision inchangée (albédo 2k / normale 1k) : la marge
sous le plafond permettrait même normale 2k (≈ 138,6 Mo réel ÷ 2 ≈ 69,3 Mo hypothétique en DXT1),
mais la spec fixe explicitement le repli 1k dès 120 Mo dépassés en BC7 ; ne pas rouvrir ce choix
ici (voir « point ouvert » dans le rapport du lot).

## Décision

1. **Albédo 2k, normale 1k** pour les 13 couches du sol (mesure ci-dessus, valeur mesurée en jeu
   dans `game/tests/ga2_ground_test.gd`). La spec anticipait déjà ce repli.
2. **Identité des couches dans les données**, pas dans le shader ni le script Python :
   `data/fx/battle_ground_layers.json` (schéma `data/schemas/fx_battle_ground_layers.schema.json`)
   liste, dans l'ordre d'empilement du `Texture2DArray`, l'identifiant Poly Haven, la taille de
   répétition (m) et un `role` optionnel pour les couches que le shader traite spécifiquement.
   `build_textures.py` et `battle_terrain.gd` (`BattleTerrain.ground_layers()`,
   `ground_role_index()`) lisent ce fichier ; le shader (`battle_ground.gdshader`) reçoit le
   nombre de couches (`layer_count`), leur taille de répétition (`layer_tile_size[]`) et l'index
   des rôles ajoutés (`idx_flowering_meadow`, `idx_trodden_grass`, `idx_stubble`,
   `idx_fresh_plough`, -1 si absents) en uniformes : aucune identité de couche codée en dur dans
   le shader, hormis les 9 rôles historiques (`L_GRASS`…`L_SNOW`) dont l'occupation (splatmap,
   pente, relief) reste un calcul de rendu spécifique à chacun, pas une donnée.
3. **4 couches ajoutées, occupation câblée sur des règles existantes** (pas de nouveau coût de
   calcul par pixel notable, pas de changement de comportement hors sol) :
   - **labour frais** (`fresh_plough`, `farm_furrows`) : parcelles « labour » posées par le
     décor (EP6, `decor_kind == 1`), distinctes du labour ambiant/procédural loin des parcelles
     (qui garde `farm_soil`, layer historique 7) ;
   - **chaume/éteules** (`stubble`, `withered_grass`) : parcelles « chaume » du décor (EP6,
     `decor_kind == 6`), jusqu'ici sans texture dédiée (teinte seulement) ;
   - **herbe piétinée** (`trodden_grass`, `grassy_cobblestone`) : halo élargi déjà cuit autour des
     chemins et routes (`splat_b.g`), sans nouvelle carte ni nouveau coût de simulation ;
   - **prairie fleurie** (`flowering_meadow`, `leafy_grass`) : part des taches de prairie grasse
     existantes (bruit `patch`/`patch2`, lot V4b), sans nouvelle carte.
   Poly Haven n'a pas de texture CC0 dédiée « prairie fleurie » (wildflower meadow) à la date du
   lot ; `leafy_grass` est le substitut le plus proche du catalogue. Point ouvert, cf. rapport.
4. **Macro-variation dédiée 50–200 m** (teinte et luminance, faible amplitude, 4 octaves de bruit
   déjà utilisé ailleurs dans le shader) appliquée à l'albédo final, indépendante des taches de
   prairie (plus grande échelle) et de la macro-variation existante (`macro`/`macro2`, hors bande
   50–200 m). Sous `ga2_on` (uniforme), coupé par `--no-ga2` : les couches et leur câblage restent
   en place (pas de double jeu de textures 1k/2k à maintenir), seule la macro-variation dédiée
   est désactivée pour le A/B.

## Conséquences

- Mémoire du sol : **≈ 43,3 Mo mesurés** (`game/tests/ga2_ground_test.gd`), largement sous le
  plafond de 120 Mo — la spec anticipait 86,6 Mo en supposant BC7 ; le format réel (DXT1,
  préexistant) est deux fois plus compact.
- `data/fx/battle_ground_layers.json` devient la source de vérité pour toute couche de sol future
  (GA4 réutilisera la macro-variation ; GA5 suit le même schéma pour les bâtiments si besoin).
- La substitution « prairie fleurie » → `leafy_grass` est un choix éditorial documenté ici et
  dans `README.md` ; à revoir si Poly Haven publie une texture plus proche.
- `--no-ga2` ne restaure pas les anciennes textures 1k (compromis assumé : un seul jeu de
  textures à maintenir) ; il coupe seulement la macro-variation dédiée pour la comparaison A/B.
- **A/B non concluant à l'écriture de cet ADR** : mesuré sur une machine partagée très chargée
  (autres sessions compilant en parallèle, `uptime` load average ≈ 120), `frame_ms_median` a varié
  de 25 à 44 ms sans direction stable, y compris `--no-ga2` parfois plus lent que le défaut — ce
  qui ne peut pas venir du coût réel de la macro-variation (4 échantillons de bruit en plus par
  pixel de sol, seule différence entre les deux). À refaire sur machine calme avant de considérer
  le budget de performance (+5 %) validé.

## Section GA5 (bâtiments)

Date : 2026-09-28. Suite du lot GA5 (`docs/wip/ga.md`), même principe qu'en §GA2 (2k Poly Haven,
« GA5 suit le même schéma pour les bâtiments » anticipé plus haut) mais avec des différences
propres aux bâtiments, détaillées ici.

1. **Import non compressé découvert et corrigé.** Contrairement au sol (déjà en `compress/mode=2`,
   VRAM compressé, avant même GA2), les textures individuelles de `game/assets/textures/buildings/`
   (matières `BuildingMaterials.SPECS`) étaient importées en **`compress/mode=0`** (« Lossless »,
   `vram_texture: false`, **sans mipmaps**) depuis le lot BR1 — un oubli de réglage, pas un choix.
   GA5 corrige ce réglage (`compress/mode=2`, `mipmaps/generate=true`, régénéré par
   `godot --headless --path game --import`) en même temps que le bump 2k. Conséquence chiffrée
   (mesure `game/tests/ga5_building_test.gd`, format réel DXT1/BC1 pour les diffuses et la plupart
   des normales sans alpha, comme en §GA2) : les **12 matières texturées** (11 historiques +
   `TimberFrame`, albédo 2k, normale et rugosité 1k quand elles existent) pèsent **≈ 47,3 Mo** au
   total (mesure conservatrice : chaque matière est comptée séparément même quand deux partagent
   le même fichier source, ex. `Planks`/`Door`) — **moins** que l'ancien 1k non compressé et sans
   mipmaps (`1024² × 4 octets/texel` par diffuse ≈ 4,2 Mo **par matière**, sans le facteur 4/3 des
   mipmaps puisqu'il n'y en avait pas) : corriger l'import pèse plus lourd sur la mémoire que le
   bump de résolution, malgré le doublement.
2. **`SPECS`/`PLAIN`/`ROOFS`/`ATLAS_LAYERS` déplacés dans les données.** `game/scripts/visual/
   building_materials.gd` lisait ces tables en dur (lot BR1). GA5 les déplace dans
   `data/art/building_materials.json` (schéma `art_building_materials.schema.json`), lu au premier
   appel de `BuildingMaterials.material()`/`atlas_layers()` — même repli que
   `BattleTerrain.ground_layers()` (dossier de données du jeu, puis `data/` du dépôt). `kit_export.py`
   (Blender) garde sa propre copie Python de ces valeurs (`MATERIALS`/`ATLAS_LAYERS`, déjà commentée
   « mêmes valeurs que `building_materials.gd` » avant GA5) : synchronisation manuelle assumée,
   inchangée par ce lot (script Blender, hors périmètre GA5, pas de réexport du kit ici).
3. **Torchis/colombage (`TimberFrame`) : matière prête, non câblée.** La spec GA5 demande des
   variantes torchis/colombage « si les bâtiments ont une place pour les utiliser ». Poly Haven
   n'a pas de texture CC0 dédiée pour ce motif (mur à pans de bois, torchis = remplissage,
   colombage = ossature) ; `TimberFrame` est donc un **composite procédural** (`build_textures.py::
   timber_frame()`/`_beam_mask()`, seed déterministe) : lattis de poteaux/sablières/entretoises
   mélangé entre `lime_plaster` (remplissage) et `rough_wood` assombri (poutres), toutes deux déjà
   CC0 Poly Haven — 0 $, pas d'IA. La matière est complète et chargée (`data/art/
   building_materials.json`, `"wired": false`, absente de `atlas_layers`), utilisable dès
   aujourd'hui via `BuildingMaterials.material("TimberFrame")`, **mais aucune surface du kit
   Blender ne porte ce nom** : l'affecter à des bâtiments réels demande soit un changement +
   réexport de `tools/blender_scripts/building_kit.py`/`kit_export.py` (Blender est disponible sur
   la machine de dev, mais le kit est versionné en `.glb` déjà exportés dans `game/assets/models/
   buildings/`, et l'indice de couche de l'atlas `Building` est **baké dans la couleur de sommet**
   de ces `.glb` à l'export — l'ajouter en milieu de tableau désaligne les modèles existants, et
   l'ajouter en fin de tableau (après `Canvas`, indice 14) tombe après `first_plain` = 11 et serait
   traité comme une matière unie, sans texture, par `building_atlas.gdshader`, sauf à aussi changer
   le shader), soit une réécriture plus large du système d'atlas. C'est aussi un **choix de
   conception non tranché ici** : quelle proportion de bâtiments (quel type, quelle région —
   la charpenterie apparente est historiquement plus caractéristique de Normandie/Île-de-France/
   Angleterre que du Midi) recevrait `TimberFrame` à la place de `Plaster`. Décision laissée au
   joueur/lead (voir rapport du lot et `docs/wip/ga.md`, section GA5) ; ni le rendu en jeu ni les
   `.glb` existants ne sont affectés par cette matière tant qu'elle n'est pas câblée.

## GA4 — Terrain de campagne et mer (addendum, 2026-09-28)

### Contexte

Le terrain de campagne (`terrain.gdshader`) mélangeait 7 couches Poly Haven 1k, assemblées à
l'exécution par `TerrainBuilder` en deux `Texture2DArray` **RGBA8 non compressés** (≈ 74,7 Mo
avec mipmaps). L'identité des couches était codée en dur deux fois (`MATERIAL_LAYERS` en
GDScript, `LAYERS` dans `tools/cent_ans_tools/geo/textures.py`). La mer (`water.gdshader`) tirait
ses vagues d'une houle procédurale (trois évaluations par pixel) et sa couleur de deux paliers.

### Décision

1. **Albédo 2k, normale + rugosité 1k**, mêmes 7 assets Poly Haven, empilés verticalement
   (`terrain_albedo_array.jpg`, `terrain_normal_array.jpg`) et **importés** en tableau compressé
   en VRAM (`2d_array_texture`, `slices/vertical=7`), comme le sol de bataille. Mesure
   (`game/tests/ga4_terrain_test.gd`, format réel DXT1) : **23,3 Mo** (albédo 18,7 + normale
   4,7) contre 74,7 Mo pour l'ancien chemin — la 2k coûte donc trois fois moins que la 1k non
   compressée. Normale en 1k (choix GA2) : détail normal discret sur la carte
   (`detail_normal_strength` 0,55, effacé dès 1,8 pixel carte par pixel écran) ; poids du dépôt
   ≈ 19 Mo au lieu de 34 Mo.
2. **Identité des couches en données** : `data/fx/campaign_terrain_textures.json` (schéma
   `fx_campaign_terrain_textures.schema.json`) : ordre, asset Poly Haven, moyenne linéaire de
   l'albédo (écrite par `cent-ans geo textures`, le tableau importé ne garde pas ses pixels côté
   CPU ; le test la recontrôle sur la source). L'ordre reste le contrat d'index fixe du shader
   (`CampaignTextures.SHADER_LAYER_ORDER`, vérifié au chargement et par pytest) : pas de
   refonte en index de données de `terrain.gdshader`, partagé avec d'autres chantiers.
3. **Tuilage** : `tile_screen_px` 72 → 96 (données) : le shader garde une tuile à taille écran
   constante, la 2k n'apporte de la netteté que si la tuile occupe plus de pixels ; une tuile
   un tiers plus grande réduit aussi la répétition visible.
4. **Macro-variation factorisée** dans `game/shaders/ga_macro.gdshaderinc`
   (`ga_macro_variation`, `ga_depth_color`), partagée par `battle_ground.gdshader` (GA2,
   résultat identique : `ga_macro_variation(h, 0.5, 1.0, 0.0)`), `terrain.gdshader` et
   `water.gdshader`. Chaque appelant fournit son bruit : texture en bataille, bruit de valeur
   procédural sur la carte. Carte : 4 octaves (9, 24, 60, 150 pixels carte ≈ 6 à 110 km),
   teinte pondérée vers les octaves fines, luminance (± 8 %) vers les larges, mêmes 4
   échantillons ; appliquée à l'albédo final, donc visible aussi au dézoom où les textures sont
   coupées.
5. **Mer** : normale tuilable 1024² **procédurale** (`water_normal.png`, œuvre propre CC0 :
   somme de trains d'ondes à vecteurs d'onde entiers, raccord exact, vent dominant) — aucune
   normale d'eau CC0 chez Poly Haven ni ambientCG (recherche du 28/09). Deux couches défilant
   à vitesses et orientations différentes, mêlées ; estompe au dézoom plus tardive que la houle
   (les mipmaps lissent). Couleur de profondeur à trois paliers (haut-fond, palier à 25 m, grand
   fond, décroissance 90 m), mêmes valeurs posées sur la mer peinte de `terrain.gdshader` pour
   éviter une couture. Tout réglage dans les données.
6. **`--no-ga4`** : ancien chemin complet (JPEG 1k par couche en RGBA8, houle procédurale, deux
   paliers, `tile_screen_px` 72, pas de macro-variation GA4). Contrairement à GA2, les anciens
   fichiers 1k restent, pour un A/B honnête (mémoire et coût d'échantillonnage) ; à supprimer à
   GA6 si le chemin GA4 est retenu.

### Conséquences

- Mémoire du terrain divisée par 3 (74,7 → 23,3 Mo) malgré la 2k ; chargement attendu plus
  court (plus d'assemblage d'images ni de génération de mipmaps à l'exécution).
- Coût shader : terrain + 4 bruits de valeur par pixel ; mer : 2 lectures de texture au lieu de
  2 évaluations de houle en plus (probablement neutre ou gain). **A/B non mesuré dans le lot**
  (machine chargée, load average > 120) : banc PB1 carte à passer par la session principale,
  cache de relief relié, passes alternées avec et sans `--no-ga4`.
- Changer d'asset : éditer `poly_haven_id` dans les données puis `cent-ans geo textures`
  (réécrit tableaux, normale d'eau et moyennes).

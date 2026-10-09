# 0241 — Végétation, bâtiments et eau régionaux

Statut : accepté (10-09, chantier TX, tranches 3 et 4 + eau)

## Contexte
La fabrique de textures (ADR 0236) a produit des images brutes 2k : 70 cartes de plantes au sol
(5 par biome) et 15 cartes d'herbe de bataille, 12 écorces et 12 planches de feuilles, 80 matières
de bâtiments par région, 12 surfaces d'eau. Il reste à les brancher sans toucher aux modèles 3D ni
aux règles : les consommateurs sont des atlas partagés (bâtiments, campagne et bataille), des
arbres procéduraux (bataille), une touffe d'herbe unique (campagne), une mer à une seule texture.

## Décision
- **Fabrique** : un paquet par usage, dans le catalogue (`packs:`), clé `kind` : `tile` (raccord +
  normale, tableau JPEG), `cards` / `sheet` (détourage rembg local, recadrage au pied de la plante,
  couleur « défrangée », tableau RGBA WebP), `files` (une carte WebP par entrée, herbe de bataille),
  `tiles` (albédo + normale JPEG 2k par entrée, eau). `equalize: false` garde la luminance propre à
  chaque matière (chaux blanche, ardoise sombre, bouleau clair). Les `.import` des tableaux de
  normales suivent la règle QW-D (`models-textures-fix`).
- **Bâtiments** : `building_regions.json` gagne `materials: {rôle: id}` par région (écrit par
  `cent-ans textures regions` : par rôle l'entrée du catalogue la plus étroite qui couvre la
  région). `BuildingMaterials` charge un tableau de 80 couches (`tx_building_*_array.jpg`,
  1024 / normales 512, 33 Mo) et des tables `region_table[région × 16 + couche de l'atlas]` lues
  par `building_atlas` (une région par matériau : bataille, `BuildingKit.region_style`) et
  `town_building` (paramètre d'instance `town_region` posé par `TownBuilder`). Rôle sans entrée,
  région inconnue, paquet absent ou `--legacy-textures` : couche par défaut. Micro-maçonnerie
  fondue sous 12 m. Hors périmètre : maquettes lointaines (`town_far`, `maquette_kit`) et
  bâtiments hors les murs partagés entre villes, qui gardent l'atlas par défaut.
- **Cartes au sol** : `GroundCards` (tableau de 70 cartes, 5 par biome, repli parent) ; `GroundClutter`
  remplace 45 % des touffes d'herbe par une carte du biome (`INSTANCE_CUSTOM.x` = sorte + 2 × (couche + 1)),
  taille réelle = `tile_m`.
- **Herbe de bataille** : `BattleGrassGroups` (vert, sec, steppique, alpin, arctique selon le biome de la
  bataille : `--battle-biome`, `fx/battle_province_biomes.json`). Cartes du groupe pour l'herbe de base et
  DA6. L'atlas FA7 (réglé à l'œil) reste intact pour le groupe vert ; hors vert ses cases d'herbe haute
  et folle reçoivent les cartes du groupe, chaume et blé restent.
- **Écorces et feuilles** : champ `textures: {bark, leaves}` des essences de `tree_species.yaml`
  (11 essences) ; `TreeTextures` + `battle_tree_bark` / `battle_tree_foliage` (tableaux de couches,
  micro-écorce fondue sous 14 m). Seules les essences procédurales de bataille qui correspondent
  (chêne, hêtre, peuplier, saule) changent ; les modèles générés (DN) et les imposteurs gardent leurs textures.
- **Eau** : `data/fx/water_detail.json` : `tx` par matière et `basins` (emplacement de `sea_basins.json` →
  matière). `water.gdshader` lit deux tableaux 1024 px (normales, albédos) et choisit la couche du bassin
  dominant ; océan et rivière claire prennent leurs surfaces 2k. Fleuves limoneux (lisses), lacs, marais
  et oasis : eau procédurale d'avant.

## Conséquences
- ~100 Mo de textures ajoutées (bâtiments 33, cartes 15, écorces 9, feuilles 2, eau ~25) ; le niveau 2k
  « haute » reste local (`hi/`, `tx_*_2048.json`, gitignoré).
- La résolution du biome de bataille est dupliquée dans `BattleGrassGroups.biome_for` : à remplacer par
  `BattleGroundTextures.biome_for` après fusion des sols de bataille.
- Fusion : `BuildingKit.region_style` porte désormais la clé `region` (copie du style).

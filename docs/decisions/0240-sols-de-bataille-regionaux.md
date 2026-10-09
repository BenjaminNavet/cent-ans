# 0240 — Sols de bataille régionaux

Statut : accepté (10-09, chantier TX, lot T2c)

## Contexte
Le sol de bataille était un jeu fixe de 13 couches Poly Haven, teinté à la main pour la steppe et
le désert (`terrain_tints`, ADR 0116). La fabrique TX (ADR 0236) a produit 100 images 2048² de
sols de bataille pour 13 rôles × 14 biomes (ADR 0242). Seul le biome du lieu doit être en mémoire.

## Décision
- Un paquet par biome, `battle_b01` à `battle_b14` (bloc `packs:` de
  `data/art/textures/ground_battle.yaml`, clés `biome` et `borrow`) : 13 couches dans l'ordre fixe
  du shader (un rôle chacune), albédo 1024 JPEG q80, normales 256 q85 ; 4 à 6 Mo chacun, 77 Mo en
  tout dans le dépôt (les matières partagées entre biomes sont dupliquées : le chargement d'un seul
  tableau reste simple). Variante 2048 locale sous `hi/` (gitignorée, `--size 2048`, ~33 Mo
  chacune), choisie quand « Qualité des textures » vaut « haute » et qu'elle est importée.
- `rock_sandstone_continental` (écartée, dalles maçonnées) : les biomes 2 et 12 empruntent le
  granite du biome 1 (`borrow: {rock: 1}`).
- `data/fx/battle_ground_layers.json` : `roles` (ordre), `materials` (rôle → biome → matière, écrit
  par `cent-ans textures battle-data`), `micro`, `terrain_biomes`, `default_biome`, et `legacy`
  (jeu Poly Haven + `terrain_tints`, conservé pour `--legacy-textures` et comme repli). Les tailles
  de répétition viennent du `tile_m` du manifeste du paquet.
- Biome d'une bataille : `--battle-biome=N`, sinon `data/fx/battle_province_biomes.json` (biome au
  point de départ de la province, écrit par `battle-data`), sinon `terrain_biomes[terrain]`.
  Repli : biome, parent (`BiomeParents`), `default_biome`, puis Poly Haven. Jamais d'échec dur.
- Grain fin : paquet `micro_battle` (8 grains `micro_ground`, 1024) fondu sous ~32 m par le shader
  à la place du détail proche Poly Haven ; une couche de grain par couche de sol (données), grains
  secs pour les biomes 7, 8, 14. Paquet propre au sol de bataille (celui de la campagne,
  `micro_ground`, vit dans une autre branche : à fusionner en un seul après coup).
- Shader : `tx_ground` atténue les teintes calées sur Poly Haven (saison/taches de prairie à 60 %,
  teinte de terre supprimée, boue ramenée de 0,7 à 0,2) ; la teinte par terrain disparaît.

## Conséquences
- `BattleTerrain.terrain_tint` devient une méthode d'instance (blanche avec les sols TX).
- `battle_vegetation.gd` lit `terrain.albedo_array` (le tableau du biome) pour la couleur de l'herbe.
- Rendu comparé à Poly Haven sur plaine et steppe : le sol généré est plus lisible et régional, le
  jeu Poly Haven est sombre et boueux ; TX est la valeur par défaut.
- Points ouverts : grain fin non jugé de près (la caméra de bataille ne descend pas assez), boréal
  vu en blé doré au printemps, 77 Mo de dépôt (dédoublonnage possible par un tableau par groupe de
  biomes si le budget devient critique).

# 0014 — Figurines de bataille skinnées, animations cuites en texture d'os

Date : 2026-09-25 (lot V2, audit A1-18 ; complète l'ADR 0006)

## Contexte
Les figurines B1/B4 (ADR 0006) sont des mannequins à membres rigides tournés par le shader : pas
de déformation continue, pas de visage ni de matière, chevaux en tubes. L'audit visuel (A1-18)
en fait le chantier décisif pour approcher un Total War. Contrainte : garder le rendu par
`MultiMesh` (un par régiment et par niveau de détail) pour des milliers de soldats, sans
squelette Godot par soldat.

## Décision
- **Sources** : personnages (62 os, 24 actions) et cheval (50 os, 13 actions) Quaternius CC0
  déjà importés (lot D0). Pipeline Blender reproductible `tools/blender_scripts/battle_skinned*.py`
  (`blender -b --python`) : pièces modulaires recolorées par **codes matière**, équipement du
  XIVe siècle modelé par script, décimation par pièce en trois niveaux (≈ 2 100-2 400 /
  500 / 230 triangles à pied ; 2 500-3 050 / 680-800 / 310-430 montés).
- **Texture d'os plutôt que VAT (textures de sommets)** : une ligne de texture par image, trois
  texels RGBA32F par os (matrice 3×4 de skinning). Une seule texture par rig sert toutes les
  figurines, tous leurs niveaux de détail et toutes leurs variantes (une VAT serait propre à
  chaque maillage et à chaque LOD, et bien plus lourde : sommets × images). Coût : 4 os × 3
  lectures par sommet (×2 pour l'interpolation entre images, ×2 pendant un fondu d'état).
  Rigs : `human` (22 os + 3 os virtuels, 23 clips, 871 images) et `cavalry` (45 os du cheval +
  22 du cavalier préfixés `R:` + 3 virtuels, 11 clips) — 630 Ko et 950 Ko compressés.
- **Os virtuels** : matrices calculées à la cuisson, sans os Blender : `Prop` (arme tenue à deux
  mains : pique, arbalète, lance ; sinon suit le poignet droit), `Nock` (milieu de la corde de
  l'arc, tiré jusqu'à la joue) et `Arrow` (flèche encochée, écrasée à zéro hors du tir).
- **Clips manquants** calculés par surcouche de poses (IK à deux os, visée de membres,
  rotations autour d'un pivot) sur une action Quaternius : arc long (encocher, bander, décocher),
  arbalète (viser, décocher, pied-de-biche, réarmer), piques (debout, abaissées, estoc), morts
  (effondrement à genoux, projeté en arrière, miroirs), « renversé puis se relève »,
  cavaliers (lance droite, levée, couchée, estoc, arc monté, chute du cavalier, cheval abattu).
- **Format** : binaires maison zlib (`CAM1` maillages, `CAB1` textures d'os) décrits dans
  `game/assets/models/battle_skinned/SOURCE.md`, lus par `BattleSkinned` (`battle_skinned.gd`) :
  pas d'import glTF de peau (index d'os réordonnés, poses de liaison décalées).
- **Shader** `battle_soldier_skinned.gdshader` : CUSTOM0 = indices d'os, CUSTOM1 = poids,
  COLOR = couleur linéaire + code matière (alpha × 16), UV = blason, UV2.x = masque de variante.
  Par régiment : jeu de 1 à 4 clips et mode (boucle, passes de mêlée tirées au sort, volée
  synchronisée sur la simulation, INSTANCE_CUSTOM) ; par soldat : phase, vitesse, clip, variante
  (pièces masquées = triangles dégénérés), taille, teintes (hachage de INSTANCE_ID, stable tant
  que le rang du soldat l'est). Matières procédurales (mailles en quinconce, tissage, gambison
  piqué, acier brossé, cuir, bois), robes et crins du cheval, teint et cheveux variés ; la livrée
  s'éclaire au loin (A1-01).
- **Correspondance états → clips** dans `BattleSkinned.STYLES` (rendu seulement, aucune règle).
- **Repli** : `--rigid-figures` (ou `--legacy-figures`) après `--` revient aux figurines B1/B4,
  pour les comparaisons A/B ; engins de siège inchangés.

## Points d'extension (lot suivant : sang, démembrements, chutes, collisions, effectifs ×2,5)
- **Morts multiples** : `death`, `death_m`, `death_knees`, `death_back` (projeté) à pied ;
  `c_death`, `c_death_m` (cheval abattu, cavalier au sol), `c_fall` (cavalier désarçonné) ;
  `knockdown` (renversé, au sol, se relève). Ajouter un clip = une ligne dans
  `human_clip_specs()` / `battle_skinned_cavalry.clip_specs()` et, si besoin, une fonction de
  pose.
- **Canal libre par soldat** : `INSTANCE_CUSTOM` = (instant, clip, sens, **sang 0-1**) en mode
  CUSTOM ; uniforme `blood` par régiment. Le masque de taches progressives est déjà dans le
  fragment (inactif à 0).
- **Cadavre figé** : mode CUSTOM avec instant = -1000 → dernière image du clip de mort ; un
  `MultiMesh` statique de cadavres n'a plus besoin d'être mis à jour.
- **Effectifs ×2,5** : LOD1 (≈ 500 triangles) au-delà de 32 m, LOD2 (≈ 230) au-delà de 75 m et
  pour les ombres ; piste d'imposteurs : cuire le LOD1 en atlas (8 angles × quelques images de
  marche) avec le même pipeline Blender, puis billboards par instance au-delà de ~200 m.

## Conséquences
- Toute retouche de figurine passe par les scripts Blender (≈ 1 min 30 pour tout régénérer),
  puis rien à importer côté Godot (binaires lus à l’exécution, 29 fichiers, ≈ 2,3 Mo).
- Les « figures » B1 (`assets/models/battle/*.glb`) restent pour le repli et les engins.
- Performance : voir `docs/wip/v2-soldats-animes.md` (banc `--units=20`, 4 550 soldats).

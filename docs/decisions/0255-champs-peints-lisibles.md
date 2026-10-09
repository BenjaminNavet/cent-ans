# 0255 — Champs peints lisibles à distance de jeu

## Contexte
Les modèles 3D de parcelles (`FieldLayer`, ADR 0222) sont coupés (`dn_fields.json` `"enabled": false`) : les champs ne sont plus que peints dans le sol (`hb_ground.gdshaderinc`, parcellaire de Voronoi ADR 0213, matières TX ADR 0243). De rig 15 à 60 le sol était un olive presque uniforme : (1) les matières TX, moyennées par le mipmapping, n'ont presque plus de contraste entre cultures ; (2) le fondu vers la carte de couleur commençait à 0,14 d'empreinte (px carte / px écran) ; (3) les haies, plus fines qu'un pixel, étaient pâles.

## Décision
- Bloc `view` dans `data/art/ground_biome_mix.json` (schéma à jour), appliqué par `HbGround.apply_view` aux uniforms `hb_*` : `fade_start`, `fade_end`, `hue_keep`, `cell_jitter`, `cell_hue`, `patch_gain`, `patch_start`, `patch_end`, `far_hedge`. Clé absente = défaut du shader.
- Nouveau dans le shader : à partir de `patch_start` d'empreinte, la parcelle cultivée prend (jusqu'à `patch_gain`) la couleur de sa culture (blé, vert, labour, jachère ; `season_field`, donc saisonnière), nuancée par parcelle. Le patchwork ne dépend plus du contraste des matières moyennées.
- Fondu repoussé (0,14/0,38 -> valeurs du JSON), haies minimales à distance (`far_hedge`).

## Conséquences
Réglages en données, sans recompilation. Les parcelles de vigne/olivier à distance prennent aussi une couleur de classe de culture (blé/vert/labour/jachère) : compromis accepté, la classe est tirée par parcelle, pas par matière. Les forêts, villes, rivières ne sont pas touchées (le patchwork ne vaut que sur les parcelles cultivées).
